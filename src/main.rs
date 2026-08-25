mod active_changes;
mod active_inventory_provider_snapshot;
#[allow(dead_code)]
mod applicability;
mod ci_test_filters;
mod diff;
mod finding;
mod generated_diff;
mod git_stream;
mod history;
mod performance;
mod preflight;
mod provider_snapshot_applicability;
mod render;
mod report;
mod rust_facts;
mod test_module_aggregate;
mod test_modules;

use std::env;
use std::error::Error;
use std::io::{self, BufWriter, Write as _};
use std::path::PathBuf;
use std::process;

use active_inventory_provider_snapshot::build_active_inventory_analysis_report_with_provider_snapshot;
use ci_test_filters::{analyze_ci_test_filters, build_ci_test_filter_analysis};
use diff::{build_diff_analysis_report, git_repo_root};
use finding::AnalysisReport;
use history::{
    DEFAULT_MAX_COMMITS, HistoryOptions, analyze_historical_companions, print_history_report,
};
use preflight::build_preflight_analysis_report;
use provider_snapshot_applicability::ProviderSnapshotIdentity;
use report::build_test_module_analysis;
use test_modules::analyze_test_modules;

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq)]
enum OutputFormat {
    #[default]
    Text,
    Json,
}

#[derive(Debug, Default, Eq, PartialEq)]
struct DiffArgs {
    base: Option<String>,
    path: Option<PathBuf>,
    format: OutputFormat,
}

#[derive(Debug, Eq, PartialEq)]
enum PreflightSource {
    Against(String),
    Inventory(PathBuf),
}

#[derive(Debug, Eq, PartialEq)]
struct PreflightArgs {
    source: PreflightSource,
    current_provider_snapshot: Option<ProviderSnapshotIdentity>,
    path: Option<PathBuf>,
    format: OutputFormat,
}

#[derive(Debug, Eq, PartialEq)]
struct HistoryArgs {
    path: PathBuf,
    max_commits: usize,
    format: OutputFormat,
}

