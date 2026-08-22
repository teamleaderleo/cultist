#![allow(dead_code)]

#[path = "../src/compact_ir.rs"]
mod compact_ir;
#[path = "../src/finding.rs"]
mod finding;
#[path = "../src/report_fingerprint.rs"]
mod report_fingerprint;

use finding::{
    AnalysisReport, Claim, ClaimKind, Evidence, Finding, Location, REPORT_SCHEMA_VERSION,
};
use report_fingerprint::{ReportFingerprint, fingerprint_report};

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum SnapshotMutation {
    ReorderFindings,
    ChangeClaimSemantics,
    PrettyJsonRoundTrip,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum MutationVerdict {
    Survived,
    Killed,
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct SnapshotMutationReceipt {
    mutation: SnapshotMutation,
    before: ReportFingerprint,
    after: ReportFingerprint,
    verdict: MutationVerdict,
}

fn representative_report() -> AnalysisReport {
    AnalysisReport {
        schema_version: REPORT_SCHEMA_VERSION,
        analysis: "mutation-report-identity".to_string(),
        repository: "/repo".to_string(),
        claims: vec![Claim::new(
            ClaimKind::Unknown,
            "semantic independence is unresolved",
        )],
        findings: vec![
            Finding::new("direct-overlap", "Direct path overlap")
                .at(Location::new("src/auth.rs", Some(12)))
                .with_claim(
                    Claim::new(ClaimKind::Proven, "both work items modify src/auth.rs")
                        .with_evidence(Evidence::new("github:pull/10")),
                )
                .with_question("Coordinate ownership?"),
            Finding::new("explicit-coordination", "Explicit coordination").with_claim(Claim::new(
                ClaimKind::Observed,
                "hold_merge_while #10 > #11",
            )),
        ],
    }
}

fn mutate(report: &AnalysisReport, mutation: SnapshotMutation) -> AnalysisReport {
    match mutation {
        SnapshotMutation::ReorderFindings => {
            let mut mutated = report.clone();
            mutated.findings.reverse();
            mutated
        }
        SnapshotMutation::ChangeClaimSemantics => {
            let mut mutated = report.clone();
            mutated.findings[0].claims[0].message.push('!');
            mutated
        }
        SnapshotMutation::PrettyJsonRoundTrip => {
            let pretty = serde_json::to_string_pretty(report).unwrap();
            serde_json::from_str(&pretty).unwrap()
        }
    }
}

fn mutation_receipt(mutation: SnapshotMutation) -> SnapshotMutationReceipt {
    let report = representative_report();
    let before = fingerprint_report(&report).unwrap();
    let after = fingerprint_report(&mutate(&report, mutation)).unwrap();
    let verdict = if before == after {
        MutationVerdict::Survived
    } else {
        MutationVerdict::Killed
    };
    SnapshotMutationReceipt {
        mutation,
        before,
        after,
        verdict,
    }
}

#[test]
fn finding_reorder_kills_exact_snapshot_identity_mutation() {
    let receipt = mutation_receipt(SnapshotMutation::ReorderFindings);
    assert_eq!(receipt.mutation, SnapshotMutation::ReorderFindings);
    assert_eq!(receipt.verdict, MutationVerdict::Killed);
    assert_ne!(receipt.before, receipt.after);
}

#[test]
fn claim_semantic_change_kills_exact_snapshot_identity_mutation() {
    let receipt = mutation_receipt(SnapshotMutation::ChangeClaimSemantics);
    assert_eq!(receipt.mutation, SnapshotMutation::ChangeClaimSemantics);
    assert_eq!(receipt.verdict, MutationVerdict::Killed);
    assert_ne!(receipt.before, receipt.after);
}

#[test]
fn json_representation_change_survives_typed_snapshot_identity_oracle() {
    let receipt = mutation_receipt(SnapshotMutation::PrettyJsonRoundTrip);
    assert_eq!(receipt.mutation, SnapshotMutation::PrettyJsonRoundTrip);
    assert_eq!(receipt.verdict, MutationVerdict::Survived);
    assert_eq!(receipt.before, receipt.after);
}
