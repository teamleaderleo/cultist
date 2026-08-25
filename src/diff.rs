use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::io::{self, BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::finding::{AnalysisReport, Claim, ClaimKind, Evidence, Finding, Location};
use crate::generated_diff::add_generated_companion_findings;
use crate::git_stream::{
    self, MAX_DIFF_LINE_BYTES, UnquotedPath, drain_stderr, read_text_line_bounded, terminate_child,
};
use crate::performance;
use crate::rust_facts::FactCache;
use crate::test_module_aggregate::{ScopeCache, TestModulePrecedent, overlay_precedent_with};
use crate::test_modules::{
    TestModuleOccurrence, analyze_test_module_files, analyze_test_modules_excluding,
};

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
struct LineRange {
    start: usize,
    end: usize,
}

#[derive(Debug, Default, Eq, PartialEq)]
struct ChangedLines {
    by_path: BTreeMap<PathBuf, Vec<LineRange>>,
}

impl ChangedLines {
    /// Records one added new-file line number. Git streams hunks in ascending
    /// new-file order per path, so lines arrive monotonically; adjacent and
    /// contiguous numbers merge into a single inclusive range, keeping storage
    /// proportional to hunks rather than added lines.
    fn insert(&mut self, path: &Path, line: usize) {
        let ranges = self.by_path.entry(path.to_path_buf()).or_default();
        if let Some(last) = ranges.last_mut() {
            // Descending lines would create overlapping ranges and break the
            // logarithmic interval search; saturated repeats are allowed.
            debug_assert!(last.end <= line, "added line numbers must ascend per path");
            if line <= last.end.saturating_add(1) {
                last.end = last.end.max(line);
                return;
            }
        }
        ranges.push(LineRange {
            start: line,
            end: line,
        });
    }

    /// Membership is an interval search over sorted, disjoint ranges:
    /// logarithmic in hunk count instead of linear over individual lines.
    fn contains(&self, path: &Path, line: usize) -> bool {
        self.by_path.get(path).is_some_and(|ranges| {
            let index = ranges.partition_point(|range| range.end < line);
            ranges.get(index).is_some_and(|range| range.start <= line)
        })
    }

    fn rust_paths(&self) -> impl Iterator<Item = &Path> {
        self.by_path
            .keys()
            .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("rs"))
            .map(PathBuf::as_path)
    }

    fn rust_file_count(&self) -> usize {
        self.rust_paths().count()
    }

    #[cfg(test)]
    fn range_count(&self, path: &Path) -> usize {
        self.by_path.get(path).map_or(0, Vec::len)
    }

    #[cfg(test)]
    fn total_range_entries(&self) -> usize {
        self.by_path.values().map(Vec::len).sum()
    }
}

pub fn git_repo_root(path: &Path) -> Result<PathBuf, Box<dyn Error>> {
    let output = performance::git_command()
        .arg("-C")
        .arg(path)
        .args(["rev-parse", "--show-toplevel"])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("{path:?} is not inside a Git repository: {stderr}").into());
    }

    let stdout = String::from_utf8(output.stdout)?;
    Ok(PathBuf::from(stdout.trim()).canonicalize()?)
}

pub fn build_diff_analysis_report(
    root: &Path,
    base: Option<&str>,
) -> Result<AnalysisReport, Box<dyn Error>> {
    build_diff_analysis_report_with_caches(
        root,
        base,
        FactCache::from_environment(),
        ScopeCache::from_environment(),
    )
}

pub(crate) fn build_diff_analysis_report_with_caches(
    root: &Path,
    base: Option<&str>,
    fact_cache: Option<FactCache>,
    scope_cache: Option<ScopeCache>,
) -> Result<AnalysisReport, Box<dyn Error>> {
    let changed = git_diff_changed_lines(root, base)?;

    let mut analysis = AnalysisReport::new("diff-precedent", root.to_string_lossy().into_owned());
    analysis.claims.push(Claim::new(
        ClaimKind::Derived,
        match base {
            Some(base) => format!(
                "The diff contains added lines in {} Rust file(s), measured from the merge base with `{base}`.",
                changed.rust_file_count()
            ),
            None => format!(
                "The working-tree diff contains added lines in {} Rust file(s) relative to HEAD.",
                changed.rust_file_count()
            ),
        },
    ));

    add_generated_companion_findings(root, base, &mut analysis)?;

    let changed_rust_paths: Vec<_> = changed.rust_paths().map(|path| root.join(path)).collect();
    if changed_rust_paths.is_empty() {
        analysis.claims.push(Claim::new(
            ClaimKind::Observed,
            "No added or renamed test-gated module declarations were found in the diff.",
        ));
        return Ok(analysis);
    }

    // Parse only changed Rust files first. Most diffs can stop here without
    // walking or parsing the rest of the repository.
    let changed_report = analyze_test_module_files(&changed_rust_paths)?;
    if !changed_report.parse_failures.is_empty() {
        for (path, error) in &changed_report.parse_failures {
            analysis.claims.push(
                Claim::new(
                    ClaimKind::Unknown,
                    "A changed Rust file could not be parsed, so diff relevance could not be determined.",
                )
                .with_evidence(Evidence::at(
                    error.clone(),
                    Location::new(
                        relative_path(root, path).to_string_lossy().into_owned(),
                        None,
                    ),
                )),
            );
        }
        return Ok(analysis);
    }

    let relevant_changed_modules = changed_report
        .occurrences
        .iter()
        .filter(|occurrence| {
            occurrence
                .path
                .strip_prefix(root)
                .is_ok_and(|path| changed.contains(path, occurrence.line))
        })
        .count();
    if relevant_changed_modules == 0 {
        analysis.claims.push(Claim::new(
            ClaimKind::Observed,
            "No added or renamed test-gated module declarations were found in the diff.",
        ));
        return Ok(analysis);
    }

    // A relevant declaration needs repository precedent. Prefer combining
    // the changed-file facts we already parsed with cached clean baseline
    // aggregates, so a small diff never walks every remaining fact row. Any
    // working state outside the overlay's exactness envelope — or any cache
    // failure — falls back to the deterministic full scan below without
    // altering findings.
    let changed_paths: BTreeSet<_> = changed_rust_paths.iter().cloned().collect();
    match overlay_precedent_with(
        fact_cache.as_ref(),
        scope_cache.as_ref(),
        root,
        &changed_paths,
        &changed_report.occurrences,
    ) {
        Ok(Some(precedent)) => {
            add_diff_findings(
                root,
                &changed_report.occurrences,
                &changed,
                &precedent,
                &mut analysis,
            );
            return Ok(analysis);
        }
        // Dirty/staged/untracked/deleted states outside the changed set.
        Ok(None) => {}
        // Baseline aggregation is advisory; recompute from live facts.
        Err(_) => {}
    }

    let excluded_paths = changed_paths;
    let mut report = analyze_test_modules_excluding(root, &excluded_paths)?;
    report.extend(changed_report);

    let precedent = TestModulePrecedent::from_occurrences(&report.occurrences);
    add_diff_findings(
        root,
        &report.occurrences,
        &changed,
        &precedent,
        &mut analysis,
    );
    Ok(analysis)
}

