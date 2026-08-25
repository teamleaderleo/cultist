use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fs;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
#[cfg(test)]
use std::process::Command;
use std::process::Stdio;
use std::str::FromStr;

use proc_macro2::{Delimiter, Group, TokenStream, TokenTree};

#[path = "generator_ownership.rs"]
mod generator_ownership;

use crate::finding::{AnalysisReport, Claim, ClaimKind, Evidence, Finding, Location};
use crate::git_stream::{
    self, MAX_LOG_LINE_BYTES, MAX_PATH_LINE_BYTES, UnquotedPath, drain_stderr,
    read_text_line_bounded, terminate_child,
};
use crate::performance;
use generator_ownership::{
    GeneratorRelation, discover_generator_relations, generated_attribute_paths,
};

const MAX_HISTORY_COMMITS: usize = 100;
const MAX_PATHS_PER_COMMIT: usize = 100;
const MIN_SYNTAX_COHORT: usize = 3;
const GENERATED_HEADER_BYTES: usize = 8 * 1024;
const GENERATED_HEADER_LINES: usize = 40;
const EXAMPLE_LIMIT: usize = 3;

#[derive(Debug, Clone, Eq, PartialEq)]
struct GeneratedMarker {
    line: usize,
    marker: String,
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct CohortExample {
    sha: String,
    subject: String,
}

#[derive(Debug, Clone, Default, Eq, PartialEq)]
struct SyntaxCohort {
    support: usize,
    opportunities: usize,
    comments_or_docs_only: usize,
    unclassified: usize,
    examples: Vec<CohortExample>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum SourceChangeClass {
    SyntaxChanged,
    CommentsOrDocsOnly,
    Unclassified,
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct SourceHistoryRecord {
    sha: String,
    parent: Option<String>,
    subject: String,
    paths: BTreeSet<PathBuf>,
    changed_paths: usize,
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct ClassifiedSourceCommit {
    sha: String,
    subject: String,
    paths: BTreeSet<PathBuf>,
    class: SourceChangeClass,
}

pub fn add_generated_companion_findings(
    root: &Path,
    base: Option<&str>,
    analysis: &mut AnalysisReport,
) -> Result<(), Box<dyn Error>> {
    let relations = discover_generator_relations(root)?;
    if relations.is_empty() {
        return Ok(());
    }
    let generated_attrs = generated_attribute_paths(root);

    // The analyzer only ever asks two questions of the changed set: which
    // changed paths are Rust files, and whether each admitted relation's exact
    // generated output changed. Restricting the Git pathspec to those classes
    // (`*.rs` plus literal output paths) keeps the query exact while letting
    // Git skip every other file class in large trees.
    let watched_outputs: BTreeSet<String> = relations
        .iter()
        .map(|relation| relation.output.clone())
        .collect();
    let anchor = diff_anchor(root, base)?;
    let changed = changed_paths(root, &anchor, &watched_outputs)?;
    if changed.is_empty() {
        return Ok(());
    }

    let changed_rust: BTreeSet<_> = changed
        .iter()
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("rs"))
        .cloned()
        .collect();
    if changed_rust.is_empty() {
        return Ok(());
    }

    let mut history_by_input = BTreeMap::<PathBuf, Vec<ClassifiedSourceCommit>>::new();

    for relation in relations {
        let input = PathBuf::from(&relation.input);
        let output = PathBuf::from(&relation.output);
        if !changed_rust.contains(&input) || changed.contains(&output) {
            continue;
        }
        if !root.join(&output).is_file() || !generated_attrs.contains(&relation.output) {
            continue;
        }
        let Some(marker) = generated_marker(root, &output) else {
            continue;
        };
        if !source_syntax_changed(root, &anchor, &input)? {
            continue;
        }

        if !history_by_input.contains_key(&input) {
            let history = classify_source_history(root, &input, MAX_HISTORY_COMMITS)?;
            history_by_input.insert(input.clone(), history);
        }
        let history = history_by_input
            .get(&input)
            .expect("source history inserted above");
        let cohort = build_syntax_cohort(history, &output);
        if cohort.opportunities < MIN_SYNTAX_COHORT || cohort.support != cohort.opportunities {
            continue;
        }

        analysis
            .findings
            .push(build_finding(&relation, &marker, &cohort));
    }

    Ok(())
}

fn build_finding(
    relation: &GeneratorRelation,
    marker: &GeneratedMarker,
    cohort: &SyntaxCohort,
) -> Finding {
    let input_location = Location::new(relation.input.clone(), None);
    let output_location = Location::new(relation.output.clone(), Some(marker.line));
    let generator_location = Location::new(relation.generator_path.clone(), None);

    let mut historical = Claim::new(
        ClaimKind::Observed,
        format!(
            "`{}` changed in {}/{} comparable Rust syntax-changing commits for `{}` ({:.1}%).",
            relation.output,
            cohort.support,
            cohort.opportunities,
            relation.input,
            percent(cohort.support, cohort.opportunities)
        ),
    );
    for example in &cohort.examples {
        historical = historical.with_evidence(Evidence::new(format!(
            "Example {}: {}",
            short_sha(&example.sha),
            example.subject
        )));
    }
    if cohort.comments_or_docs_only > 0 {
        historical = historical.with_evidence(Evidence::new(format!(
            "{} comment/doc/whitespace-only source commit(s) were excluded from the syntax cohort.",
            cohort.comments_or_docs_only
        )));
    }
    if cohort.unclassified > 0 {
        historical = historical.with_evidence(Evidence::new(format!(
            "{} source commit(s) could not be classified and were excluded from the syntax cohort.",
            cohort.unclassified
        )));
    }

    Finding::new(
        "generated-companion-missing",
        "Generated companion absent from source syntax change",
    )
    .at(input_location.clone())
    .with_claim(
        Claim::new(
            ClaimKind::Derived,
            format!(
                "The current diff changes normalized Rust syntax in `{}` and omits `{}`.",
                relation.input, relation.output
            ),
        )
        .with_evidence(Evidence::at(
            "Changed source is present in the current diff.",
            input_location,
        ))
        .with_evidence(Evidence::new(format!(
            "`{}` is absent from the current diff.",
            relation.output
        ))),
    )
    .with_claim(
        Claim::new(
            ClaimKind::Derived,
            format!(
                "Cargo alias `cargo {}` invokes generator package `{}`, and `{}` reads `{}` and writes `{}`.",
                relation.alias,
                relation.package,
                relation.function,
                relation.input,
                relation.output
            ),
        )
        .with_evidence(Evidence::at(
            "Literal repository-path read/write relation is present in this generator source.",
            generator_location,
        )),
    )
    .with_claim(
        Claim::new(
            ClaimKind::Observed,
            format!(
                "`{}` declares itself generated and `.gitattributes` marks the exact path `linguist-generated=true`.",
                relation.output
            ),
        )
        .with_evidence(Evidence::at(marker.marker.clone(), output_location)),
    )
    .with_claim(historical)
    .with_claim(Claim::new(
        ClaimKind::Unknown,
        "Repository evidence establishes generation ownership and precedent, but it does not establish whether this current absence is intentional or whether this source edit changes generated bytes.",
    ))
    .with_question(format!(
        "Was `cargo {}` intentionally deferred for this source change, or is `{}` stale?",
        relation.alias, relation.output
    ))
}

fn diff_anchor(root: &Path, base: Option<&str>) -> Result<String, Box<dyn Error>> {
    match base {
        Some(base) => {
            let output = performance::git_command()
                .arg("-C")
                .arg(root)
                .args(["merge-base", base, "HEAD"])
                .output()?;
            if !output.status.success() {
                return Err(format!(
                    "could not find merge base for `{base}` and HEAD: {}",
                    String::from_utf8_lossy(&output.stderr)
                )
                .into());
            }
            Ok(String::from_utf8(output.stdout)?.trim().to_string())
        }
        None => Ok("HEAD".to_string()),
    }
}

/// Streams `git diff --name-only` over the analyzer's exact file classes:
/// every Rust file plus the literal generated-output paths admitted by the
/// discovered relations. Paths stream into the result set as raw Git output
/// becomes facts; nothing output-sized is retained beyond the set itself.
fn changed_paths(
    root: &Path,
    anchor: &str,
    watched_outputs: &BTreeSet<String>,
) -> Result<BTreeSet<PathBuf>, Box<dyn Error>> {
    let mut command = performance::git_command();
    command
        .arg("-C")
        .arg(root)
        .args([
            "-c",
            "core.quotepath=false",
            "diff",
            "--name-only",
            "--no-renames",
        ])
        .arg(anchor)
        .arg("--")
        .arg("*.rs");
    for output in watched_outputs {
        // Literal magic disables glob interpretation so generated outputs with
        // pathspec metacharacters match exactly.
        command.arg(format!(":(literal){output}"));
    }
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = command.spawn()?;
    let stdout = child
        .stdout
        .take()
        .ok_or("git diff --name-only did not provide a stdout pipe")?;
    let stderr = child
        .stderr
        .take()
        .ok_or("git diff --name-only did not provide a stderr pipe")?;
    let stderr_reader = drain_stderr(stderr);

    let paths = match stream_changed_paths(BufReader::new(stdout)) {
        Ok(paths) => paths,
        Err(error) => {
            terminate_child(child, stderr_reader);
            return Err(format!("failed to stream git diff --name-only output: {error}").into());
        }
    };

    let status = child.wait()?;
    let stderr_text = stderr_reader.finish();
    if !status.success() {
        return Err(format!("git diff --name-only failed: {stderr_text}").into());
    }

    Ok(paths)
}

fn stream_changed_paths<R: BufRead>(mut reader: R) -> std::io::Result<BTreeSet<PathBuf>> {
    let mut paths = BTreeSet::new();
    let mut text = String::new();
    let mut scratch = Vec::new();

    loop {
        if read_text_line_bounded(&mut reader, &mut text, &mut scratch, MAX_PATH_LINE_BYTES)? == 0 {
            break;
        }
        let line = text.trim_end_matches(['\n', '\r']);
        if line.is_empty() {
            continue;
        }
        paths.insert(decode_git_path_line(line)?);
    }

    Ok(paths)
}

fn decode_git_path_line(line: &str) -> std::io::Result<PathBuf> {
    match git_stream::c_style_unquote(line) {
        Ok(UnquotedPath::Verbatim) => Ok(PathBuf::from(line)),
        Ok(UnquotedPath::Decoded(bytes)) => String::from_utf8(bytes)
            .map(PathBuf::from)
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "quoted path is not UTF-8")),
        Err(reason) => Err(io::Error::new(io::ErrorKind::InvalidData, reason)),
    }
}

fn source_syntax_changed(root: &Path, anchor: &str, path: &Path) -> Result<bool, Box<dyn Error>> {
    let current = fs::read_to_string(root.join(path))?;
    let Some(before) = source_at_revision(root, anchor, path) else {
        return Ok(false);
    };
    let Some(before) = rust_syntax_fingerprint(&before) else {
        return Ok(false);
    };
    let Some(current) = rust_syntax_fingerprint(&current) else {
        return Ok(false);
    };
    Ok(before != current)
}

fn classify_source_history(
    root: &Path,
    input: &Path,
    max_commits: usize,
) -> Result<Vec<ClassifiedSourceCommit>, Box<dyn Error>> {
    let records = read_source_history(root, input, max_commits)?;
    let considered: Vec<_> = records
        .into_iter()
        .filter(|record| {
            !is_revert_subject(&record.subject) && record.changed_paths <= MAX_PATHS_PER_COMMIT
        })
        .collect();
    let versions = read_source_versions(root, input, &considered)?;
    let mut classified = Vec::with_capacity(considered.len());

    for record in considered {
        let class = match record.parent.as_deref() {
            Some(parent) => {
                let after_key = revision_spec(&record.sha, input);
                let before_key = revision_spec(parent, input);
                let after = versions
                    .get(&after_key)
                    .and_then(|source| source.as_deref());
                let before = versions
                    .get(&before_key)
                    .and_then(|source| source.as_deref());
                match (before, after) {
                    (Some(before), Some(after)) => match (
                        rust_syntax_fingerprint(before),
                        rust_syntax_fingerprint(after),
                    ) {
                        (Some(before), Some(after)) if before == after => {
                            SourceChangeClass::CommentsOrDocsOnly
                        }
                        (Some(_), Some(_)) => SourceChangeClass::SyntaxChanged,
                        _ => SourceChangeClass::Unclassified,
                    },
                    _ => SourceChangeClass::Unclassified,
                }
            }
            None => SourceChangeClass::Unclassified,
        };

        classified.push(ClassifiedSourceCommit {
            sha: record.sha,
            subject: record.subject,
            paths: record.paths,
            class,
        });
    }

    Ok(classified)
}

fn build_syntax_cohort(history: &[ClassifiedSourceCommit], output: &Path) -> SyntaxCohort {
    let mut cohort = SyntaxCohort::default();

    for commit in history {
        match commit.class {
            SourceChangeClass::CommentsOrDocsOnly => {
                cohort.comments_or_docs_only += 1;
                continue;
            }
            SourceChangeClass::Unclassified => {
                cohort.unclassified += 1;
                continue;
            }
            SourceChangeClass::SyntaxChanged => {}
        }

        cohort.opportunities += 1;
        if commit.paths.contains(output) {
            cohort.support += 1;
            if cohort.examples.len() < EXAMPLE_LIMIT {
                cohort.examples.push(CohortExample {
                    sha: commit.sha.clone(),
                    subject: commit.subject.clone(),
                });
            }
        }
    }

    cohort
}

/// Streams the batched `git log` feed for one input path. Raw output becomes
/// records incrementally; once a commit crosses the broad-commit threshold its
/// retained path strings are dropped and only counting continues, which keeps
/// exclusion decisions exact without holding every path of a mass commit.
fn read_source_history(
    root: &Path,
    input: &Path,
    max_commits: usize,
) -> Result<Vec<SourceHistoryRecord>, Box<dyn Error>> {
    let mut child = performance::git_command()
        .arg("-C")
        .arg(root)
        .args([
            "-c",
            "core.quotepath=false",
            "log",
            "--format=%x1e%H%x1f%P%x1f%s",
            "--name-only",
            "--no-renames",
            "--no-color",
            "--no-ext-diff",
            "--root",
            "--no-merges",
            "--full-diff",
            "-n",
        ])
        .arg(max_commits.to_string())
        .arg("--")
        .arg(input)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let stdout = child
        .stdout
        .take()
        .ok_or("git log did not provide a stdout pipe")?;
    let stderr = child
        .stderr
        .take()
        .ok_or("git log did not provide a stderr pipe")?;
    let stderr_reader = drain_stderr(stderr);

    let parsed = match stream_source_history_log(BufReader::new(stdout), MAX_PATHS_PER_COMMIT) {
        Ok(parsed) => parsed,
        Err(error) => {
            terminate_child(child, stderr_reader);
            return Err(format!(
                "failed to stream git history for {}: {error}",
                input.display()
            )
            .into());
        }
    };

    let status = child.wait()?;
    let stderr_text = stderr_reader.finish();
    if !status.success() {
        return Err(format!("git log failed for {}: {stderr_text}", input.display()).into());
    }

    parsed.ok_or_else(|| format!("could not parse git history for {}", input.display()).into())
}

fn stream_source_history_log<R: BufRead>(
    mut reader: R,
    max_paths_per_commit: usize,
) -> std::io::Result<Option<Vec<SourceHistoryRecord>>> {
    let mut records = Vec::new();
    let mut current: Option<SourceHistoryAccumulator> = None;
    let mut malformed = false;
    let mut text = String::new();
    let mut scratch = Vec::new();

    loop {
        if read_text_line_bounded(&mut reader, &mut text, &mut scratch, MAX_LOG_LINE_BYTES)? == 0 {
            break;
        }
        if malformed {
            continue;
        }

        if let Some(metadata) = text.strip_prefix('\u{1e}') {
            if let Some(record) = current.take() {
                records.push(record.finish(max_paths_per_commit));
            }
            match source_history_summary(metadata) {
                Some((sha, parent, subject)) => {
                    current = Some(SourceHistoryAccumulator::new(sha, parent, subject));
                }
                None => malformed = true,
            }
        } else if let Some(accumulator) = current.as_mut() {
            let path = text.trim_end_matches(['\n', '\r']).trim();
            if !path.is_empty() {
                accumulator.offer_path(max_paths_per_commit, path);
            }
        } else if !text.trim().is_empty() {
            malformed = true;
        }
    }

    if malformed {
        return Ok(None);
    }
    if let Some(record) = current.take() {
        records.push(record.finish(max_paths_per_commit));
    }

    Ok(Some(records))
}

struct SourceHistoryAccumulator {
    sha: String,
    parent: Option<String>,
    subject: String,
    paths: BTreeSet<PathBuf>,
    counting_only: bool,
    counted_overflow_paths: usize,
}

impl SourceHistoryAccumulator {
    fn new(sha: String, parent: Option<String>, subject: String) -> Self {
        Self {
            sha,
            parent,
            subject,
            paths: BTreeSet::new(),
            counting_only: false,
            counted_overflow_paths: 0,
        }
    }

