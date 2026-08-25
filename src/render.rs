use std::fmt::Write as _;
use std::io;

use crate::finding::{AnalysisReport, Claim, ClaimKind, Evidence, Finding, Location};

/// Renders the canonical text projection into an owned string.
///
/// Equivalent byte-for-byte to [`write_analysis_report`]; kept for callers
/// that need the rendered report as a value.
#[allow(dead_code)]
pub fn render_analysis_report(report: &AnalysisReport) -> String {
    let mut buffer = Vec::new();
    write_analysis_report(&mut buffer, report).expect("writing into a Vec cannot fail");
    String::from_utf8(buffer).expect("the text renderer only writes UTF-8")
}

/// Streams the canonical text projection straight into `writer` without
/// building an output-sized intermediate string.
pub fn write_analysis_report<W: io::Write>(
    writer: &mut W,
    report: &AnalysisReport,
) -> io::Result<()> {
    writeln!(writer, "{}", analysis_title(&report.analysis))?;
    let mut started = true;

    for claim in &report.claims {
        write_claim(writer, claim, 0, &mut started)?;
    }

    for (index, finding) in report.findings.iter().enumerate() {
        writeln!(writer, "\nFINDING {}: {}", index + 1, finding.title)?;
        write_finding(writer, finding, &mut started)?;
    }

    Ok(())
}

/// Experimental agent-oriented projection over the same canonical AnalysisReport.
///
/// References are deliberately report-local in v0: F1 is the first finding,
/// F1.C2 is its second claim, and C1 is the first report-level claim. They are
/// not durable IDs and must not be persisted as cross-run identity.
///
/// This function is research-only until a public terse format and expansion
/// contract earn promotion.
#[allow(dead_code)]
pub fn render_terse_analysis_report(report: &AnalysisReport) -> String {
    let mut output = String::new();
    let mut emitted = false;

    for (index, claim) in report.claims.iter().enumerate() {
        if claim.kind == ClaimKind::Unknown {
            writeln!(output, "U C{} {}", index + 1, claim.message).unwrap();
            emitted = true;
        }
    }

    for (finding_index, finding) in report.findings.iter().enumerate() {
        let finding_ref = format!("F{}", finding_index + 1);
        write!(output, "{finding_ref} {}", finding.kind).unwrap();
        if let Some(location) = &finding.location {
            write!(output, " @{}", format_location(location)).unwrap();
        }
        output.push('\n');

        for (claim_index, claim) in finding.claims.iter().enumerate() {
            writeln!(
                output,
                "  C{} {} {}",
                claim_index + 1,
                claim_kind_token(claim.kind),
                claim.message
            )
            .unwrap();
        }

        if let Some(question) = &finding.question {
            writeln!(output, "  Q {question}").unwrap();
        }

        emitted = true;
    }

    if !emitted {
        // This is intentionally literal rather than a pass/approval
        // disposition: AnalysisReport currently proves only absence of
        // findings in this projection, not that the change is globally "OK".
        output.push_str("NO_FINDINGS\n");
    }

    output
}

fn write_finding<W: io::Write>(
    writer: &mut W,
    finding: &Finding,
    started: &mut bool,
) -> io::Result<()> {
    if let Some(location) = &finding.location {
        writeln!(writer, "  at {}", format_location(location))?;
    }

    for claim in &finding.claims {
        write_claim(writer, claim, 2, started)?;
    }

    if let Some(question) = &finding.question {
        writeln!(writer, "\nQUESTION")?;
        writeln!(writer, "  {question}")?;
    }

    Ok(())
}

fn write_claim<W: io::Write>(
    writer: &mut W,
    claim: &Claim,
    indent: usize,
    started: &mut bool,
) -> io::Result<()> {
    // Every block ends with exactly one newline, so a separator blank line is
    // required exactly when something has already been written. This mirrors
    // the historical "not empty and not ending on a blank line" rule without
    // inspecting accumulated bytes.
    if *started {
        writer.write_all(b"\n")?;
    }
    *started = true;

    let prefix = " ".repeat(indent);
    writeln!(writer, "{prefix}{}", claim_kind_label(claim.kind))?;
    writeln!(writer, "{prefix}  {}", claim.message)?;

    for evidence in &claim.evidence {
        write_evidence(writer, evidence, indent + 2)?;
    }

    Ok(())
}