fn add_diff_findings(
    root: &Path,
    candidate_occurrences: &[TestModuleOccurrence],
    changed: &ChangedLines,
    precedent: &TestModulePrecedent,
    analysis: &mut AnalysisReport,
) {
    let changed_modules: Vec<&TestModuleOccurrence> = candidate_occurrences
        .iter()
        .filter(|occurrence| {
            occurrence
                .path
                .strip_prefix(root)
                .is_ok_and(|path| changed.contains(path, occurrence.line))
        })
        .collect();

    for occurrence in changed_modules {
        let local_names: BTreeSet<String> = candidate_occurrences
            .iter()
            .filter(|other| {
                other.path == occurrence.path
                    && (other.line != occurrence.line || other.name != occurrence.name)
            })
            .map(|other| other.name.clone())
            .collect();
        let different_local_names: Vec<_> = local_names
            .iter()
            .filter(|name| name.as_str() != occurrence.name)
            .cloned()
            .collect();
        let (precedent_counts, total_without_target) = precedent.excluding(&occurrence.name);
        let repository_count = precedent_counts
            .get(&occurrence.name)
            .copied()
            .unwrap_or_default();
        let precedent_total = total_without_target;
        let one_off = repository_count == 0 && precedent_total > 0;
        let tension = precedent_tension(&precedent_counts, &local_names);

        if different_local_names.is_empty() && !one_off {
            continue;
        }

        let occurrence_location = Location::new(
            relative_path(root, &occurrence.path)
                .to_string_lossy()
                .into_owned(),
            Some(occurrence.line),
        );
        let mut finding = Finding::new("test-module-precedent", "Test-module precedent")
            .at(occurrence_location.clone())
            .with_claim(
                Claim::new(
                    ClaimKind::Observed,
                    format!(
                        "`{}` appears {repository_count} time(s) across {precedent_total} existing test-gated modules, excluding the changed declaration.",
                        occurrence.name
                    ),
                )
                .with_evidence(Evidence::at(
                    format!(
                        "This change adds `mod {}` behind a test cfg.",
                        occurrence.name
                    ),
                    occurrence_location,
                ))
                .with_evidence(Evidence::new(format!(
                    "Repository precedent counts: {}.",
                    top_counts_summary(&precedent_counts)
                ))),
            );

        if !different_local_names.is_empty() {
            finding = finding.with_claim(Claim::new(
                ClaimKind::Observed,
                format!(
                    "The same file already uses: {}.",
                    different_local_names
                        .iter()
                        .map(|name| format!("`{name}`"))
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            ));
        }

        if let Some((repository_name, file_name)) = tension {
            finding.kind = "test-module-precedent-tension".to_string();
            finding.title = "Test-module precedent tension".to_string();
            finding = finding.with_claim(Claim::new(
                ClaimKind::Observed,
                format!(
                    "Repository-wide precedent favors `{repository_name}`, while existing file-local precedent favors `{file_name}`."
                ),
            ));
        }

        let observation = match tension {
            Some((repository_name, file_name)) => {
                let alignment = if occurrence.name == repository_name {
                    "The change follows repository-wide precedent and differs from file-local precedent."
                } else if occurrence.name == file_name {
                    "The change follows file-local precedent and differs from repository-wide precedent."
                } else {
                    "The change follows neither of the two conflicting precedent scopes."
                };
                format!("Repository-wide and file-local precedent disagree. {alignment}")
            }
            None if !different_local_names.is_empty() && one_off => {
                "The new name differs from this file's existing precedent and does not appear elsewhere in the repository.".to_string()
            }
            None if !different_local_names.is_empty() => {
                "The new name differs from this file's existing test-module precedent.".to_string()
            }
            None => {
                "The new name does not appear among the repository's existing test-gated modules."
                    .to_string()
            }
        };

        finding = finding
            .with_claim(Claim::new(ClaimKind::Observed, observation))
            .with_claim(Claim::new(
                ClaimKind::Unknown,
                "Repository evidence alone does not establish which naming scope should govern this change.",
            ))
            .with_question(
                "Is the distinct module name intentional, or should it follow nearby precedent?",
            );

        analysis.findings.push(finding);
    }

    if analysis.findings.is_empty() {
        analysis.claims.push(Claim::new(
            ClaimKind::Observed,
            "The changed test-module names match existing repository precedent.",
        ));
    }
}

fn git_diff_changed_lines(root: &Path, base: Option<&str>) -> Result<ChangedLines, Box<dyn Error>> {
    let anchor = match base {
        Some(base) => merge_base(root, base)?,
        None => "HEAD".to_string(),
    };

    let mut command = performance::git_command();
    command
        .arg("-C")
        .arg(root)
        .args([
            "-c",
            "core.quotepath=false",
            "diff",
            "--unified=0",
            "--no-ext-diff",
            "--no-color",
            "--no-prefix",
        ])
        .arg(anchor)
        .arg("--")
        .arg("*.rs");
    run_git_diff_streaming(command)
}

fn run_git_diff_streaming(mut command: Command) -> Result<ChangedLines, Box<dyn Error>> {
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command.spawn()?;

    let stdout = child
        .stdout
        .take()
        .ok_or("git diff did not provide a stdout pipe")?;
    let stderr = child
        .stderr
        .take()
        .ok_or("git diff did not provide a stderr pipe")?;
    let stderr_reader = drain_stderr(stderr);

    let changed = match parse_changed_lines(BufReader::new(stdout)) {
        Ok(changed) => changed,
        Err(error) => {
            terminate_child(child, stderr_reader);
            return Err(format!("failed to stream git diff output: {error}").into());
        }
    };
    let status = child.wait()?;
    let stderr_text = stderr_reader.finish();

    if !status.success() {
        return Err(format!("git diff failed with status {status}: {stderr_text}").into());
    }

    Ok(changed)
}

fn merge_base(root: &Path, base: &str) -> Result<String, Box<dyn Error>> {
    let output = performance::git_command()
        .arg("-C")
        .arg(root)
        .args(["merge-base", base, "HEAD"])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("could not find merge base for `{base}` and HEAD: {stderr}").into());
    }

    Ok(String::from_utf8(output.stdout)?.trim().to_string())
}