    fn offer_path(&mut self, max_paths_per_commit: usize, path: &str) {
        if self.counting_only {
            self.counted_overflow_paths += 1;
            return;
        }
        self.paths.insert(PathBuf::from(path));
        if self.paths.len() > max_paths_per_commit {
            self.counting_only = true;
            self.paths.clear();
            self.counted_overflow_paths = 0;
        }
    }

    fn finish(self, max_paths_per_commit: usize) -> SourceHistoryRecord {
        let changed_paths = if self.counting_only {
            max_paths_per_commit + 1 + self.counted_overflow_paths
        } else {
            self.paths.len()
        };
        SourceHistoryRecord {
            sha: self.sha,
            parent: self.parent,
            subject: self.subject,
            paths: self.paths,
            changed_paths,
        }
    }
}

fn source_history_summary(metadata: &str) -> Option<(String, Option<String>, String)> {
    let metadata = metadata.trim_end_matches(['\n', '\r']);
    let mut fields = metadata.splitn(3, '\u{1f}');
    let sha = fields.next()?.trim().to_string();
    let parent = fields
        .next()?
        .split_whitespace()
        .next()
        .map(ToOwned::to_owned);
    let subject = fields.next()?.trim().to_string();
    Some((sha, parent, subject))
}

fn read_source_versions(
    root: &Path,
    input: &Path,
    records: &[SourceHistoryRecord],
) -> Result<BTreeMap<String, Option<String>>, Box<dyn Error>> {
    let mut requested = BTreeSet::new();
    for record in records {
        requested.insert(revision_spec(&record.sha, input));
        if let Some(parent) = &record.parent {
            requested.insert(revision_spec(parent, input));
        }
    }

    let mut results = BTreeMap::new();
    let mut safe = Vec::new();
    for spec in requested {
        if spec.contains(['\n', '\r']) {
            results.insert(spec, None);
        } else {
            safe.push(spec);
        }
    }
    results.extend(read_git_blobs(root, &safe)?);
    Ok(results)
}

fn read_git_blobs(
    root: &Path,
    specs: &[String],
) -> Result<BTreeMap<String, Option<String>>, Box<dyn Error>> {
    if specs.is_empty() {
        return Ok(BTreeMap::new());
    }

    let mut child = performance::git_command()
        .arg("-C")
        .arg(root)
        .args(["cat-file", "--batch"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;

    {
        let mut stdin = child
            .stdin
            .take()
            .ok_or("git cat-file did not provide a stdin pipe")?;
        for spec in specs {
            stdin.write_all(spec.as_bytes())?;
            stdin.write_all(b"\n")?;
        }
    }

    let stdout = child
        .stdout
        .take()
        .ok_or("git cat-file did not provide a stdout pipe")?;
    let mut reader = BufReader::new(stdout);
    let mut results = BTreeMap::new();

    for spec in specs {
        let mut header = String::new();
        if reader.read_line(&mut header)? == 0 {
            return Err(format!("git cat-file ended before reading `{spec}`").into());
        }
        let header = header.trim_end_matches(['\n', '\r']);
        if header.ends_with(" missing") {
            results.insert(spec.clone(), None);
            continue;
        }

        let mut fields = header.split_whitespace();
        let _object = fields
            .next()
            .ok_or_else(|| format!("invalid git cat-file header for `{spec}`"))?;
        let kind = fields
            .next()
            .ok_or_else(|| format!("invalid git cat-file header for `{spec}`"))?;
        let size = fields
            .next()
            .ok_or_else(|| format!("invalid git cat-file header for `{spec}`"))?
            .parse::<usize>()?;
        if fields.next().is_some() {
            return Err(format!("invalid git cat-file header for `{spec}`").into());
        }

        let mut bytes = vec![0; size];
        reader.read_exact(&mut bytes)?;
        let mut terminator = [0_u8; 1];
        reader.read_exact(&mut terminator)?;
        if terminator != *b"\n" {
            return Err(format!("invalid git cat-file payload terminator for `{spec}`").into());
        }

        let source = if kind == "blob" {
            String::from_utf8(bytes).ok()
        } else {
            None
        };
        results.insert(spec.clone(), source);
    }

    let status = child.wait()?;
    if !status.success() {
        return Err(format!("git cat-file failed with status {status}").into());
    }

    Ok(results)
}

fn revision_spec(revision: &str, path: &Path) -> String {
    format!("{revision}:{}", normalize_path(path))
}

fn source_at_revision(root: &Path, revision: &str, path: &Path) -> Option<String> {
    let spec = revision_spec(revision, path);
    let output = performance::git_command()
        .arg("-C")
        .arg(root)
        .args(["show", &spec])
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8(output.stdout).ok())
        .flatten()
}

fn rust_syntax_fingerprint(source: &str) -> Option<String> {
    let tokens = TokenStream::from_str(source).ok()?;
    Some(strip_doc_attributes(tokens).to_string())
}

fn strip_doc_attributes(stream: TokenStream) -> TokenStream {
    let tokens: Vec<_> = stream.into_iter().collect();
    let mut output = TokenStream::new();
    let mut index = 0;

    while index < tokens.len() {
        if is_hash(&tokens[index]) {
            if index + 1 < tokens.len() && is_doc_group(&tokens[index + 1]) {
                index += 2;
                continue;
            }
            if index + 2 < tokens.len()
                && is_bang(&tokens[index + 1])
                && is_doc_group(&tokens[index + 2])
            {
                index += 3;
                continue;
            }
        }

        let token = match tokens[index].clone() {
            TokenTree::Group(group) => {
                let mut normalized =
                    Group::new(group.delimiter(), strip_doc_attributes(group.stream()));
                normalized.set_span(group.span());
                TokenTree::Group(normalized)
            }
            other => other,
        };
        output.extend([token]);
        index += 1;
    }

    output
}

fn is_hash(token: &TokenTree) -> bool {
    matches!(token, TokenTree::Punct(punct) if punct.as_char() == '#')
}

fn is_bang(token: &TokenTree) -> bool {
    matches!(token, TokenTree::Punct(punct) if punct.as_char() == '!')
}

fn is_doc_group(token: &TokenTree) -> bool {
    let TokenTree::Group(group) = token else {
        return false;
    };
    group.delimiter() == Delimiter::Bracket
        && matches!(group.stream().into_iter().next(), Some(TokenTree::Ident(ident)) if ident == "doc")
}

fn generated_marker(root: &Path, output: &Path) -> Option<GeneratedMarker> {
    let bytes = fs::read(root.join(output)).ok()?;
    let prefix = &bytes[..bytes.len().min(GENERATED_HEADER_BYTES)];
    let text = String::from_utf8_lossy(prefix);
    text.lines()
        .take(GENERATED_HEADER_LINES)
        .enumerate()
        .find_map(|(index, line)| {
            strong_generated_marker(line).then(|| GeneratedMarker {
                line: index + 1,
                marker: line.trim().to_string(),
            })
        })
}

fn strong_generated_marker(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    lower.contains("@generated")
        || lower.contains("do not edit")
        || lower.contains("automatically generated")
        || lower.contains("auto-generated")
}

fn normalize_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn is_revert_subject(subject: &str) -> bool {
    subject
        .trim_start()
        .to_ascii_lowercase()
        .starts_with("revert")
}

fn percent(support: usize, opportunities: usize) -> f64 {
    if opportunities == 0 {
        0.0
    } else {
        support as f64 * 100.0 / opportunities as f64
    }
}

fn short_sha(sha: &str) -> &str {
    sha.get(..sha.len().min(8)).unwrap_or(sha)
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;
    use crate::diff::build_diff_analysis_report;

    fn unique_temp_dir(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "cargo-cultist-generated-history-{name}-{}-{nanos}",
            std::process::id()
        ))
    }