fn write_evidence<W: io::Write>(
    writer: &mut W,
    evidence: &Evidence,
    indent: usize,
) -> io::Result<()> {
    let prefix = " ".repeat(indent);
    match &evidence.location {
        Some(location) => writeln!(
            writer,
            "{prefix}- {} ({})",
            evidence.message,
            format_location(location)
        ),
        None => writeln!(writer, "{prefix}- {}", evidence.message),
    }
}

fn format_location(location: &Location) -> String {
    match location.line {
        Some(line) => format!("{}:{line}", location.path),
        None => location.path.clone(),
    }
}

fn claim_kind_label(kind: ClaimKind) -> &'static str {
    match kind {
        ClaimKind::Proven => "PROVEN",
        ClaimKind::Derived => "DERIVED",
        ClaimKind::Observed => "OBSERVED",
        ClaimKind::Inferred => "INFERRED",
        ClaimKind::Unknown => "UNKNOWN",
    }
}

#[allow(dead_code)]
fn claim_kind_token(kind: ClaimKind) -> &'static str {
    match kind {
        ClaimKind::Proven => "P",
        ClaimKind::Derived => "D",
        ClaimKind::Observed => "O",
        ClaimKind::Inferred => "I",
        ClaimKind::Unknown => "?",
    }
}