/// Parses a `--unified=0` unified diff stream into per-path added-line ranges.
///
/// The stream is consumed line by line with a bounded line buffer, so raw
/// patch bytes become facts and are released incrementally. Hunk headers carry
/// the new-side line count; the parser consumes exactly that many body lines,
/// which keeps structural headers (`diff --git`, `+++`, binary markers) from
/// being mistaken for hunk content and lets malformed bodies fail closed
/// instead of silently mis-attributing lines.
fn parse_changed_lines<R: BufRead>(reader: R) -> io::Result<ChangedLines> {
    parse_changed_lines_bounded(reader, MAX_DIFF_LINE_BYTES)
}

fn parse_changed_lines_bounded<R: BufRead>(
    mut reader: R,
    max_line_bytes: usize,
) -> io::Result<ChangedLines> {
    let mut changed = ChangedLines::default();
    let mut current_path: Option<PathBuf> = None;
    // Some(next new-file line number) while a hunk body is active.
    let mut current_new_line: Option<usize> = None;
    // Remaining body lines in the active hunk's new side (0 = none).
    let mut remaining_new_lines: usize = 0;
    let mut text = String::new();
    let mut scratch = Vec::new();

    loop {
        text.clear();
        if read_text_line_bounded(&mut reader, &mut text, &mut scratch, max_line_bytes)? == 0 {
            break;
        }
        let line = text.trim_end_matches(['\n', '\r']);

        if line.starts_with("@@") {
            let (start, count) = parse_hunk_header(line)?;
            current_new_line = Some(start);
            remaining_new_lines = count;
            continue;
        }

        if remaining_new_lines == 0 {
            current_new_line = None;
        } else if let Some(new_line) = current_new_line {
            match consume_hunk_body_line(&mut changed, current_path.as_deref(), line, new_line)? {
                HunkBodyOutcome::Advanced => {
                    current_new_line = Some(new_line.saturating_add(1));
                    remaining_new_lines -= 1;
                }
                HunkBodyOutcome::Retained => {}
            }
            continue;
        }

        // Outside any active hunk: only file headers are structural here.
        if let Some(target) = line.strip_prefix("+++ ") {
            current_path = parse_diff_target(target)?;
        }
    }

    Ok(changed)
}

enum HunkBodyOutcome {
    /// The line belongs to the hunk's new side and advanced the cursor.
    Advanced,
    /// The line is old-side or metadata and left the cursor unchanged.
    Retained,
}

fn consume_hunk_body_line(
    changed: &mut ChangedLines,
    path: Option<&Path>,
    line: &str,
    new_line: usize,
) -> io::Result<HunkBodyOutcome> {
    // An empty body line is an empty context line whose leading space some
    // producers trim away; it still occupies one new-side slot.
    let first = line.as_bytes().first().copied();
    match first {
        None | Some(b' ') => Ok(HunkBodyOutcome::Advanced),
        Some(b'\\') => Ok(HunkBodyOutcome::Retained),
        Some(b'-') => Ok(HunkBodyOutcome::Retained),
        Some(b'+') => {
            if let Some(path) = path {
                changed.insert(path, new_line);
            }
            Ok(HunkBodyOutcome::Advanced)
        }
        Some(_) => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("malformed hunk body line: {line:.80}"),
        )),
    }
}

/// Parses the new-side start and count out of a `@@ -a,b +c,d @@ context`
/// header. A missing count means one line. Malformed numbers fail closed:
/// silently skipping a header would mis-attribute every following line.
fn parse_hunk_header(header: &str) -> io::Result<(usize, usize)> {
    let malformed = || {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("malformed hunk header: {header:.120}"),
        )
    };

    let range = header
        .split_whitespace()
        .find(|part| part.starts_with('+'))
        .ok_or_else(malformed)?;
    let spec = range.trim_start_matches('+');
    let (start, count) = spec.split_once(',').unwrap_or((spec, ""));
    let start = start.parse::<usize>().map_err(|_| malformed())?;
    let count = if count.is_empty() {
        1
    } else {
        count.parse::<usize>().map_err(|_| malformed())?
    };
    Ok((start, count))
}

/// Resolves a `+++ ` target token into a tracked path. `/dev/null` marks a
/// created-from-nothing or deleted file. Git C-quotes targets containing
/// quotes, backslashes, or control characters; those decode back to exact
/// bytes. A trailing tab is Git's marker for paths with trailing whitespace
/// and is not part of the path.
fn parse_diff_target(token: &str) -> io::Result<Option<PathBuf>> {
    if token == "/dev/null" {
        return Ok(None);
    }
    let token = token.strip_suffix('\t').unwrap_or(token);
    match git_stream::c_style_unquote(token) {
        Ok(UnquotedPath::Verbatim) => Ok(Some(PathBuf::from(token))),
        Ok(UnquotedPath::Decoded(bytes)) => String::from_utf8(bytes)
            .map(|path| Some(PathBuf::from(path)))
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "quoted path is not UTF-8")),
        Err(reason) => Err(io::Error::new(io::ErrorKind::InvalidData, reason)),
    }
}

