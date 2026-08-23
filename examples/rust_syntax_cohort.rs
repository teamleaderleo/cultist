use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::error::Error;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::str::FromStr;

use proc_macro2::{Delimiter, Group, TokenStream, TokenTree};

const DEFAULT_MAX_COMMITS: usize = 100;
const MAX_PATHS_PER_COMMIT: usize = 100;
const TOP_COMPANIONS: usize = 15;

#[derive(Debug, Clone)]
struct Commit {
    sha: String,
    subject: String,
    paths: BTreeSet<PathBuf>,
    edit_class: RustEditClass,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(crate) enum RustEditClass {
    SyntaxChanged,
    CommentsOrWhitespaceOnly,
    Unclassified,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("rust-syntax-cohort: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let mut args = env::args().skip(1);
    let root = PathBuf::from(
        args.next()
            .ok_or("usage: rust_syntax_cohort REPO FILE [MAX_COMMITS]")?,
    )
    .canonicalize()?;
    let anchor = PathBuf::from(
        args.next()
            .ok_or("usage: rust_syntax_cohort REPO FILE [MAX_COMMITS]")?,
    );
    let max_commits = args
        .next()
        .map(|value| value.parse::<usize>())
        .transpose()?
        .unwrap_or(DEFAULT_MAX_COMMITS);
    if args.next().is_some() {
        return Err("usage: rust_syntax_cohort REPO FILE [MAX_COMMITS]".into());
    }
    if anchor.is_absolute() {
        return Err("FILE must be repository-relative".into());
    }
    if anchor.extension().and_then(|ext| ext.to_str()) != Some("rs") {
        return Err("this probe currently requires a Rust source file".into());
    }

    let shas = history_shas(&root, &anchor, max_commits)?;
    let discovered = shas.len();
    let mut focused = Vec::new();
    let mut excluded_reverts = 0;
    let mut excluded_broad = 0;

    for sha in shas {
        let (subject, paths) = commit_metadata(&root, &sha)?;
        if is_revert_subject(&subject) {
            excluded_reverts += 1;
            continue;
        }
        if paths.len() > MAX_PATHS_PER_COMMIT {
            excluded_broad += 1;
            continue;
        }
        let edit_class = classify_rust_edit(&root, &sha, &anchor);
        focused.push(Commit {
            sha,
            subject,
            paths,
            edit_class,
        });
    }

    let syntax: Vec<_> = focused
        .iter()
        .filter(|commit| commit.edit_class == RustEditClass::SyntaxChanged)
        .collect();
    let comments_only: Vec<_> = focused
        .iter()
        .filter(|commit| commit.edit_class == RustEditClass::CommentsOrWhitespaceOnly)
        .collect();
    let unclassified = focused
        .iter()
        .filter(|commit| commit.edit_class == RustEditClass::Unclassified)
        .count();

    println!("RUST SYNTAX COHORT PROBE");
    println!("  repository: {}", root.display());
    println!("  anchor: {}", anchor.display());
    println!("  discovered non-merge commits: {discovered}");
    println!("  focused commits: {}", focused.len());
    println!("  syntax-changing commits: {}", syntax.len());
    println!(
        "  comments/docs/whitespace-only commits: {}",
        comments_only.len()
    );
    println!("  unclassified commits: {unclassified}");
    println!("  excluded reverts: {excluded_reverts}");
    println!("  excluded broad commits: {excluded_broad}");

    if !comments_only.is_empty() {
        println!("\nCOMMENTS/DOCS/WHITESPACE-ONLY EXAMPLES");
        for commit in comments_only.iter().take(10) {
            println!("  {}  {}", short_sha(&commit.sha), commit.subject);
        }
    }

    let all_refs: Vec<_> = focused.iter().collect();
    let all_counts = companion_counts(&anchor, &all_refs);
    let syntax_counts = companion_counts(&anchor, &syntax);

    let mut paths: Vec<_> = syntax_counts.keys().cloned().collect();
    paths.sort_by(|a, b| {
        syntax_counts[b]
            .cmp(&syntax_counts[a])
            .then_with(|| {
                all_counts
                    .get(b)
                    .unwrap_or(&0)
                    .cmp(all_counts.get(a).unwrap_or(&0))
            })
            .then_with(|| a.cmp(b))
    });

    println!("\nDIRECTIONAL COMPANIONS");
    println!("  P(companion changes | anchor has a Rust syntax change)");
    for path in paths.into_iter().take(TOP_COMPANIONS) {
        let syntax_support = syntax_counts[&path];
        let all_support = all_counts.get(&path).copied().unwrap_or_default();
        println!(
            "  {:<64} syntax {:>3}/{:<3} {:>5.1}%   all {:>3}/{:<3} {:>5.1}%",
            path.display(),
            syntax_support,
            syntax.len(),
            percent(syntax_support, syntax.len()),
            all_support,
            focused.len(),
            percent(all_support, focused.len())
        );
    }

    println!("\nINTERPRETATION BOUNDARY");
    println!("  This probe refines the historical cohort by lexical Rust syntax identity.");
    println!(
        "  It ignores comments, doc attributes, and whitespace when deciding whether the anchor's Rust syntax changed."
    );
    println!(
        "  Companion frequency remains association evidence; generator ownership and semantic relation type require separate evidence."
    );

    Ok(())
}

fn history_shas(
    root: &Path,
    anchor: &Path,
    max_commits: usize,
) -> Result<Vec<String>, Box<dyn Error>> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["log", "--no-merges", "--format=%H", "-n"])
        .arg(max_commits.to_string())
        .arg("--")
        .arg(anchor)
        .output()?;
    if !output.status.success() {
        return Err(format!(
            "git log failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    Ok(String::from_utf8(output.stdout)?
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToOwned::to_owned)
        .collect())
}

fn commit_metadata(root: &Path, sha: &str) -> Result<(String, BTreeSet<PathBuf>), Box<dyn Error>> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args([
            "-c",
            "core.quotepath=false",
            "show",
            "--format=%s%x1e",
            "--name-only",
            "--no-renames",
            "--no-color",
            "--no-ext-diff",
            "--root",
        ])
        .arg(sha)
        .arg("--")
        .output()?;
    if !output.status.success() {
        return Err(format!(
            "git show failed for {sha}: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    let text = String::from_utf8(output.stdout)?;
    let (subject, paths) = text
        .split_once('\u{1e}')
        .ok_or_else(|| format!("could not parse git show output for {sha}"))?;
    let paths = paths
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(PathBuf::from)
        .collect();
    Ok((subject.trim().to_string(), paths))
}

pub(crate) fn classify_rust_edit(root: &Path, sha: &str, anchor: &Path) -> RustEditClass {
    let after = source_at_revision(root, sha, anchor);
    let before = source_at_revision(root, &format!("{sha}^"), anchor);
    let (Some(before), Some(after)) = (before, after) else {
        return RustEditClass::Unclassified;
    };
    let (Ok(before), Ok(after)) = (
        rust_syntax_fingerprint(&before),
        rust_syntax_fingerprint(&after),
    ) else {
        return RustEditClass::Unclassified;
    };
    if before == after {
        RustEditClass::CommentsOrWhitespaceOnly
    } else {
        RustEditClass::SyntaxChanged
    }
}

fn source_at_revision(root: &Path, revision: &str, anchor: &Path) -> Option<String> {
    let spec = format!("{revision}:{}", anchor.to_string_lossy().replace('\\', "/"));
    let output = Command::new("git")
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

fn rust_syntax_fingerprint(source: &str) -> Result<String, Box<dyn Error>> {
    let tokens = TokenStream::from_str(source)?;
    Ok(strip_doc_attributes(tokens).to_string())
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
    if group.delimiter() != Delimiter::Bracket {
        return false;
    }
    matches!(group.stream().into_iter().next(), Some(TokenTree::Ident(ident)) if ident == "doc")
}

fn companion_counts(anchor: &Path, commits: &[&Commit]) -> BTreeMap<PathBuf, usize> {
    let mut counts = BTreeMap::new();
    for commit in commits {
        for path in &commit.paths {
            if path != anchor {
                *counts.entry(path.clone()).or_default() += 1;
            }
        }
    }
    counts
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
    use super::*;

    #[test]
    fn ignores_line_and_doc_comment_changes() {
        let before = r#"
            // old comment
            /// old docs
            fn answer() -> usize { 42 }
        "#;
        let after = r#"
            // new comment
            /// new docs
            fn answer() -> usize { 42 }
        "#;
        assert_eq!(
            rust_syntax_fingerprint(before).unwrap(),
            rust_syntax_fingerprint(after).unwrap()
        );
    }

    #[test]
    fn ignores_inner_doc_comment_changes() {
        let before = "//! old docs\nfn answer() {}";
        let after = "//! new docs\nfn answer() {}";
        assert_eq!(
            rust_syntax_fingerprint(before).unwrap(),
            rust_syntax_fingerprint(after).unwrap()
        );
    }

    #[test]
    fn detects_code_changes() {
        let before = "fn answer() -> usize { 41 }";
        let after = "fn answer() -> usize { 42 }";
        assert_ne!(
            rust_syntax_fingerprint(before).unwrap(),
            rust_syntax_fingerprint(after).unwrap()
        );
    }

    #[test]
    fn preserves_non_doc_attributes() {
        let before = "#[cfg(unix)] fn answer() {}";
        let after = "#[cfg(windows)] fn answer() {}";
        assert_ne!(
            rust_syntax_fingerprint(before).unwrap(),
            rust_syntax_fingerprint(after).unwrap()
        );
    }
}