fn main() {
    performance::init_from_environment();
    let result = run();
    if let Err(error) = &result {
        eprintln!("cargo-cultist: {error}");
    }
    performance::emit_if_enabled();
    if result.is_err() {
        process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let mut args: Vec<String> = env::args().skip(1).collect();

    // Cargo invokes third-party subcommands as `cargo-<name> <name> ...`.
    // Accept direct invocation (`cargo-cultist`) too.
    if args.first().is_some_and(|arg| arg == "cultist") {
        args.remove(0);
    }

    if args.first().is_some_and(|arg| is_change_command(arg)) {
        let command = args.remove(0);
        return run_diff(&command, args);
    }

    if args.first().is_some_and(|arg| arg == "preflight") {
        args.remove(0);
        return run_preflight(args);
    }

    if args.first().is_some_and(|arg| arg == "history") {
        args.remove(0);
        return run_history(args);
    }

    if args.first().is_some_and(|arg| arg == "ci-tests") {
        args.remove(0);
        return run_ci_tests(args);
    }

    if args.iter().any(|arg| arg == "-h" || arg == "--help") {
        print_help();
        return Ok(());
    }

    if args.iter().any(|arg| arg == "-V" || arg == "--version") {
        println!("cargo-cultist {VERSION}");
        return Ok(());
    }

    let (format, path) = parse_root_args(args)?;
    let root = path.unwrap_or(env::current_dir()?).canonicalize()?;
    let report = analyze_test_modules(&root)?;
    let analysis = build_test_module_analysis(&root, &report);
    emit_analysis(&analysis, format)
}

fn is_change_command(arg: &str) -> bool {
    matches!(arg, "check" | "diff")
}

fn run_diff(command: &str, args: Vec<String>) -> Result<(), Box<dyn Error>> {
    if args.iter().any(|arg| arg == "-h" || arg == "--help") {
        print_change_help(command);
        return Ok(());
    }

    let DiffArgs { base, path, format } = parse_diff_args(args, command)?;
    let requested_root = path.unwrap_or(env::current_dir()?);
    let requested_root = requested_root.canonicalize()?;
    let root = git_repo_root(&requested_root)?;
    let analysis = build_diff_analysis_report(&root, base.as_deref())?;
    emit_analysis(&analysis, format)
}

fn run_preflight(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    if args.iter().any(|arg| arg == "-h" || arg == "--help") {
        print_preflight_help();
        return Ok(());
    }

    let PreflightArgs {
        source,
        current_provider_snapshot,
        path,
        format,
    } = parse_preflight_args(args)?;

    let requested = path.unwrap_or(env::current_dir()?);
    let requested = requested.canonicalize()?;
    let probe = if requested.is_file() {
        requested
            .parent()
            .ok_or("could not determine the preflight path's parent directory")?
    } else {
        requested.as_path()
    };
    let root = git_repo_root(probe)?;
    let relative = requested
        .strip_prefix(&root)
        .map_err(|_| "preflight path is outside the resolved Git repository")?;
    let scope = (!relative.as_os_str().is_empty()).then(|| relative.to_path_buf());

    let analysis = match source {
        PreflightSource::Against(against) => {
            build_preflight_analysis_report(&root, &against, scope.as_deref())?
        }
        PreflightSource::Inventory(inventory) => {
            let inventory = if inventory.is_absolute() {
                inventory
            } else {
                env::current_dir()?.join(inventory)
            };
            let inventory = inventory.canonicalize()?;
            if !inventory.is_file() {
                return Err(format!(
                    "active-change inventory is not a file: {}",
                    inventory.display()
                )
                .into());
            }
            build_active_inventory_analysis_report_with_provider_snapshot(
                &root,
                &inventory,
                scope.as_deref(),
                current_provider_snapshot.as_ref(),
            )?
        }
    };
    emit_analysis(&analysis, format)
}

fn emit_analysis(analysis: &AnalysisReport, format: OutputFormat) -> Result<(), Box<dyn Error>> {
    let stdout = io::stdout();
    let mut writer = BufWriter::new(stdout.lock());
    match format {
        OutputFormat::Text => {
            writeln!(writer, "cargo-cultist {VERSION}")?;
            writeln!(writer, "repository: {}\n", analysis.repository)?;
            render::write_analysis_report(&mut writer, analysis)?;
        }
        OutputFormat::Json => {
            serde_json::to_writer_pretty(&mut writer, analysis)?;
            writeln!(writer)?;
        }
    }
    writer.flush()?;
    Ok(())
}

fn run_history(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    if args.iter().any(|arg| arg == "-h" || arg == "--help") {
        print_history_help();
        return Ok(());
    }

    let HistoryArgs {
        path,
        max_commits,
        format,
    } = parse_history_args(args)?;

    let requested = if path.is_absolute() {
        path
    } else {
        env::current_dir()?.join(path)
    };
    let requested = requested.canonicalize()?;
    if !requested.is_file() {
        return Err(format!(
            "history currently expects a file path; got {}",
            requested.display()
        )
        .into());
    }

    let probe = requested
        .parent()
        .ok_or("could not determine the history path's parent directory")?;
    let root = git_repo_root(probe)?;
    let anchor = requested
        .strip_prefix(&root)
        .map_err(|_| "history path is outside the resolved Git repository")?;

    let report = analyze_historical_companions(
        &root,
        anchor,
        HistoryOptions {
            max_commits,
            ..HistoryOptions::default()
        },
    )?;

    match format {
        OutputFormat::Text => {
            println!("cargo-cultist {VERSION}");
            println!("repository: {}\n", root.display());
            print_history_report(&report);
        }
        OutputFormat::Json => {
            let stdout = io::stdout();
            let mut writer = BufWriter::new(stdout.lock());
            serde_json::to_writer_pretty(&mut writer, &report)?;
            writeln!(writer)?;
            writer.flush()?;
        }
    }

    Ok(())
}

fn run_ci_tests(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    if args.iter().any(|arg| arg == "-h" || arg == "--help") {
        print_ci_tests_help();
        return Ok(());
    }

    let (format, path) = parse_root_args(args)?;
    let requested_root = path.unwrap_or(env::current_dir()?);
    let requested_root = requested_root.canonicalize()?;
    let root = git_repo_root(&requested_root)?;
    let report = analyze_ci_test_filters(&root)?;
    let analysis = build_ci_test_filter_analysis(&root, &report);
    emit_analysis(&analysis, format)
}

fn parse_root_args(args: Vec<String>) -> Result<(OutputFormat, Option<PathBuf>), Box<dyn Error>> {
    let mut format = OutputFormat::Text;
    let mut path = None;
    let mut args = args.into_iter();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--format" => {
                format = parse_output_format(
                    &args.next().ok_or("`--format` requires `text` or `json`")?,
                )?;
            }
            _ if arg.starts_with('-') => {
                return Err(format!("unknown option `{arg}`; try `cargo cultist --help`").into());
            }
            _ => {
                if path.is_some() {
                    return Err(
                        "expected at most one path argument; try `cargo cultist --help`".into(),
                    );
                }
                path = Some(PathBuf::from(arg));
            }
        }
    }

    Ok((format, path))
}

