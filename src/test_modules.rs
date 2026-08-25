use std::collections::BTreeSet;
use std::error::Error;
use std::path::{Path, PathBuf};

use crate::performance;
use crate::rust_facts::{RustFactScan, scan_rust_paths, scan_rust_repository};

pub(crate) const SKIPPED_DIRS: &[&str] = &[".git", "target", "node_modules"];

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TestModuleOccurrence {
    pub name: String,
    pub path: PathBuf,
    pub line: usize,
}

#[derive(Debug, Default)]
pub struct TestModuleReport {
    pub occurrences: Vec<TestModuleOccurrence>,
    pub parse_failures: Vec<(PathBuf, String)>,
}

impl TestModuleReport {
    pub fn extend(&mut self, other: Self) {
        self.occurrences.extend(other.occurrences);
        self.parse_failures.extend(other.parse_failures);
        sort_report(self);
    }
}

pub fn analyze_test_modules(root: &Path) -> Result<TestModuleReport, Box<dyn Error>> {
    analyze_test_modules_excluding(root, &BTreeSet::new())
}

pub fn analyze_test_modules_excluding(
    root: &Path,
    excluded_paths: &BTreeSet<PathBuf>,
) -> Result<TestModuleReport, Box<dyn Error>> {
    let scan = scan_rust_repository(root, excluded_paths, SKIPPED_DIRS)?;
    Ok(test_module_report(scan))
}

pub fn analyze_test_module_files(paths: &[PathBuf]) -> Result<TestModuleReport, Box<dyn Error>> {
    Ok(test_module_report(scan_rust_paths(paths)?))
}

fn test_module_report(scan: RustFactScan) -> TestModuleReport {
    performance::record_rust_scan(scan.parsed_files, scan.cache_hits);
    let mut report = TestModuleReport::default();

    for file in scan.files {
        if let Some(error) = file.facts.parse_error {
            report.parse_failures.push((file.path, error));
            continue;
        }

        for occurrence in file.facts.test_modules {
            report.occurrences.push(TestModuleOccurrence {
                name: occurrence.name,
                path: file.path.clone(),
                line: occurrence.line,
            });
        }
    }

    sort_report(&mut report);
    report
}

fn sort_report(report: &mut TestModuleReport) {
    report
        .occurrences
        .sort_by(|a, b| a.path.cmp(&b.path).then(a.line.cmp(&b.line)));
    report.parse_failures.sort_by(|a, b| a.0.cmp(&b.0));
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::process::Command;
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

    fn names(source: &str) -> Vec<String> {
        let root = unique_temp_dir("test-module-names");
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("fixture.rs");
        std::fs::write(&path, source).unwrap();
        let report = analyze_test_module_files(&[path]).unwrap();
        std::fs::remove_dir_all(root).unwrap();
        report
            .occurrences
            .into_iter()
            .map(|occurrence| occurrence.name)
            .collect()
    }

    #[test]
    fn finds_test_gated_modules() {
        assert_eq!(
            names(
                r#"
                #[cfg(test)]
                mod tests {}

                #[cfg(all(unix, test))]
                mod unix_tests {}
                "#,
            ),
            vec!["tests".to_string(), "unix_tests".to_string()]
        );
    }

    #[test]
    fn ignores_ordinary_and_not_test_modules() {
        assert!(names("mod production {}").is_empty());
        assert!(names("#[cfg(not(test))] mod production {}").is_empty());
    }

    #[test]
    fn targeted_scan_reads_only_requested_files() {
        let root = unique_temp_dir("targeted-scan");
        fs::create_dir_all(&root).unwrap();
        let selected = root.join("selected.rs");
        let ignored = root.join("ignored.rs");
        fs::write(&selected, "#[cfg(test)]\nmod selected_tests {}\n").unwrap();
        fs::write(&ignored, "this is deliberately invalid Rust {{{").unwrap();

        let report = analyze_test_module_files(std::slice::from_ref(&selected)).unwrap();

        assert_eq!(report.occurrences.len(), 1);
        assert_eq!(report.occurrences[0].name, "selected_tests");
        assert!(report.parse_failures.is_empty());

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn repository_scan_budget_counts_git_and_parsed_files() {
        let root = unique_temp_dir("repository-scan-budget");
        fs::create_dir_all(root.join("src")).unwrap();
        run_git(&root, &["init", "-q"]);
        fs::write(root.join("src/lib.rs"), "#[cfg(test)]\nmod tests {}\n").unwrap();

        let (report, counters) = performance::capture(|| analyze_test_modules(&root).unwrap());

        assert_eq!(report.occurrences.len(), 1);
        assert_eq!(counters.git_subprocesses, 4);
        assert_eq!(counters.rust_files_parsed, 1);
        assert_eq!(counters.rust_cache_hits, 0);

        fs::remove_dir_all(root).unwrap();
    }
}
