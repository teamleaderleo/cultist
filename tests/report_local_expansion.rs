#[allow(dead_code)]
#[path = "../src/compact_ir.rs"]
mod compact_ir;
#[allow(dead_code)]
#[path = "../src/finding.rs"]
mod finding;
#[allow(dead_code)]
#[path = "../src/render.rs"]
mod render;
#[allow(dead_code)]
#[path = "../src/report_fingerprint.rs"]
mod report_fingerprint;

use finding::{
    AnalysisReport, Claim, ClaimKind, Evidence, Finding, Location, REPORT_SCHEMA_VERSION,
};
use render::render_terse_analysis_report;
use report_fingerprint::{ReportFingerprint, fingerprint_report};

#[derive(Debug)]
enum Expansion<'a> {
    Claim(&'a Claim),
    Finding(&'a Finding),
    Evidence(&'a Evidence),
    Question(&'a str),
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum ResolveError {
    BaseMismatch,
    InvalidReference,
    NotFound,
}

fn resolve<'a>(
    report: &'a AnalysisReport,
    expected_report: &ReportFingerprint,
    reference: &str,
) -> Result<Expansion<'a>, ResolveError> {
    let current = fingerprint_report(report).map_err(|_| ResolveError::BaseMismatch)?;
    if &current != expected_report {
        return Err(ResolveError::BaseMismatch);
    }

    if let Some(index) = one_based_suffix(reference, 'C') {
        return report
            .claims
            .get(index)
            .map(Expansion::Claim)
            .ok_or(ResolveError::NotFound);
    }

    let Some(rest) = reference.strip_prefix('F') else {
        return Err(ResolveError::InvalidReference);
    };
    let (finding_text, tail) = rest.split_once('.').unwrap_or((rest, ""));
    let finding_index = one_based_index(finding_text)?;
    let finding = report
        .findings
        .get(finding_index)
        .ok_or(ResolveError::NotFound)?;

    if tail.is_empty() {
        return Ok(Expansion::Finding(finding));
    }
    if tail == "Q" {
        return finding
            .question
            .as_deref()
            .map(Expansion::Question)
            .ok_or(ResolveError::NotFound);
    }

    let Some(claim_tail) = tail.strip_prefix('C') else {
        return Err(ResolveError::InvalidReference);
    };
    let (claim_text, evidence_tail) = claim_tail.split_once('.').unwrap_or((claim_tail, ""));
    let claim_index = one_based_index(claim_text)?;
    let claim = finding
        .claims
        .get(claim_index)
        .ok_or(ResolveError::NotFound)?;

    if evidence_tail.is_empty() {
        return Ok(Expansion::Claim(claim));
    }

    let Some(evidence_text) = evidence_tail.strip_prefix('E') else {
        return Err(ResolveError::InvalidReference);
    };
    let evidence_index = one_based_index(evidence_text)?;
    claim
        .evidence
        .get(evidence_index)
        .map(Expansion::Evidence)
        .ok_or(ResolveError::NotFound)
}

fn one_based_suffix(reference: &str, prefix: char) -> Option<usize> {
    let rest = reference.strip_prefix(prefix)?;
    if rest.contains('.') {
        return None;
    }
    one_based_index(rest).ok()
}

fn one_based_index(value: &str) -> Result<usize, ResolveError> {
    let value = value
        .parse::<usize>()
        .map_err(|_| ResolveError::InvalidReference)?;
    value.checked_sub(1).ok_or(ResolveError::InvalidReference)
}

fn sample_report() -> AnalysisReport {
    AnalysisReport {
        schema_version: REPORT_SCHEMA_VERSION,
        analysis: "report-local-expansion".to_string(),
        repository: "/repo".to_string(),
        claims: vec![Claim::new(
            ClaimKind::Unknown,
            "provider evidence is unavailable",
        )],
        findings: vec![
            Finding::new("preflight-overlap", "Concurrent path overlap")
                .at(Location::new("src/auth.rs", Some(42)))
                .with_claim(
                    Claim::new(ClaimKind::Proven, "both work items modify src/auth.rs")
                        .with_evidence(Evidence::new(
                            "current work changes src/auth.rs at exact head aaa111",
                        ))
                        .with_evidence(Evidence::new(
                            "other work changes src/auth.rs at exact head bbb222",
                        )),
                )
                .with_question("Coordinate ownership before continuing?"),
        ],
    }
}

#[test]
fn terse_can_omit_support_while_fingerprint_bound_expansion_recovers_it() {
    let report = sample_report();
    let fingerprint = fingerprint_report(&report).unwrap();
    let terse = render_terse_analysis_report(&report);

    assert!(terse.contains("F1 preflight-overlap @src/auth.rs:42"));
    assert!(terse.contains("C1 P both work items modify src/auth.rs"));
    assert!(!terse.contains("exact head aaa111"));
    assert!(!terse.contains("exact head bbb222"));

    let Expansion::Claim(claim) = resolve(&report, &fingerprint, "F1.C1").unwrap() else {
        panic!("expected claim expansion");
    };
    assert_eq!(claim.evidence.len(), 2);
    assert_eq!(
        claim.evidence[0].message,
        "current work changes src/auth.rs at exact head aaa111"
    );

    let Expansion::Evidence(evidence) = resolve(&report, &fingerprint, "F1.C1.E2").unwrap() else {
        panic!("expected evidence expansion");
    };
    assert_eq!(
        evidence.message,
        "other work changes src/auth.rs at exact head bbb222"
    );
}

#[test]
fn report_level_claim_finding_and_question_are_expandable_from_the_same_snapshot() {
    let report = sample_report();
    let fingerprint = fingerprint_report(&report).unwrap();

    let Expansion::Claim(claim) = resolve(&report, &fingerprint, "C1").unwrap() else {
        panic!("expected top-level claim");
    };
    assert_eq!(claim.kind, ClaimKind::Unknown);

    let Expansion::Finding(finding) = resolve(&report, &fingerprint, "F1").unwrap() else {
        panic!("expected finding");
    };
    assert_eq!(finding.kind, "preflight-overlap");
    assert_eq!(finding.title, "Concurrent path overlap");

    let Expansion::Question(question) = resolve(&report, &fingerprint, "F1.Q").unwrap() else {
        panic!("expected question");
    };
    assert_eq!(question, "Coordinate ownership before continuing?");
}

#[test]
fn stale_report_fingerprint_rejects_a_previously_valid_local_reference() {
    let mut report = sample_report();
    let old_fingerprint = fingerprint_report(&report).unwrap();

    report.findings[0].claims[0].message =
        "both work items modify the authorization surface".to_string();

    assert!(matches!(
        resolve(&report, &old_fingerprint, "F1.C1"),
        Err(ResolveError::BaseMismatch)
    ));
}

#[test]
fn reordered_snapshot_rejects_old_fingerprint_before_resolving_position() {
    let mut report = sample_report();
    report.findings.push(
        Finding::new("other", "Other finding")
            .with_claim(Claim::new(ClaimKind::Observed, "other observation")),
    );
    let old_fingerprint = fingerprint_report(&report).unwrap();

    report.findings.swap(0, 1);

    assert!(matches!(
        resolve(&report, &old_fingerprint, "F1"),
        Err(ResolveError::BaseMismatch)
    ));
}

#[test]
fn malformed_and_missing_references_fail_explicitly() {
    let report = sample_report();
    let fingerprint = fingerprint_report(&report).unwrap();

    assert!(matches!(
        resolve(&report, &fingerprint, "F0"),
        Err(ResolveError::InvalidReference)
    ));
    assert!(matches!(
        resolve(&report, &fingerprint, "F2"),
        Err(ResolveError::NotFound)
    ));
    assert!(matches!(
        resolve(&report, &fingerprint, "F1.X1"),
        Err(ResolveError::InvalidReference)
    ));
    assert!(matches!(
        resolve(&report, &fingerprint, "F1.C1.E3"),
        Err(ResolveError::NotFound)
    ));
}