fn parse_diff_args(args: Vec<String>, command: &str) -> Result<DiffArgs, Box<dyn Error>> {
    debug_assert!(is_change_command(command));
    let mut parsed = DiffArgs::default();
    let mut args = args.into_iter();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--base" => {
                if parsed.base.is_some() {
                    return Err("`--base` may only be specified once".into());
                }
                parsed.base = Some(args.next().ok_or("`--base` requires a Git revision")?);
            }
            "--format" => {
                parsed.format = parse_output_format(
                    &args.next().ok_or("`--format` requires `text` or `json`")?,
                )?;
            }
            _ if arg.starts_with('-') => {
                return Err(format!(
                    "unknown {command} option `{arg}`; try `cargo cultist {command} --help`"
                )
                .into());
            }
            _ => {
                if parsed.path.is_some() {
                    return Err(format!(
                        "expected at most one path argument; try `cargo cultist {command} --help`"
                    )
                    .into());
                }
                parsed.path = Some(PathBuf::from(arg));
            }
        }
    }

    Ok(parsed)
}

fn parse_preflight_args(args: Vec<String>) -> Result<PreflightArgs, Box<dyn Error>> {
    let mut against = None;
    let mut inventory = None;
    let mut current_provider_snapshot = None;
    let mut path = None;
    let mut format = OutputFormat::Text;
    let mut args = args.into_iter();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--current-provider-snapshot" => {
                if current_provider_snapshot.is_some() {
                    return Err("`--current-provider-snapshot` may only be specified once".into());
                }
                let value = args
                    .next()
                    .ok_or("`--current-provider-snapshot` requires `sha256:<64-lowercase-hex>`")?;
                current_provider_snapshot = Some(ProviderSnapshotIdentity::parse(value)?);
            }
            "--against" => {
                if against.is_some() {
                    return Err("`--against` may only be specified once".into());
                }
                against = Some(args.next().ok_or("`--against` requires a Git revision")?);
            }
            "--inventory" => {
                if inventory.is_some() {
                    return Err("`--inventory` may only be specified once".into());
                }
                inventory = Some(PathBuf::from(
                    args.next().ok_or("`--inventory` requires a JSON file")?,
                ));
            }
            "--format" => {
                format = parse_output_format(
                    &args.next().ok_or("`--format` requires `text` or `json`")?,
                )?;
            }
            _ if arg.starts_with('-') => {
                return Err(format!(
                    "unknown preflight option `{arg}`; try `cargo cultist preflight --help`"
                )
                .into());
            }
            _ => {
                if path.is_some() {
                    return Err(
                        "expected at most one path argument; try `cargo cultist preflight --help`"
                            .into(),
                    );
                }
                path = Some(PathBuf::from(arg));
            }
        }
    }

    let source = match (against, inventory) {
        (Some(against), None) => PreflightSource::Against(against),
        (None, Some(inventory)) => PreflightSource::Inventory(inventory),
        (Some(_), Some(_)) => {
            return Err(
                "preflight accepts only one of `--against REV` or `--inventory FILE`".into(),
            );
        }
        (None, None) => {
            return Err(
                "preflight requires exactly one of `--against REV` or `--inventory FILE`".into(),
            );
        }
    };

    if current_provider_snapshot.is_some() && matches!(&source, PreflightSource::Against(_)) {
        return Err("`--current-provider-snapshot` is only valid with `--inventory FILE`".into());
    }

    Ok(PreflightArgs {
        source,
        current_provider_snapshot,
        path,
        format,
    })
}