fn analysis_title(analysis: &str) -> String {
    analysis
        .split(['-', '_'])
        .filter(|part| !part.is_empty())
        .map(str::to_ascii_uppercase)
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::finding::{Evidence, Finding};

    fn sample_report(claims: Vec<Claim>, findings: Vec<Finding>) -> AnalysisReport {
        AnalysisReport {
            schema_version: 1,
            analysis: "diff-precedent".to_string(),
            repository: "/r\u{e9}po".to_string(),
            claims,
            findings,
        }
    }

    #[test]
    fn streamed_output_is_byte_identical_to_rendered_string() {
        let fixtures = vec![
            sample_report(Vec::new(), Vec::new()),
            sample_report(
                vec![
                    Claim::new(ClaimKind::Derived, "First derived claim."),
                    Claim::new(
                        ClaimKind::Unknown,
                        "Multi-line\nmessage with \u{00fc}n\u{ed}code and trailing spaces.  ",
                    ),
                ],
                Vec::new(),
            ),
            sample_report(
                Vec::new(),
                vec![
                    Finding::new("kind-a", "Finding without claims")
                        .at(Location::new("src/a.rs", Some(7))),
                    Finding::new("kind-b", "Finding with evidence")
                        .at(Location::new("src/b.rs", None))
                        .with_claim(
                            Claim::new(ClaimKind::Observed, "Observed tension.").with_evidence(
                                Evidence::at(
                                    "Local precedent.",
                                    Location::new("src/b.rs", Some(3)),
                                ),
                            ),
                        )
                        .with_claim(Claim::new(ClaimKind::Inferred, "Inferred scope."))
                        .with_question("Which precedent governs?"),
                ],
            ),
        ];

        for report in &fixtures {
            let mut streamed = Vec::new();
            write_analysis_report(&mut streamed, report).unwrap();
            assert_eq!(streamed, render_analysis_report(report).into_bytes());
        }
    }

    #[test]
    fn renders_provenance_and_evidence() {
        let report = AnalysisReport {
            schema_version: 1,
            analysis: "diff-precedent".to_string(),
            repository: "/repo".to_string(),
            claims: vec![Claim::new(
                ClaimKind::Derived,
                "The diff changes one Rust file.",
            )],
            findings: vec![
                Finding::new("example", "Example tension")
                    .at(Location::new("src/lib.rs", Some(42)))
                    .with_claim(
                        Claim::new(ClaimKind::Observed, "Two scopes disagree.").with_evidence(
                            Evidence::at(
                                "The file already uses `tests`.",
                                Location::new("src/lib.rs", Some(10)),
                            ),
                        ),
                    )
                    .with_claim(Claim::new(
                        ClaimKind::Unknown,
                        "The repository does not state which scope wins.",
                    ))
                    .with_question("Which scope is intentional here?"),
            ],
        };

        let rendered = render_analysis_report(&report);
        assert!(rendered.contains("DIFF PRECEDENT"));
        assert!(rendered.contains("DERIVED"));
        assert!(rendered.contains("OBSERVED"));
        assert!(rendered.contains("UNKNOWN"));
        assert!(rendered.contains("src/lib.rs:42"));
        assert!(rendered.contains("QUESTION"));
    }

    #[test]
    fn terse_renders_clean_report_as_literal_no_findings() {
        let report = AnalysisReport {
            schema_version: 1,
            analysis: "diff-precedent".to_string(),
            repository: "/repo".to_string(),
            claims: vec![Claim::new(ClaimKind::Derived, "One file changed.")],
            findings: Vec::new(),
        };

        assert_eq!(render_terse_analysis_report(&report), "NO_FINDINGS\n");
    }

    #[test]
    fn terse_does_not_turn_claim_only_report_into_ok_disposition() {
        let report = AnalysisReport {
            schema_version: 1,
            analysis: "example".to_string(),
            repository: "/repo".to_string(),
            claims: vec![Claim::new(
                ClaimKind::Inferred,
                "This may be routine, but the canonical report does not approve it.",
            )],
            findings: Vec::new(),
        };

        let rendered = render_terse_analysis_report(&report);
        assert_eq!(rendered, "NO_FINDINGS\n");
        assert!(!rendered.contains("OK"));
    }

    #[test]
    fn terse_keeps_report_level_unknown_visible() {
        let report = AnalysisReport {
            schema_version: 1,
            analysis: "example".to_string(),
            repository: "/repo".to_string(),
            claims: vec![Claim::new(
                ClaimKind::Unknown,
                "Target-native execution has not run.",
            )],
            findings: Vec::new(),
        };

        assert_eq!(
            render_terse_analysis_report(&report),
            "U C1 Target-native execution has not run.\n"
        );
    }

    #[test]
    fn terse_uses_report_local_refs_and_omits_evidence_prose() {
        let report = AnalysisReport {
            schema_version: 1,
            analysis: "diff-precedent".to_string(),
            repository: "/repo".to_string(),
            claims: Vec::new(),
            findings: vec![
                Finding::new("precedent-tension", "Example tension")
                    .at(Location::new("src/lib.rs", Some(42)))
                    .with_claim(
                        Claim::new(ClaimKind::Observed, "Two scopes disagree.").with_evidence(
                            Evidence::at(
                                "Verbose supporting evidence should stay expandable.",
                                Location::new("src/lib.rs", Some(10)),
                            ),
                        ),
                    )
                    .with_claim(Claim::new(
                        ClaimKind::Unknown,
                        "The repository does not state which scope wins.",
                    ))
                    .with_question("Which scope is intentional here?"),
            ],
        };

        let rendered = render_terse_analysis_report(&report);
        assert_eq!(
            rendered,
            "F1 precedent-tension @src/lib.rs:42\n  C1 O Two scopes disagree.\n  C2 ? The repository does not state which scope wins.\n  Q Which scope is intentional here?\n"
        );
        assert!(!rendered.contains("Verbose supporting evidence"));
    }

    #[test]
    fn labels_every_claim_kind() {
        assert_eq!(claim_kind_label(ClaimKind::Proven), "PROVEN");
        assert_eq!(claim_kind_label(ClaimKind::Derived), "DERIVED");
        assert_eq!(claim_kind_label(ClaimKind::Observed), "OBSERVED");
        assert_eq!(claim_kind_label(ClaimKind::Inferred), "INFERRED");
        assert_eq!(claim_kind_label(ClaimKind::Unknown), "UNKNOWN");
    }

    #[test]
    fn tokens_every_claim_kind() {
        assert_eq!(claim_kind_token(ClaimKind::Proven), "P");
        assert_eq!(claim_kind_token(ClaimKind::Derived), "D");
        assert_eq!(claim_kind_token(ClaimKind::Observed), "O");
        assert_eq!(claim_kind_token(ClaimKind::Inferred), "I");
        assert_eq!(claim_kind_token(ClaimKind::Unknown), "?");
    }
}