    fn run_git(root: &Path, args: &[&str]) {
        let output = Command::new("git")
            .arg("-C")
            .arg(root)
            .args(args)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "git command failed: git {args:?}\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    #[test]
    fn comments_and_docs_do_not_change_syntax_fingerprint() {
        let before = "// old\n/// old docs\nfn answer() -> usize { 42 }";
        let after = "// new\n/// new docs\nfn answer() -> usize { 42 }";
        assert_eq!(
            rust_syntax_fingerprint(before),
            rust_syntax_fingerprint(after)
        );
    }

    #[test]
    fn code_change_changes_syntax_fingerprint() {
        assert_ne!(
            rust_syntax_fingerprint("fn answer() -> usize { 41 }"),
            rust_syntax_fingerprint("fn answer() -> usize { 42 }")
        );
    }

    #[test]
    fn generated_marker_requires_strong_phrase() {
        assert!(strong_generated_marker(
            "// Auto-generated code, DO NOT EDIT DIRECTLY!"
        ));
        assert!(strong_generated_marker("// @generated"));
        assert!(!strong_generated_marker(
            "// generated by parser at runtime"
        ));
    }

    #[test]
    fn semantic_cohort_requires_all_comparable_commits() {
        let cohort = SyntaxCohort {
            support: 9,
            opportunities: 10,
            ..SyntaxCohort::default()
        };
        assert_ne!(cohort.support, cohort.opportunities);
    }

    #[test]
    fn parses_batched_source_history_records() {
        let output = concat!(
            "\x1eabc\x1fparent\x1ffeat: one\n\n",
            "src/input.rs\n",
            "generated/output.rs\n",
            "\x1edef\x1fabc\x1fdocs: two\n\n",
            "src/input.rs\n",
        );
        let records = stream_source_history_log(Cursor::new(output), MAX_PATHS_PER_COMMIT)
            .unwrap()
            .unwrap();
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].sha, "abc");
        assert_eq!(records[0].parent.as_deref(), Some("parent"));
        assert!(records[0].paths.contains(Path::new("generated/output.rs")));
        assert_eq!(records[0].changed_paths, 2);
        assert_eq!(records[1].subject, "docs: two");
        assert_eq!(records[1].changed_paths, 1);
    }

    #[test]
    fn broad_source_commits_count_without_retaining_paths() {
        let mut output = String::from("\x1ebroad\x1fparent\x1fmass change\n\n");
        for index in 0..9 {
            output.push_str(&format!("src/generated_{index}.rs\n"));
        }
        output.push_str("\x1enarrow\x1fbroad\x1fsmall change\n\nsrc/input.rs\n");

        let records = stream_source_history_log(Cursor::new(output.as_bytes()), 4)
            .unwrap()
            .unwrap();
        assert_eq!(records.len(), 2);
        assert!(records[0].paths.is_empty());
        assert_eq!(records[0].changed_paths, 9);
        assert_eq!(records[1].changed_paths, 1);
        assert_eq!(records[1].paths.len(), 1);
    }

    #[test]
    fn duplicate_overflow_lines_are_counted_not_retained() {
        // Crossing the threshold switches the commit to counting mode; every
        // later path line is counted even when it duplicates an earlier one.
        let output = "\x1ea\x1fp\x1fs\nb.rs\na.rs\na.rs\nc.rs\nd.rs\n";
        let records = stream_source_history_log(Cursor::new(output), 2)
            .unwrap()
            .unwrap();
        assert!(records[0].paths.is_empty());
        // Two retained distinct paths + the crossing line + one counted line.
        assert_eq!(records[0].changed_paths, 4);
    }

    #[test]
    fn malformed_source_history_fails_without_panicking() {
        for bad in [
            "garbage before any record\n",
            "\x1esha-only\x1fparent-only\n",
            "\x1esha\x1f",
            "\x1ea\x1fp\x1fs\np\n\x1eb\x1fp",
        ] {
            let parsed = stream_source_history_log(Cursor::new(bad), MAX_PATHS_PER_COMMIT).unwrap();
            assert!(parsed.is_none(), "{bad:?} must fail closed");
        }
    }

    #[test]
    fn oversized_source_history_lines_fail_closed() {
        let long_path = format!("{}\n", "p".repeat(MAX_LOG_LINE_BYTES + 1));
        let error = stream_source_history_log(
            Cursor::new(format!("\x1ea\x1fp\x1fs\n{long_path}").into_bytes()),
            MAX_PATHS_PER_COMMIT,
        )
        .unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn batched_history_preserves_syntax_cohort_semantics() {
        let root = unique_temp_dir("cohort");
        fs::create_dir_all(root.join("src")).unwrap();
        fs::create_dir_all(root.join("generated")).unwrap();
        run_git(&root, &["init", "-q", "-b", "main"]);
        run_git(&root, &["config", "user.email", "cultist@example.invalid"]);
        run_git(&root, &["config", "user.name", "Cargo Cultist Tests"]);

        fs::write(root.join("src/input.rs"), "fn value() -> usize { 0 }\n").unwrap();
        fs::write(
            root.join("generated/output.rs"),
            "const VALUE: usize = 0;\n",
        )
        .unwrap();
        run_git(&root, &["add", "."]);
        run_git(&root, &["commit", "-q", "-m", "baseline"]);

        fs::write(
            root.join("src/input.rs"),
            "// comment only\nfn value() -> usize { 0 }\n",
        )
        .unwrap();
        run_git(&root, &["add", "src/input.rs"]);
        run_git(&root, &["commit", "-q", "-m", "docs only"]);

        fs::write(root.join("src/input.rs"), "fn value() -> usize { 1 }\n").unwrap();
        fs::write(
            root.join("generated/output.rs"),
            "const VALUE: usize = 1;\n",
        )
        .unwrap();
        run_git(&root, &["add", "."]);
        run_git(&root, &["commit", "-q", "-m", "syntax one"]);

        fs::write(root.join("src/input.rs"), "fn value() -> usize { 2 }\n").unwrap();
        fs::write(
            root.join("generated/output.rs"),
            "const VALUE: usize = 2;\n",
        )
        .unwrap();
        run_git(&root, &["add", "."]);
        run_git(&root, &["commit", "-q", "-m", "syntax two"]);

        let history = classify_source_history(&root, Path::new("src/input.rs"), 10).unwrap();
        let cohort = build_syntax_cohort(&history, Path::new("generated/output.rs"));
        assert_eq!(cohort.opportunities, 2);
        assert_eq!(cohort.support, 2);
        assert_eq!(cohort.comments_or_docs_only, 1);
        assert_eq!(cohort.unclassified, 1);

        fs::remove_dir_all(root).unwrap();
    }

    fn init_generator_repo(name: &str) -> PathBuf {
        let root = unique_temp_dir(name);
        fs::create_dir_all(root.join("src")).unwrap();
        fs::create_dir_all(root.join("data")).unwrap();
        fs::create_dir_all(root.join(".cargo")).unwrap();
        fs::create_dir_all(root.join("packages/gen_task/src")).unwrap();
        run_git(&root, &["init", "-q", "-b", "main"]);
        run_git(&root, &["config", "user.email", "cultist@example.invalid"]);
        run_git(&root, &["config", "user.name", "Cargo Cultist Tests"]);
        fs::write(
            root.join(".cargo/config.toml"),
            "[alias]\ngen = \"run -p gen_task\"\n",
        )
        .unwrap();
        fs::write(
            root.join("packages/gen_task/Cargo.toml"),
            "[package]\nname = \"gen_task\"\nversion = \"0.0.0\"\n",
        )
        .unwrap();
        fs::write(
            root.join("packages/gen_task/src/main.rs"),
            concat!(
                "fn generate() -> std::io::Result<()> {\n",
                "    let root = project_root::get_project_root()\n",
                "        .map_err(|error| std::io::Error::other(error.to_string()))?;\n",
                "    let source = std::fs::read_to_string(root.join(\"src/input.rs\"))?;\n",
                "    let target = root.join(\"data/out.json\");\n",
                "    std::fs::write(&target, source)?;\n",
                "    Ok(())\n",
                "}\n",
            ),
        )
        .unwrap();
        fs::write(
            root.join(".gitattributes"),
            "data/out.json linguist-generated=true\n",
        )
        .unwrap();
        root
    }

    fn write_pair(root: &Path, value: usize, include_output: bool) {
        if value == 0 {
            fs::write(root.join("src/input.rs"), "fn value() -> usize { 0 }\n").unwrap();
            write_output(root, 0);
        } else {
            fs::write(
                root.join("src/input.rs"),
                format!("// v{value}\nfn value() -> usize {{ {value} }}\n"),
            )
            .unwrap();
            if include_output {
                write_output(root, value);
            }
        }
    }

    fn write_output(root: &Path, value: usize) {
        fs::write(
            root.join("data/out.json"),
            // Non-.rs generated output: only the literal-output pathspec keeps
            // its diff membership exact.
            format!("@generated do not edit\n{{\"value\": {value}}}\n"),
        )
        .unwrap();
    }

    #[test]
    fn restricted_pathspec_keeps_non_rust_generated_outputs_exact_end_to_end() {
        let root = init_generator_repo("pathspec-e2e");
        write_pair(&root, 0, true);
        run_git(&root, &["add", "."]);
        run_git(&root, &["commit", "-q", "-m", "baseline"]);

        for value in 1..=4 {
            write_pair(&root, value, true);
            run_git(&root, &["add", "."]);
            run_git(&root, &["commit", "-q", "-m", &format!("regen {value}")]);
        }

        // Working tree: input syntax changes together with its non-Rust
        // generated companion. Exact output membership means no finding.
        write_pair(&root, 5, true);
        let analysis = build_diff_analysis_report(&root, None).unwrap();
        assert!(
            analysis
                .findings
                .iter()
                .all(|finding| finding.kind != "generated-companion-missing"),
            "companion changed with the input; expected no missing-companion finding"
        );

        // Control: restore the companion, regenerate the input alone, and the
        // finding must appear.
        write_output(&root, 4);
        write_pair(&root, 6, false);
        let analysis = build_diff_analysis_report(&root, None).unwrap();
        assert!(
            analysis
                .findings
                .iter()
                .any(|finding| finding.kind == "generated-companion-missing"),
            "stale non-.rs companion must be reported; claims: {:?}",
            analysis
                .claims
                .iter()
                .map(|claim| claim.message.clone())
                .collect::<Vec<_>>()
        );

        fs::remove_dir_all(root).unwrap();
    }
}