fn parse_history_args(args: Vec<String>) -> Result<HistoryArgs, Box<dyn Error>> {
    let mut path = None;
    let mut max_commits = DEFAULT_MAX_COMMITS;
    let mut format = OutputFormat::Text;
    let mut args = args.into_iter();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--max-commits" => {
                let value = args
                    .next()
                    .ok_or("`--max-commits` requires a positive integer")?;
                max_commits = value
                    .parse::<usize>()
                    .map_err(|_| "`--max-commits` requires a positive integer")?;
                if max_commits == 0 {
                    return Err("`--max-commits` requires a positive integer".into());
                }
            }
            "--format" => {
                format = parse_output_format(
                    &args.next().ok_or("`--format` requires `text` or `json`")?,
                )?;
            }
            _ if arg.starts_with('-') => {
                return Err(format!(
                    "unknown history option `{arg}`; try `cargo cultist history --help`"
                )
                .into());
            }
            _ => {
                if path.is_some() {
                    return Err(
                        "history expects exactly one file path; try `cargo cultist history --help`"
                            .into(),
                    );
                }
                path = Some(PathBuf::from(arg));
            }
        }
    }

    Ok(HistoryArgs {
        path: path.ok_or("history requires a file path")?,
        max_commits,
        format,
    })
}

fn parse_output_format(value: &str) -> Result<OutputFormat, String> {
    match value {
        "text" => Ok(OutputFormat::Text),
        "json" => Ok(OutputFormat::Json),
        _ => Err(format!(
            "unsupported output format `{value}`; expected `text` or `json`"
        )),
    }
}

fn print_help() {
    println!(
        r#"cargo-cultist {VERSION}
Recover repository evidence that can change the next justified action.

USAGE:
    cargo cultist [--format text|json] [PATH]
    cargo cultist check [--base REV] [--format text|json] [PATH]
    cargo cultist diff [--base REV] [--format text|json] [PATH]
    cargo cultist preflight --against REV [--format text|json] [PATH]
    cargo cultist preflight --inventory FILE [--current-provider-snapshot sha256:HEX] [--format text|json] [PATH]
    cargo cultist history [--max-commits N] [--format text|json] FILE
    cargo cultist ci-tests [--format text|json] [PATH]
    cargo-cultist [--format text|json] [PATH]
    cargo-cultist check [--base REV] [--format text|json] [PATH]
    cargo-cultist diff [--base REV] [--format text|json] [PATH]
    cargo-cultist preflight --against REV [--format text|json] [PATH]
    cargo-cultist preflight --inventory FILE [--current-provider-snapshot sha256:HEX] [--format text|json] [PATH]
    cargo-cultist history [--max-commits N] [--format text|json] FILE
    cargo-cultist ci-tests [--format text|json] [PATH]

COMMANDS:
    check      Inspect live work using the same analyzer and report model as diff.
    diff       Inspect live work against repository evidence and precedent.
    preflight  Compare concurrent change sets for collision evidence.
    history    Explore which paths historically change with one file.
    ci-tests   Compare supported CI test filters with explicit test-name evidence.

Without a command, cargo-cultist inspects repository-wide test-module naming
conventions without inventing a universal rule."#
    );
}

fn change_help(command: &str) -> String {
    debug_assert!(is_change_command(command));
    format!(
        r#"cargo-cultist {command}

USAGE:
    cargo cultist {command} [--base REV] [--format text|json] [PATH]

`check` and `diff` execute the same change analyzer and emit the same report.
By default, the analyzer compares the working tree (including staged changes) against HEAD.
With --base REV, it compares changes from the merge base of REV and HEAD.

Supported evidence currently includes changed Rust test-module precedent and
generated-companion analysis when the repository provides the required evidence."#
    )
}

fn print_change_help(command: &str) {
    println!("{}", change_help(command));
}

