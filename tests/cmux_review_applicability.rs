#![allow(dead_code)]

#[path = "../src/applicability.rs"]
mod applicability;
#[path = "../src/cmux_review.rs"]
mod cmux_review;
#[path = "../src/cmux_review_applicability.rs"]
mod cmux_review_applicability;
#[path = "../src/finding.rs"]
mod finding;

use applicability::ApplicabilityStatus;
use cmux_review::{CmuxReviewReceipt, CmuxReviewSource};
use cmux_review_applicability::{
    CmuxReviewApplicabilityRequest, CmuxReviewContinuityDisposition, CmuxReviewCoordinateStatus,
    CmuxReviewCurrentContext, fingerprint_source, project_cmux_review_for_context,
};
use serde_json::json;

const RULESET_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const RULESET_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

fn receipt() -> CmuxReviewReceipt {
    serde_json::from_value(json!({
        "schema_version": 1,
        "policy_version": "cmux-review/v1",
        "repository_root": "/repo",
        "ruleset_sha256": RULESET_A,
        "source": {
            "repository_id": "github:owner/repo",
            "base_sha": "1111111111111111111111111111111111111111",
            "head_sha": "2222222222222222222222222222222222222222",
            "tree_sha": "3333333333333333333333333333333333333333",
            "working_tree_dirty": true,
            "patch_sha256": null
        },
        "brief": {
            "intent": "Review one repairable change",
            "requirements": [],
            "out_of_scope_changes": [],
            "behavior_changed": [],
            "risk_areas": [],
            "file_groups": [],
            "reading_order": [],
            "safeguards": [],
            "coverage_gaps": []
        },
        "summary": {
            "hypotheses_investigated": 1,
            "suppressed": 0,
            "refuted": 0,
            "verified": 1,
            "human_judgment": 0
        },
        "findings": [
            {
                "id": "BUG-01",
                "title": "Repairable defect",
                "severity": "P1",
                "claims": [
                    {
                        "kind": "inferred",
                        "message": "The changed path may fail.",
                        "evidence": []
                    },
                    {
                        "kind": "proven",
                        "message": "The focused test fails on the reviewed source.",
                        "evidence": [
                            {"kind": "test", "summary": "fails before repair"}
                        ]
                    }
                ],
                "failure_mode": "Focused failure",
                "paths": ["src/lib.rs"],
                "discovery_sources": ["correctness"],
                "challenge": {
                    "disposition": "survives_challenge",
                    "evidence": []
                },
                "verification": {
                    "result": "reproduced",
                    "evidence": [
                        {"kind": "test", "summary": "fails before repair"}
                    ]
                },
                "repair": {
                    "attempted": true,
                    "result": "fixed",
                    "after_source": {
                        "repository_id": "github:owner/repo",
                        "base_sha": "1111111111111111111111111111111111111111",
                        "head_sha": "2222222222222222222222222222222222222222",
                        "tree_sha": "4444444444444444444444444444444444444444",
                        "working_tree_dirty": true,
                        "patch_sha256": null
                    },
                    "verification": {
                        "result": "passed",
                        "evidence": [
                            {"kind": "test", "summary": "passes after repair"}
                        ]
                    }
                },
                "disposition": "repaired"
            }
        ],
        "created_at": "2026-09-22T00:00:00Z"
    }))
    .unwrap()
}

fn current(
    source: Option<CmuxReviewSource>,
    policy_version: &str,
    ruleset_sha256: Option<&str>,
) -> CmuxReviewCurrentContext {
    CmuxReviewCurrentContext {
        source,
        policy_version: policy_version.to_string(),
        ruleset_sha256: ruleset_sha256.map(str::to_string),
    }
}

fn request(
    receipt: CmuxReviewReceipt,
    current: CmuxReviewCurrentContext,
) -> CmuxReviewApplicabilityRequest {
    CmuxReviewApplicabilityRequest { receipt, current }
}

#[test]
fn exact_review_source_and_policy_reuse_review() {
    let receipt = receipt();
    let context = current(
        Some(receipt.source.clone()),
        &receipt.policy_version,
        receipt.ruleset_sha256.as_deref(),
    );
    let projection = project_cmux_review_for_context(&request(receipt, context)).unwrap();

    assert_eq!(
        projection.source_applicability.status,
        ApplicabilityStatus::Applies
    );
    assert_eq!(
        projection.policy_version_status,
        CmuxReviewCoordinateStatus::Matched
    );
    assert_eq!(
        projection.ruleset_status,
        CmuxReviewCoordinateStatus::Matched
    );
    assert_eq!(
        projection.disposition,
        CmuxReviewContinuityDisposition::ReuseExactReview
    );
}