fn repository_dominant_name(counts: &BTreeMap<String, usize>) -> Option<&str> {
    let dominant_count = counts.values().copied().max()?;
    let mut dominant = counts
        .iter()
        .filter(|(_, count)| **count == dominant_count)
        .map(|(name, _)| name.as_str());
    let first = dominant.next()?;
    dominant.next().is_none().then_some(first)
}

fn file_local_name(local_names: &BTreeSet<String>) -> Option<&str> {
    if local_names.len() == 1 {
        local_names.iter().next().map(String::as_str)
    } else {
        None
    }
}

fn precedent_tension<'a>(
    counts: &'a BTreeMap<String, usize>,
    local_names: &'a BTreeSet<String>,
) -> Option<(&'a str, &'a str)> {
    let repository_name = repository_dominant_name(counts)?;
    let file_name = file_local_name(local_names)?;
    (repository_name != file_name).then_some((repository_name, file_name))
}

fn top_counts_summary(counts: &BTreeMap<String, usize>) -> String {
    let mut counts: Vec<_> = counts.iter().collect();
    counts.sort_by(|(name_a, count_a), (name_b, count_b)| {
        count_b.cmp(count_a).then(name_a.cmp(name_b))
    });

    counts
        .into_iter()
        .take(5)
        .map(|(name, count)| format!("`{name}`={count}"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn relative_path<'a>(root: &'a Path, path: &'a Path) -> &'a Path {
    path.strip_prefix(root).unwrap_or(path)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    fn unique_temp_dir(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "cargo-cultist-{name}-{}-{nanos}",
            std::process::id()
        ))
    }

    fn run_git(root: &Path, args: &[&str]) {
        let status = Command::new("git")
            .arg("-C")
            .arg(root)
            .args(args)
            .status()
            .unwrap();
        assert!(status.success(), "git command failed: git {args:?}");
    }

    fn init_repo(name: &str) -> PathBuf {
        let root = unique_temp_dir(name);
        fs::create_dir_all(&root).unwrap();
        run_git(&root, &["init", "-q"]);
        run_git(&root, &["config", "user.email", "cultist@example.invalid"]);
        run_git(&root, &["config", "user.name", "Cargo Cultist Tests"]);
        fs::write(root.join("README.md"), "baseline\n").unwrap();
        fs::write(root.join("changed.rs"), "fn baseline() {}\n").unwrap();
        fs::write(root.join("unrelated.rs"), [0xff, 0xfe, 0xfd]).unwrap();
        run_git(&root, &["add", "."]);
        run_git(&root, &["commit", "-q", "-m", "baseline"]);
        root
    }

    // ---- diff-time baseline aggregate fixtures ---------------------------

    struct IsolatedCaches {
        fact_root: PathBuf,
        scope_root: PathBuf,
        base: PathBuf,
    }

    fn isolated_caches(name: &str) -> IsolatedCaches {
        let base = unique_temp_dir(name);
        let isolated = IsolatedCaches {
            fact_root: base.join("facts"),
            scope_root: base.join("scopes"),
            base,
        };
        fs::create_dir_all(&isolated.fact_root).unwrap();
        fs::create_dir_all(&isolated.scope_root).unwrap();
        isolated
    }

    fn caches_for(isolated: &IsolatedCaches) -> (Option<FactCache>, Option<ScopeCache>) {
        (
            Some(FactCache {
                root: isolated.fact_root.clone(),
            }),
            Some(ScopeCache {
                root: isolated.scope_root.clone(),
            }),
        )
    }

    /// Seven-file repository with two directory scopes plus a root scope.
    fn init_multi_scope_repo(name: &str) -> PathBuf {
        let root = unique_temp_dir(name);
        fs::create_dir_all(root.join("a")).unwrap();
        fs::create_dir_all(root.join("b")).unwrap();
        fs::write(root.join("README.md"), "baseline\n").unwrap();
        fs::write(root.join("r.rs"), "#[cfg(test)]\nmod r_tests {}\n").unwrap();
        for (dir, prefix, count) in [("a", "f", 3), ("b", "g", 3)] {
            for index in 0..count {
                let module = format!("{dir}_{prefix}{index}_tests");
                fs::write(
                    root.join(dir).join(format!("{prefix}{index}.rs")),
                    format!("#[cfg(test)]\nmod {module} {{}}\n"),
                )
                .unwrap();
            }
        }
        run_git(&root, &["init", "-q"]);
        run_git(&root, &["config", "user.email", "cultist@example.invalid"]);
        run_git(&root, &["config", "user.name", "Cargo Cultist Tests"]);
        run_git(&root, &["add", "."]);
        run_git(&root, &["commit", "-q", "-m", "baseline"]);
        root
    }

    fn stage_append(root: &Path, relative: &str, addition: &str) {
        let path = root.join(relative);
        let mut current = fs::read_to_string(&path).unwrap();
        current.push_str(addition);
        fs::write(&path, current).unwrap();
        run_git(root, &["add", relative]);
    }

    /// The deterministic pre-change behavior: scan everything except the
    /// changed paths, then extend with the changed-file facts.
    fn legacy_findings(root: &Path) -> Vec<Finding> {
        let mut analysis =
            AnalysisReport::new("diff-precedent", root.to_string_lossy().into_owned());
        let changed = git_diff_changed_lines(root, None).unwrap();
        let changed_rust_paths: Vec<_> = changed.rust_paths().map(|path| root.join(path)).collect();
        assert!(
            !changed_rust_paths.is_empty(),
            "scenario must change Rust files"
        );
        let changed_report = analyze_test_module_files(&changed_rust_paths).unwrap();
        assert!(changed_report.parse_failures.is_empty());
        let excluded_paths: BTreeSet<_> = changed_rust_paths.into_iter().collect();
        let mut report = analyze_test_modules_excluding(root, &excluded_paths).unwrap();
        report.extend(changed_report);
        let precedent = TestModulePrecedent::from_occurrences(&report.occurrences);
        add_diff_findings(
            root,
            &report.occurrences,
            &changed,
            &precedent,
            &mut analysis,
        );
        analysis.findings
    }

    fn findings_text(findings: &[Finding]) -> String {
        findings
            .iter()
            .map(|finding| serde_json::to_string(finding).unwrap())
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn corrupt_scope_entries(scope_root: &Path) {
        for entry in fs::read_dir(scope_root).unwrap().flatten() {
            if entry.path().extension().and_then(|ext| ext.to_str()) == Some("json") {
                fs::write(entry.path(), b"corrupted bytes").unwrap();
            }
        }
    }

    fn age_scope_entries(scope_root: &Path) {
        for entry in fs::read_dir(scope_root).unwrap().flatten() {
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
                continue;
            }
            let mut envelope: serde_json::Value =
                serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
            envelope["schema_version"] = serde_json::Value::from(0);
            fs::write(&path, serde_json::to_vec(&envelope).unwrap()).unwrap();
        }
    }

    #[test]
    fn warm_small_diff_matches_full_scan_and_scales_with_affected_scope() {
        let root = init_multi_scope_repo("warm-small-diff");
        let isolated = isolated_caches("warm-small-diff-caches");

        stage_append(&root, "a/f0.rs", "\n#[cfg(test)]\nmod added_a_tests {}\n");
        let (_, cold) = performance::capture(|| {
            let (facts, scopes) = caches_for(&isolated);
            build_diff_analysis_report_with_caches(&root, None, facts, scopes).unwrap()
        });
        // The staged edit makes `a/f0.rs` volatile, so it is parsed fresh as
        // changed-file facts and never joins the clean scope tree: seven
        // parses cover the changed file plus the six remaining clean rows.
        assert_eq!(cold.rust_files_parsed, 7);
        assert_eq!(cold.rust_cache_hits, 0);
        assert_eq!(cold.baseline_scope_hits, 0);
        assert_eq!(cold.baseline_scope_computed, 3);

        stage_append(&root, "b/g1.rs", "\n#[cfg(test)]\nmod added_b_tests {}\n");
        let (warm_analysis, warm) = performance::capture(|| {
            let (facts, scopes) = caches_for(&isolated);
            build_diff_analysis_report_with_caches(&root, None, facts, scopes).unwrap()
        });

        // Both staged files are reparsed as changed-file facts; everything
        // else comes from caches: the sibling `a` scope is served whole and
        // only the affected `b` scope plus the root fold recompute, so work
        // scales with the affected scope instead of the repository rows.
        assert_eq!(warm.rust_files_parsed, 2);
        assert_eq!(warm.rust_cache_hits, 3);
        assert_eq!(warm.baseline_scope_hits, 1);
        assert_eq!(warm.baseline_scope_computed, 2);

        let oracle = legacy_findings(&root);
        assert_eq!(warm_analysis.findings.len(), 2);
        assert_eq!(warm_analysis.findings, oracle);

        let text = findings_text(&warm_analysis.findings);
        assert!(text.contains("`added_b_tests` appears 0 time(s)"));
        assert!(text.contains("`added_a_tests` appears 0 time(s)"));

        fs::remove_dir_all(root).unwrap();
        fs::remove_dir_all(isolated.base).unwrap();
    }

    #[test]
    fn corrupt_baseline_cache_recomputes_without_altering_findings() {
        let root = init_multi_scope_repo("corrupt-baseline");
        let isolated = isolated_caches("corrupt-baseline-caches");

        stage_append(&root, "a/f0.rs", "\n#[cfg(test)]\nmod added_a_tests {}\n");
        let _ = performance::capture(|| {
            let (facts, scopes) = caches_for(&isolated);
            build_diff_analysis_report_with_caches(&root, None, facts, scopes)
        });
        corrupt_scope_entries(&isolated.scope_root);

        let (analysis, counters) = performance::capture(|| {
            let (facts, scopes) = caches_for(&isolated);
            build_diff_analysis_report_with_caches(&root, None, facts, scopes).unwrap()
        });

        assert_eq!(counters.baseline_scope_hits, 0);
        assert_eq!(counters.baseline_scope_computed, 3);
        // Fact-layer reuse survives: corruption costs re-aggregation, not reparsing.
        // The clean tree holds six rows (the staged file stays volatile).
        assert_eq!(counters.rust_files_parsed, 1);
        assert_eq!(counters.rust_cache_hits, 6);

        assert_eq!(analysis.findings, legacy_findings(&root));

        fs::remove_dir_all(root).unwrap();
        fs::remove_dir_all(isolated.base).unwrap();
    }

    #[test]
    fn version_old_baseline_cache_is_ignored_and_recomputed() {
        let root = init_multi_scope_repo("stale-baseline-schema");
        let isolated = isolated_caches("stale-baseline-caches");

        stage_append(&root, "a/f0.rs", "\n#[cfg(test)]\nmod added_a_tests {}\n");
        let _ = performance::capture(|| {
            let (facts, scopes) = caches_for(&isolated);
            build_diff_analysis_report_with_caches(&root, None, facts, scopes)
        });
        age_scope_entries(&isolated.scope_root);

        let (analysis, counters) = performance::capture(|| {
            let (facts, scopes) = caches_for(&isolated);
            build_diff_analysis_report_with_caches(&root, None, facts, scopes).unwrap()
        });

        assert_eq!(counters.baseline_scope_hits, 0);
        assert_eq!(counters.baseline_scope_computed, 3);
        assert_eq!(analysis.findings, legacy_findings(&root));

        fs::remove_dir_all(root).unwrap();
        fs::remove_dir_all(isolated.base).unwrap();
    }

    #[test]
    fn untracked_file_outside_diff_forces_exact_fallback() {
        let root = init_multi_scope_repo("untracked-fallback");
        let isolated = isolated_caches("untracked-fallback-caches");

        stage_append(&root, "a/f0.rs", "\n#[cfg(test)]\nmod added_a_tests {}\n");
        // Untracked files never appear in `git diff HEAD`, so this row sits
        // outside the changed set and must invalidate the overlay instead of
        // being dropped from the precedent counts.
        fs::write(
            root.join("scratch.rs"),
            "#[cfg(test)]\nmod scratch_tests {}\n#[cfg(test)]\nmod added_a_tests {}\n",
        )
        .unwrap();

        let (analysis, counters) = performance::capture(|| {
            let (facts, scopes) = caches_for(&isolated);
            build_diff_analysis_report_with_caches(&root, None, facts, scopes).unwrap()
        });

        assert_eq!(counters.baseline_scope_hits, 0);
        assert_eq!(counters.baseline_scope_computed, 0);
        assert_eq!(analysis.findings, legacy_findings(&root));

        let text = findings_text(&analysis.findings);
        assert!(
            text.contains("`added_a_tests` appears 1 time(s)"),
            "the untracked contribution must be counted exactly: {text}"
        );

        fs::remove_dir_all(root).unwrap();
        fs::remove_dir_all(isolated.base).unwrap();
    }

    #[test]
    fn deleted_file_updates_precedent_counts() {
        let root = init_multi_scope_repo("deleted-scope");
        let isolated = isolated_caches("deleted-scope-caches");
        fs::create_dir_all(root.join("x")).unwrap();
        fs::write(
            root.join("x/only.rs"),
            "#[cfg(test)]\nmod target_tests {}\n",
        )
        .unwrap();
        run_git(&root, &["add", "x/only.rs"]);
        run_git(&root, &["commit", "-q", "-m", "add target"]);

        fs::remove_file(root.join("x/only.rs")).unwrap();
        stage_append(&root, "a/f0.rs", "\n#[cfg(test)]\nmod target_tests {}\n");

        let (analysis, counters) = performance::capture(|| {
            let (facts, scopes) = caches_for(&isolated);
            build_diff_analysis_report_with_caches(&root, None, facts, scopes).unwrap()
        });

        assert_eq!(counters.baseline_scope_hits, 0);
        assert_eq!(counters.baseline_scope_computed, 3);
        assert_eq!(analysis.findings, legacy_findings(&root));

        let text = findings_text(&analysis.findings);
        assert!(
            text.contains("`target_tests` appears 0 time(s)"),
            "the deleted file's declaration must leave the counts: {text}"
        );

        fs::remove_dir_all(root).unwrap();
        fs::remove_dir_all(isolated.base).unwrap();
    }

    #[test]
    fn detects_scope_tension_only_with_clear_precedent() {
        let counts = BTreeMap::from([("tests".to_string(), 33), ("unit_tests".to_string(), 88)]);
        let local_names = BTreeSet::from(["tests".to_string()]);
        assert_eq!(
            precedent_tension(&counts, &local_names),
            Some(("unit_tests", "tests"))
        );

        let mixed_local = BTreeSet::from(["tests".to_string(), "special_tests".to_string()]);
        assert_eq!(precedent_tension(&counts, &mixed_local), None);

        let tied_counts =
            BTreeMap::from([("tests".to_string(), 10), ("unit_tests".to_string(), 10)]);
        assert_eq!(precedent_tension(&tied_counts, &local_names), None);
    }

    #[test]
    fn excludes_changed_occurrence_from_precedent_counts() {
        let root = Path::new("/repo");
        let changed = TestModuleOccurrence {
            name: "unit_tests".to_string(),
            path: root.join("src/lib.rs"),
            line: 40,
        };
        let occurrences = vec![
            TestModuleOccurrence {
                name: "tests".to_string(),
                path: root.join("src/lib.rs"),
                line: 20,
            },
            changed.clone(),
        ];

        let precedent = TestModulePrecedent::from_occurrences(&occurrences);
        let (counts, total) = precedent.excluding("unit_tests");
        assert_eq!(counts.get("tests"), Some(&1));
        assert_eq!(counts.get("unit_tests"), None);
        assert_eq!(total, 1);
    }

    #[test]
    fn precedent_excluding_keeps_same_name_occurrences_elsewhere() {
        let root = Path::new("/repo");
        let occurrences = vec![
            TestModuleOccurrence {
                name: "shared".to_string(),
                path: root.join("src/a.rs"),
                line: 10,
            },
            TestModuleOccurrence {
                name: "shared".to_string(),
                path: root.join("src/b.rs"),
                line: 20,
            },
        ];
        let precedent = TestModulePrecedent::from_occurrences(&occurrences);

        let (counts, total) = precedent.excluding("shared");
        assert_eq!(counts.get("shared"), Some(&1));
        assert_eq!(total, 1);
    }

    #[test]
    fn parses_added_lines_from_zero_context_diff() {
        let patch = r#"diff --git src/a.rs src/a.rs
--- src/a.rs
+++ src/a.rs
@@ -10,0 +11,2 @@
+#[cfg(test)]
+mod special_tests {}
@@ -30 +32 @@
-old
+new
"#;

        let changed = parse_changed_lines(patch.as_bytes()).unwrap();
        assert!(changed.contains(Path::new("src/a.rs"), 11));
        assert!(changed.contains(Path::new("src/a.rs"), 12));
        assert!(changed.contains(Path::new("src/a.rs"), 32));
        assert!(!changed.contains(Path::new("src/a.rs"), 31));
        assert_eq!(changed.range_count(Path::new("src/a.rs")), 2);
    }

    #[test]
    fn compacts_contiguous_added_lines_into_one_range() {
        let patch = r#"diff --git src/a.rs src/a.rs
--- src/a.rs
+++ src/a.rs
@@ -10,0 +11,4 @@
+one
+two
+three
+four
"#;

        let changed = parse_changed_lines(patch.as_bytes()).unwrap();
        assert_eq!(changed.range_count(Path::new("src/a.rs")), 1);
        assert!(changed.contains(Path::new("src/a.rs"), 11));
        assert!(changed.contains(Path::new("src/a.rs"), 14));
        assert!(!changed.contains(Path::new("src/a.rs"), 15));
    }

    #[test]
    fn selects_test_modules_whose_declaration_line_was_added() {
        let root = Path::new("/repo");
        let occurrences = [
            TestModuleOccurrence {
                name: "tests".to_string(),
                path: root.join("src/lib.rs"),
                line: 20,
            },
            TestModuleOccurrence {
                name: "special_tests".to_string(),
                path: root.join("src/lib.rs"),
                line: 40,
            },
        ];
        let mut changed = ChangedLines::default();
        changed.insert(Path::new("src/lib.rs"), 40);

        let selected: Vec<_> = occurrences
            .iter()
            .filter(|occurrence| {
                occurrence
                    .path
                    .strip_prefix(root)
                    .is_ok_and(|path| changed.contains(path, occurrence.line))
            })
            .collect();
        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].name, "special_tests");
    }

    #[test]
    fn docs_only_diff_skips_repository_rust_scan() {
        let root = init_repo("docs-only-diff");
        fs::write(root.join("README.md"), "changed docs\n").unwrap();

        let analysis = build_diff_analysis_report(&root, None).unwrap();

        assert!(analysis.findings.is_empty());
        assert!(analysis.claims.iter().any(|claim| {
            claim
                .message
                .contains("No added or renamed test-gated module declarations")
        }));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn irrelevant_rust_diff_skips_unrelated_repository_rust_scan() {
        let root = init_repo("irrelevant-rust-diff");
        fs::write(
            root.join("changed.rs"),
            "fn changed() { println!(\"hi\"); }\n",
        )
        .unwrap();

        let analysis = build_diff_analysis_report(&root, None).unwrap();

        assert!(analysis.findings.is_empty());
        assert!(analysis.claims.iter().any(|claim| {
            claim
                .message
                .contains("No added or renamed test-gated module declarations")
        }));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn changed_rust_parse_failure_stays_unknown_without_repository_scan() {
        let root = init_repo("changed-parse-failure");
        fs::write(root.join("changed.rs"), "fn changed( {\n").unwrap();

        let analysis = build_diff_analysis_report(&root, None).unwrap();

        assert!(analysis.findings.is_empty());
        assert!(analysis.claims.iter().any(|claim| {
            claim.kind == ClaimKind::Unknown
                && claim
                    .message
                    .contains("diff relevance could not be determined")
        }));
        assert!(!analysis.claims.iter().any(|claim| {
            claim.kind == ClaimKind::Observed
                && claim
                    .message
                    .contains("No added or renamed test-gated module declarations")
        }));
        fs::remove_dir_all(root).unwrap();
    }

    struct TrickleReader<R> {
        inner: R,
    }

    impl<R: io::Read> io::Read for TrickleReader<R> {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            if buf.is_empty() {
                return Ok(0);
            }
            self.inner.read(&mut buf[..1])
        }
    }

    fn parse_bytes(bytes: &[u8]) -> ChangedLines {
        parse_changed_lines(io::Cursor::new(bytes)).unwrap()
    }

    #[test]
    fn fragmented_reads_parse_identically_to_whole_buffers() {
        let patch = b"diff --git src/a.rs src/a.rs\n\
                      --- src/a.rs\n\
                      +++ src/a.rs\r\n\
                      @@ -10,0 +11,2 @@\r\n\
                      +one\n\
                      +two\n\
                      @@ -30 +32 @@\n\
                      -old\n\
                      \\ No newline at end of file\n\
                      +new";
        let whole = parse_bytes(patch);
        let fragmented = parse_changed_lines(BufReader::new(TrickleReader {
            inner: io::Cursor::new(patch),
        }))
        .unwrap();
        assert_eq!(whole, fragmented);
        assert!(whole.contains(Path::new("src/a.rs"), 11));
        assert!(whole.contains(Path::new("src/a.rs"), 12));
        assert!(whole.contains(Path::new("src/a.rs"), 32));
        assert!(!whole.contains(Path::new("src/a.rs"), 13));
    }

    #[test]
    fn binary_file_markers_between_files_do_not_corrupt_ranges() {
        let patch = concat!(
            "diff --git src/a.rs src/a.rs\n",
            "--- src/a.rs\n",
            "+++ src/a.rs\n",
            "@@ -1,0 +2,1 @@\n",
            "+added\n",
            "diff --git data.bin data.bin\n",
            "index 111..222 100644\n",
            "Binary files data.bin and data.bin differ\n",
            "diff --git src/b.rs src/b.rs\n",
            "--- src/b.rs\n",
            "+++ src/b.rs\n",
            "@@ -5,0 +6,2 @@\n",
            "+first\n",
            "+second\n",
        );
        let changed = parse_bytes(patch.as_bytes());
        assert!(changed.contains(Path::new("src/a.rs"), 2));
        assert!(changed.contains(Path::new("src/b.rs"), 7));
        assert_eq!(changed.range_count(Path::new("src/a.rs")), 1);
        assert_eq!(changed.range_count(Path::new("src/b.rs")), 1);
        assert!(!changed.by_path.contains_key(Path::new("data.bin")));
    }

    #[test]
    fn counted_hunks_stop_consuming_at_the_header_count() {
        // An added line whose content itself starts with `++ ` renders as a
        // line beginning with `+++`; the counted state machine must treat it
        // as hunk content, not as the next file header.
        let patch = concat!(
            "diff --git src/a.rs src/a.rs\n",
            "--- src/a.rs\n",
            "+++ src/a.rs\n",
            "@@ -1,0 +2,3 @@\n",
            "+first\n",
            "+++ looks like a header\n",
            "+third\n",
        );
        let changed = parse_bytes(patch.as_bytes());
        assert_eq!(changed.range_count(Path::new("src/a.rs")), 1);
        assert!(changed.contains(Path::new("src/a.rs"), 3));
        assert!(changed.contains(Path::new("src/a.rs"), 4));
        assert!(!changed.contains(Path::new("looks like a header"), 1));
    }

    #[test]
    fn quoted_target_paths_decode_c_escapes() {
        let patch = concat!(
            "diff --git src/we\tird.rs src/we\tird.rs\n",
            "--- src/we\tird.rs\n",
            "+@@ header-shaped content\n",
            "+++ \"src/we\\tird.rs\"\n",
            "@@ -0,0 +1,1 @@\n",
            "+inside\n",
        );
        let changed = parse_bytes(patch.as_bytes());
        assert!(changed.contains(Path::new("src/we\tird.rs"), 1));
    }

    #[test]
    fn malformed_hunk_headers_fail_closed_instead_of_skipping() {
        for bad in [
            &b"+++ src/a.rs\n@@ garbage @@\n+x\n"[..],
            b"+++ src/a.rs\n@@ -1,0 +99999999999999999999,2 @@\n+x\n",
            b"+++ src/a.rs\n@@ -1,0 +18446744073709551616 @@\n+x\n",
        ] {
            let error = parse_changed_lines(io::Cursor::new(bad)).unwrap_err();
            assert_eq!(error.kind(), io::ErrorKind::InvalidData, "{error}");
        }
    }

    #[test]
    fn malformed_hunk_body_lines_fail_closed() {
        let patch = b"+++ src/a.rs\n@@ -1,0 +1,2 @@\n+ok\nunexpected body\n";
        let error = parse_changed_lines(io::Cursor::new(patch)).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn oversized_lines_fail_closed_with_a_bounded_buffer() {
        let long_line = vec![b'a'; 128];
        let mut patch = b"+++ src/a.rs\n@@ -1,0 +1,2 @@\n".to_vec();
        patch.extend_from_slice(&long_line);

        let error = parse_changed_lines_bounded(io::Cursor::new(&patch), 32).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn saturating_hunk_starts_never_overflow() {
        let max = usize::MAX;
        let patch = format!("+++ src/a.rs\n@@ -1,0 +{max},2 @@\n+a\n+b\n");
        let changed = parse_bytes(patch.as_bytes());
        assert!(changed.contains(Path::new("src/a.rs"), max));
        assert!(changed.range_count(Path::new("src/a.rs")) <= 2);
    }

    #[test]
    fn million_line_contiguous_addition_is_one_range_entry() {
        let started = std::time::Instant::now();
        const LINES: usize = 1_000_000;
        let mut patch = String::with_capacity(LINES * 8 + 64);
        patch.push_str("+++ src/generated.rs\n@@ -0,0 +1,");
        patch.push_str(&LINES.to_string());
        patch.push_str(" @@\n");
        for _ in 0..LINES {
            patch.push_str("+generated line\n");
        }

        let changed = parse_bytes(patch.as_bytes());
        let elapsed = started.elapsed();

        // Structural memory proxy: one merged range entry proves storage is
        // O(hunks), independent of the million added lines.
        assert_eq!(changed.total_range_entries(), 1);
        assert_eq!(changed.range_count(Path::new("src/generated.rs")), 1);
        assert!(changed.contains(Path::new("src/generated.rs"), 1));
        assert!(changed.contains(Path::new("src/generated.rs"), LINES));
        assert!(!changed.contains(Path::new("src/generated.rs"), LINES + 1));

        println!(
            "million-line contiguous addition: {} range entries in {elapsed:?}",
            changed.total_range_entries()
        );
        assert!(
            elapsed.as_secs() < 60,
            "million-line parse should stay far below wall-clock CI limits"
        );
    }

    #[test]
    fn sparse_additions_keep_disjoint_ranges_and_boundary_membership() {
        const HUNKS: usize = 50_000;
        let mut patch = String::from("+++ src/sparse.rs\n");
        for hunk in 0..HUNKS {
            let start = hunk * 4 + 1;
            patch.push_str(&format!("@@ -{start},0 +{start},1 @@\n+sparse\n"));
        }
        let changed = parse_bytes(patch.as_bytes());

        assert_eq!(changed.range_count(Path::new("src/sparse.rs")), HUNKS);
        assert_eq!(changed.total_range_entries(), HUNKS);
        for probe in [0_usize, 1, 2_000, 199_999, 200_000] {
            let expected = probe > 0 && (probe - 1) % 4 == 0 && probe <= HUNKS * 4;
            assert_eq!(
                changed.contains(Path::new("src/sparse.rs"), probe),
                expected,
                "probe {probe}"
            );
        }
    }

    #[test]
    fn overlapping_and_adjacent_ranges_merge_into_canonical_form() {
        let mut changed = ChangedLines::default();
        for line in [10_usize, 11, 12, 14, 15] {
            changed.insert(Path::new("src/a.rs"), line);
        }
        assert_eq!(changed.range_count(Path::new("src/a.rs")), 2);
        assert!(changed.contains(Path::new("src/a.rs"), 12));
        assert!(!changed.contains(Path::new("src/a.rs"), 13));
        assert!(changed.contains(Path::new("src/a.rs"), 15));

        let later_file = ChangedLines {
            by_path: BTreeMap::from([(
                PathBuf::from("src/b.rs"),
                vec![
                    LineRange { start: 5, end: 6 },
                    LineRange { start: 9, end: 9 },
                    LineRange {
                        start: 100,
                        end: 100,
                    },
                    LineRange {
                        start: 4096,
                        end: 8192,
                    },
                ],
            )]),
        };
        for line in [5_usize, 6, 8, 9, 100, 4095, 5000, 8192, 8193] {
            assert_eq!(
                later_file.contains(Path::new("src/b.rs"), line),
                [5, 6, 9, 100, 5000, 8192].contains(&line),
                "probe {line}"
            );
        }
    }

    #[test]
    fn parser_errors_kill_and_reap_the_child_promptly() {
        let started = std::time::Instant::now();
        let mut command = Command::new("/bin/sh");
        command
            .arg("-c")
            .arg("printf '+++ src/a.rs\n@@ broken header @@\n'; sleep 30");
        let error = run_git_diff_streaming(command).expect_err("malformed stream must fail");

        assert!(
            error.to_string().contains("malformed hunk header"),
            "{error}"
        );
        assert!(
            started.elapsed().as_secs() < 20,
            "a stuck child must be killed instead of waited on: {:?}",
            started.elapsed()
        );
    }

    #[test]
    fn stderr_pressure_cannot_deadlock_stdout_parsing() {
        let mut command = Command::new("/bin/sh");
        command.arg("-c").arg(
            "yes x | head -c 2097152 >&2; \
             printf '+++ src/a.rs\n@@ -0,0 +1,1 @@\n+line\n'",
        );
        let changed = run_git_diff_streaming(command).unwrap();
        assert!(changed.contains(Path::new("src/a.rs"), 1));
    }

    #[test]
    fn nonzero_child_exit_reports_stderr_text() {
        let outside = unique_temp_dir("outside-repo");
        fs::create_dir_all(&outside).unwrap();
        let mut command = Command::new("git");
        command.arg("-C").arg(&outside).arg("log").arg("-1");
        let error = run_git_diff_streaming(command).expect_err("outside a repo git must fail");

        let message = error.to_string();
        assert!(message.contains("failed with status"), "{message}");
        assert!(message.contains("fatal:"), "{message}");
        fs::remove_dir_all(outside).unwrap();
    }
}