fn print_preflight_help() {
    println!(
        r#"cargo-cultist preflight

USAGE:
    cargo cultist preflight --against REV [--format text|json] [PATH]
    cargo cultist preflight --inventory FILE [--current-provider-snapshot sha256:HEX] [--format text|json] [PATH]

With --against, compares current work with REV from their merge base and reports
direct path overlap as PROVEN collision evidence. Current work includes committed
branch changes plus staged and unstaged tracked changes.

With --inventory, admits one bounded local active-change JSON snapshot, compares
its recorded changed paths, and surfaces typed explicit coordination edges as
OBSERVED supplied evidence. When the inventory binds `provider_snapshot_identity`,
`--current-provider-snapshot` supplies the independently derived current population
identity; INVALID or UNKNOWN applicability withholds current-routing collision
findings and asks for a refreshed provider view. Inventory mode performs no
provider/network fetch."#
    );
}

fn print_history_help() {
    println!(
        r#"cargo-cultist history

USAGE:
    cargo cultist history [--max-commits N] [--format text|json] FILE

Explores the most recent non-merge commits touching FILE and reports which
other paths changed in the same considered commits. Revert commits and broad
commits changing more than 100 paths are excluded from the first-pass cohort.

This is research instrumentation for temporal and negative-space precedent.
It reports associations, examples, and counterexamples without turning
co-change frequency into a correctness claim."#
    );
}