#[test]
fn same_head_with_different_candidate_tree_requires_refresh() {
    let receipt = receipt();
    let mut changed = receipt.source.clone();
    changed.tree_sha =
        "9999999999999999999999999999999999999999".to_string();

    let projection = project_cmux_review_for_context(&request(
        receipt,
        current(Some(changed), "cmux-review/v1", Some(RULESET_A)),
    ))
    .unwrap();

    assert_eq!(
        projection.source_applicability.status,
        ApplicabilityStatus::Invalid
    );
    assert_eq!(
        projection.disposition,
        CmuxReviewContinuityDisposition::RefreshReview
    );
}

#[test]
fn repaired_source_requires_a_fresh_review() {
    let receipt = receipt();
    let repaired_source = receipt.findings[0]
        .repair
        .as_ref()
        .and_then(|repair| repair.after_source.clone())
        .unwrap();

    let projection = project_cmux_review_for_context(&request(
        receipt,
        current(Some(repaired_source), "cmux-review/v1", Some(RULESET_A)),
    ))
    .unwrap();

    assert_eq!(
        projection.source_applicability.status,
        ApplicabilityStatus::Invalid
    );
    assert_eq!(
        projection.disposition,
        CmuxReviewContinuityDisposition::RefreshReview
    );
}

#[test]
fn policy_version_change_requires_refresh() {
    let receipt = receipt();
    let projection = project_cmux_review_for_context(&request(
        receipt.clone(),
        current(
            Some(receipt.source.clone()),
            "cmux-review/v2",
            Some(RULESET_A),
        ),
    ))
    .unwrap();

    assert_eq!(
        projection.source_applicability.status,
        ApplicabilityStatus::Applies
    );
    assert_eq!(
        projection.policy_version_status,
        CmuxReviewCoordinateStatus::Mismatched
    );
    assert_eq!(
        projection.disposition,
        CmuxReviewContinuityDisposition::RefreshReview
    );
}

#[test]
fn ruleset_change_requires_refresh() {
    let receipt = receipt();
    let projection = project_cmux_review_for_context(&request(
        receipt.clone(),
        current(
            Some(receipt.source.clone()),
            &receipt.policy_version,
            Some(RULESET_B),
        ),
    ))
    .unwrap();

    assert_eq!(
        projection.source_applicability.status,
        ApplicabilityStatus::Applies
    );
    assert_eq!(
        projection.ruleset_status,
        CmuxReviewCoordinateStatus::Mismatched
    );
    assert_eq!(
        projection.disposition,
        CmuxReviewContinuityDisposition::RefreshReview
    );
}

#[test]
fn missing_current_source_stays_unknown() {
    let projection = project_cmux_review_for_context(&request(
        receipt(),
        current(None, "cmux-review/v1", Some(RULESET_A)),
    ))
    .unwrap();

    assert_eq!(
        projection.source_applicability.status,
        ApplicabilityStatus::Unknown
    );
    assert_eq!(
        projection.disposition,
        CmuxReviewContinuityDisposition::NeedCurrentSource
    );
}

#[test]
fn source_fingerprint_changes_with_repository_identity() {
    let receipt = receipt();
    let mut changed = receipt.source.clone();
    changed.repository_id = "github:other/repo".to_string();

    assert_ne!(
        fingerprint_source(&receipt.source).unwrap(),
        fingerprint_source(&changed).unwrap()
    );
}

#[test]
fn source_fingerprint_changes_with_candidate_tree() {
    let receipt = receipt();
    let mut changed = receipt.source.clone();
    changed.tree_sha =
        "9999999999999999999999999999999999999999".to_string();

    assert_ne!(
        fingerprint_source(&receipt.source).unwrap(),
        fingerprint_source(&changed).unwrap()
    );
}

#[test]
fn identity_only_head_change_reuses_review() {
    let receipt = receipt();
    let mut changed = receipt.source.clone();
    changed.head_sha =
        "8888888888888888888888888888888888888888".to_string();

    assert_eq!(
        fingerprint_source(&receipt.source).unwrap(),
        fingerprint_source(&changed).unwrap()
    );

    let projection = project_cmux_review_for_context(&request(
        receipt,
        current(Some(changed), "cmux-review/v1", Some(RULESET_A)),
    ))
    .unwrap();

    assert_eq!(
        projection.disposition,
        CmuxReviewContinuityDisposition::ReuseExactReview
    );
}

#[test]
fn patch_digest_is_audit_evidence_not_reuse_identity() {
    let receipt = receipt();
    let mut changed = receipt.source.clone();
    changed.patch_sha256 =
        Some("7777777777777777777777777777777777777777777777777777777777777777".to_string());

    assert_eq!(
        fingerprint_source(&receipt.source).unwrap(),
        fingerprint_source(&changed).unwrap()
    );
}
