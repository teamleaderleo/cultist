#![allow(dead_code)]

#[path = "../src/applicability.rs"]
mod applicability;
#[path = "../src/cmux_review.rs"]
mod cmux_review;
#[path = "../src/cmux_review_applicability.rs"]
mod cmux_review_applicability;
#[path = "../src/finding.rs"]
mod finding;

use applicability::{ApplicabilityStatus, EvaluationContext};
use cmux_review::{CmuxReviewReceipt, CmuxReviewRepair};
use cmux_review_applicability::{
    CmuxReviewApplicabilityRequest, CmuxReviewContinuityDisposition,
    project_cmux_review_for_context,
};
use serde_json::json;

fn receipt() -> CmuxReviewReceipt {
    serde_json::from_value(json!({
        "schema_version": 1,
        "policy_version": "cmux-review/v1",
        "repository_root": "/repo",
        "source": {
            "base_sha": "1111111111111111111111111111111111111111",
            "head_sha": "2222222222222222222222222222222222222222",
            "diff_sha256": "3333333333333333333333333333333333333333333333333333333333333333",
            "working_tree_dirty": false
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
                        "base_sha": "1111111111111111111111111111111111111111",
                        "head_sha": "4444444444444444444444444444444444444444",
                        "diff_sha256": "5555555555555555555555555555555555555555555555555555555555555555",
                        "working_tree_dirty": false
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

fn request(receipt: CmuxReviewReceipt, revision: Option<&str>) -> CmuxReviewApplicabilityRequest {
    CmuxReviewApplicabilityRequest {
        receipt,
        current: EvaluationContext {
            revision: revision.map(str::to_string),
            ..EvaluationContext::default()
        },
    }
}

#[test]
fn exact_review_head_reuses_review() {
    let receipt = receipt();
    let reviewed_head = receipt.source.head_sha.clone();
    let projection =
        project_cmux_review_for_context(&request(receipt, Some(&reviewed_head))).unwrap();

    assert_eq!(projection.applicability.status, ApplicabilityStatus::Applies);
    assert_eq!(
        projection.disposition,
        CmuxReviewContinuityDisposition::ReuseExactReview
    );
}

#[test]
fn repaired_source_requires_a_fresh_review() {
    let receipt = receipt();
    let repaired_head = receipt.findings[0]
        .repair
        .as_ref()
        .and_then(|repair: &CmuxReviewRepair| repair.after_source.as_ref())
        .unwrap()
        .head_sha
        .clone();
    let projection =
        project_cmux_review_for_context(&request(receipt, Some(&repaired_head))).unwrap();

    assert_eq!(projection.applicability.status, ApplicabilityStatus::Invalid);
    assert_eq!(
        projection.disposition,
        CmuxReviewContinuityDisposition::RefreshReview
    );
}

#[test]
fn missing_current_revision_stays_unknown() {
    let projection = project_cmux_review_for_context(&request(receipt(), None)).unwrap();

    assert_eq!(projection.applicability.status, ApplicabilityStatus::Unknown);
    assert_eq!(
        projection.disposition,
        CmuxReviewContinuityDisposition::NeedCurrentRevision
    );
}

#[test]
fn abbreviated_current_revision_is_rejected() {
    let error =
        project_cmux_review_for_context(&request(receipt(), Some("deadbee"))).unwrap_err();

    assert!(
        error
            .to_string()
            .contains("current revision must be an exact 40- or 64-character")
    );
}