fn print_ci_tests_help() {
    println!(
        r#"cargo-cultist ci-tests

USAGE:
    cargo cultist ci-tests [--format text|json] [PATH]

Research instrumentation for CI test-filter drift. The first slice recognizes
literal single-line `cargo [ +TOOLCHAIN ] test --lib FILTER` commands in GitHub Actions and
compares FILTER with explicit #[test] function names plus declared Rust module names.

A zero syntax match remains a question. Macro-generated or build-time tests
are represented as UNKNOWN until authoritative test-listing evidence exists."#
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::finding::{Claim, ClaimKind};

    #[test]
    fn json_writer_emission_matches_serde_string_bytes() {
        let analysis = AnalysisReport {
            schema_version: 1,
            analysis: "diff-precedent".to_string(),
            repository: "/r\u{e9}po".to_string(),
            claims: vec![
                Claim::new(ClaimKind::Derived, "First claim."),
                Claim::new(
                    ClaimKind::Unknown,
                    "Unicode \u{00fc}n\u{ed}code and \"quotes\" survive identically.",
                ),
            ],
            findings: Vec::new(),
        };

        let mut streamed = Vec::new();
        serde_json::to_writer_pretty(&mut streamed, &analysis).unwrap();
        writeln!(streamed).unwrap();

        let expected = format!("{}\n", serde_json::to_string_pretty(&analysis).unwrap());
        assert_eq!(streamed, expected.into_bytes());
    }

    #[test]
    fn recognizes_check_and_diff_as_the_same_change_command() {
        assert!(is_change_command("check"));
        assert!(is_change_command("diff"));
        assert!(!is_change_command("history"));
    }

    #[test]
    fn parses_check_and_diff_args_identically() {
        let args = vec![
            "--base".to_string(),
            "origin/main".to_string(),
            "--format".to_string(),
            "json".to_string(),
            ".".to_string(),
        ];
        let check = parse_diff_args(args.clone(), "check").unwrap();
        let diff = parse_diff_args(args, "diff").unwrap();
        assert_eq!(check, diff);
    }

    #[test]
    fn check_help_names_check_command() {
        let help = change_help("check");
        assert!(help.contains("cargo-cultist check"));
        assert!(help.contains("cargo cultist check"));
        assert!(!help.contains("cargo-cultist diff\n"));
    }

    #[test]
    fn diff_help_names_diff_command() {
        let help = change_help("diff");
        assert!(help.contains("cargo-cultist diff"));
        assert!(help.contains("cargo cultist diff"));
        assert!(!help.contains("cargo-cultist check\n"));
    }

    #[test]
    fn check_parse_errors_name_check_command() {
        let error = parse_diff_args(vec!["--bogus".to_string()], "check").unwrap_err();
        let message = error.to_string();
        assert!(message.contains("unknown check option `--bogus`"));
        assert!(message.contains("cargo cultist check --help"));
    }

    #[test]
    fn diff_parse_errors_name_diff_command() {
        let error = parse_diff_args(vec!["--bogus".to_string()], "diff").unwrap_err();
        let message = error.to_string();
        assert!(message.contains("unknown diff option `--bogus`"));
        assert!(message.contains("cargo cultist diff --help"));
    }

    #[test]
    fn parses_preflight_against_path_and_format() {
        let parsed = parse_preflight_args(vec![
            "--against".to_string(),
            "other-agent".to_string(),
            "--format".to_string(),
            "json".to_string(),
            "src".to_string(),
        ])
        .unwrap();

        assert_eq!(
            parsed.source,
            PreflightSource::Against("other-agent".to_string())
        );
        assert_eq!(parsed.path, Some(PathBuf::from("src")));
        assert_eq!(parsed.format, OutputFormat::Json);
    }

    #[test]
    fn parses_preflight_inventory_path_and_format() {
        let parsed = parse_preflight_args(vec![
            "--inventory".to_string(),
            "active.json".to_string(),
            "--format".to_string(),
            "json".to_string(),
            "src".to_string(),
        ])
        .unwrap();

        assert_eq!(
            parsed.source,
            PreflightSource::Inventory(PathBuf::from("active.json"))
        );
        assert_eq!(parsed.path, Some(PathBuf::from("src")));
        assert_eq!(parsed.format, OutputFormat::Json);
    }

    #[test]
    fn parses_preflight_inventory_current_provider_snapshot() {
        let digest = format!("sha256:{}", "a".repeat(64));
        let parsed = parse_preflight_args(vec![
            "--inventory".to_string(),
            "active.json".to_string(),
            "--current-provider-snapshot".to_string(),
            digest.clone(),
        ])
        .unwrap();

        assert_eq!(
            parsed
                .current_provider_snapshot
                .as_ref()
                .map(ProviderSnapshotIdentity::as_str),
            Some(digest.as_str())
        );
    }

    #[test]
    fn rejects_current_provider_snapshot_with_against_mode() {
        let digest = format!("sha256:{}", "a".repeat(64));
        assert!(
            parse_preflight_args(vec![
                "--against".to_string(),
                "main".to_string(),
                "--current-provider-snapshot".to_string(),
                digest,
            ])
            .is_err()
        );
    }

    #[test]
    fn rejects_malformed_current_provider_snapshot() {
        assert!(
            parse_preflight_args(vec![
                "--inventory".to_string(),
                "active.json".to_string(),
                "--current-provider-snapshot".to_string(),
                "sha256:ABC".to_string(),
            ])
            .is_err()
        );
    }

    #[test]
    fn rejects_preflight_without_source() {
        assert!(parse_preflight_args(vec![]).is_err());
    }

    #[test]
    fn rejects_preflight_with_both_sources() {
        assert!(
            parse_preflight_args(vec![
                "--against".to_string(),
                "other".to_string(),
                "--inventory".to_string(),
                "active.json".to_string(),
            ])
            .is_err()
        );
    }

    #[test]
    fn parses_root_json_format() {
        let (format, path) =
            parse_root_args(vec!["--format".to_string(), "json".to_string()]).unwrap();
        assert_eq!(format, OutputFormat::Json);
        assert_eq!(path, None);
    }

    #[test]
    fn parses_history_path_limit_and_format() {
        let parsed = parse_history_args(vec![
            "--max-commits".to_string(),
            "42".to_string(),
            "--format".to_string(),
            "json".to_string(),
            "src/lib.rs".to_string(),
        ])
        .unwrap();

        assert_eq!(parsed.path, PathBuf::from("src/lib.rs"));
        assert_eq!(parsed.max_commits, 42);
        assert_eq!(parsed.format, OutputFormat::Json);
    }

    #[test]
    fn rejects_missing_history_path() {
        assert!(parse_history_args(vec![]).is_err());
    }

    #[test]
    fn rejects_zero_history_limit() {
        assert!(
            parse_history_args(vec![
                "--max-commits".to_string(),
                "0".to_string(),
                "src/lib.rs".to_string(),
            ])
            .is_err()
        );
    }

    #[test]
    fn rejects_multiple_diff_paths() {
        assert!(parse_diff_args(vec!["a".to_string(), "b".to_string()], "diff").is_err());
    }
}
