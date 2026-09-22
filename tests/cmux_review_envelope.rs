#![allow(dead_code)]

#[path = "../src/cmux_review.rs"]
mod cmux_review;
#[path = "../src/finding.rs"]
mod finding;

use cmux_review::*;
use finding::ClaimKind;
use serde_json::json;

fn sample_receipt() -> CmuxReviewReceipt {
    serde_json::from_value(json!({
        "schema_version": 1,
        "policy_version": "cmux-review/v1",
        "repository_root": "/repo",
        "ruleset_sha256": "4444444444444444444444444444444444444444444444444444444444444444",
        "source": {
            "base_sha": "1111111111111111111111111111111111111111",
            "head_sha": "2222222222222222222222222222222222222222",
            "diff_sha256": "3333333333333333333333333333333333333333333333333333333333333333",
            "working_tree_dirty": false
        },
        "brief": {
            "intent": "Fix reconnect behavior",
            "requirements": [
                {
                    "requirement": "Reconnect after restart",
                    "status": "satisfied",
                    "evidence": ["targeted integration test"]
                },
                {
                    "requirement": "Preserve authorization",
                    "status": "uncertain",
                    "evidence": []
                }
            ],
            "out_of_scope_changes": [],
            "behavior_changed": ["Reconnect ownership moved"],
            "risk_areas": [
                {"level": "high", "area": "session lifecycle", "reason": "ownership changed"}
            ],
            "file_groups": [
                {"label": "reconnect", "paths": ["src/reconnect.rs"]}
            ],
            "reading_order": ["src/reconnect.rs"],
            "safeguards": ["retry cancellation test"],
            "coverage_gaps": ["daemon restart during retry"]
        },
        "summary": {
            "hypotheses_investigated": 3,
            "suppressed": 1,
            "refuted": 0,
            "verified": 1,
            "human_judgment": 1
        },
        "findings": [
            {
                "id": "RACE-01",
                "title": "Concurrent reconnect mutation",
                "severity": "P1",
                "claims": [
                    {
                        "kind": "inferred",
                        "message": "Reconnect mutation may race during teardown.",
                        "evidence": []
                    },
                    {
                        "kind": "proven",
                        "message": "The targeted reproducer reaches concurrent mutation.",
                        "evidence": [
                            {
                                "kind": "reproduction",
                                "summary": "reproduced 10/10",
                                "command": "cargo test reconnect_race"
                            }
                        ]
                    }
                ],
                "failure_mode": "Concurrent state mutation",
                "paths": ["src/reconnect.rs"],
                "discovery_sources": ["correctness"],
                "challenge": {
                    "disposition": "survives_challenge",
                    "evidence": []
                },
                "verification": {
                    "result": "reproduced",
                    "evidence": [
                        {
                            "kind": "test",
                            "summary": "focused reproducer failed before repair"
                        }
                    ]
                },
                "repair": {
                    "attempted": true,
                    "result": "fixed",
                    "after_source": {
                        "base_sha": "1111111111111111111111111111111111111111",
                        "head_sha": "5555555555555555555555555555555555555555",
                        "diff_sha256": "6666666666666666666666666666666666666666666666666666666666666666",
                        "working_tree_dirty": false
                    },
                    "verification": {
                        "result": "passed",
                        "evidence": [
                            {
                                "kind": "test",
                                "summary": "the original discriminator passes after repair"
                            }
                        ]
                    },
                    "notes": "serialized ownership transition"
                },
                "disposition": "repaired"
            },
            {
                "id": "AUTH-02",
                "title": "Authorization intent remains ambiguous",
                "severity": "P1",
                "claims": [
                    {
                        "kind": "unknown",
                        "message": "The task does not establish whether the new route preserves the old authorization boundary.",
                        "evidence": []
                    }
                ],
                "failure_mode": "Potential authorization widening",
                "paths": ["src/auth.rs"],
                "discovery_sources": ["intent"],
                "challenge": {
                    "disposition": "uncertain",
                    "evidence": []
                },
                "verification": {
                    "result": "human_judgment",
                    "evidence": []
                },
                "repair": null,
                "disposition": "human_required"
            },
            {
                "id": "STYLE-03",
                "title": "Naming nit",
                "severity": "P3",
                "claims": [
                    {
                        "kind": "inferred",
                        "message": "A helper name could be shorter.",
                        "evidence": []
                    }
                ],
                "failure_mode": "No runtime failure",
                "paths": ["src/reconnect.rs"],
                "discovery_sources": ["style"],
                "challenge": {
                    "disposition": "survives_challenge",
                    "evidence": []
                },
                "verification": {
                    "result": "not_reproduced",
                    "evidence": []
                },
                "repair": null,
                "disposition": "suppressed"
            }
        ],
        "created_at": "2026-09-22T00:00:00Z"
    }))
    .unwrap()
}

#[test]
fn projects_attention_frontier_and_quiet_findings() {
    let receipt = sample_receipt();
    let envelope = project_cmux_review_receipt(&receipt).unwrap();

    assert_eq!(envelope.requirements.satisfied, 1);
    assert_eq!(envelope.requirements.uncertain, 1);
    assert_eq!(
        envelope
            .attention
            .iter()
            .map(|finding| finding.id.as_str())
            .collect::<Vec<_>>(),
        vec!["RACE-01", "AUTH-02"]
    );
    assert_eq!(
        envelope
            .quiet
            .iter()
            .map(|finding| finding.id.as_str())
            .collect::<Vec<_>>(),
        vec!["STYLE-03"]
    );
    assert!(envelope.frontier.iter().any(|item| {
        item.finding_id == "AUTH-02" && item.kind == CmuxReviewFrontierKind::UnknownClaim
    }));
    assert!(envelope.frontier.iter().any(|item| {
        item.finding_id == "AUTH-02" && item.kind == CmuxReviewFrontierKind::HumanRequired
    }));
    assert_eq!(envelope.attention[0].claims[0].kind, ClaimKind::Inferred);
    assert_eq!(envelope.attention[0].claims[1].kind, ClaimKind::Proven);
    assert_eq!(
        envelope.attention[0]
            .repair
            .as_ref()
            .and_then(|repair| repair.after_source.as_ref())
            .map(|source| source.head_sha.as_str()),
        Some("5555555555555555555555555555555555555555")
    );
}

#[test]
fn rejects_repaired_finding_without_post_repair_verification() {
    let mut receipt = sample_receipt();
    receipt.findings[0].repair.as_mut().unwrap().verification = None;

    let error = project_cmux_review_receipt(&receipt).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("without post-repair verification")
    );
}

#[test]
fn rejects_repaired_finding_with_unchanged_resulting_source() {
    let mut receipt = sample_receipt();
    receipt.findings[0].repair.as_mut().unwrap().after_source = Some(receipt.source.clone());

    let error = project_cmux_review_receipt(&receipt).unwrap_err();
    assert!(error.to_string().contains("resulting source is unchanged"));
}

#[test]
fn rejects_supported_verification_without_proven_or_derived_claim() {
    let mut receipt = sample_receipt();
    receipt.findings[0]
        .claims
        .retain(|claim| claim.kind == ClaimKind::Inferred);

    let error = project_cmux_review_receipt(&receipt).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("without a proven or derived claim")
    );
}

#[test]
fn rejects_refuted_summary_drift() {
    let mut receipt = sample_receipt();
    receipt.summary.refuted = 1;

    let error = project_cmux_review_receipt(&receipt).unwrap_err();
    assert!(error.to_string().contains("summary.refuted=1"));
}

#[test]
fn rejects_abbreviated_source_revisions() {
    let mut receipt = sample_receipt();
    receipt.source.head_sha = "deadbee".to_string();

    let error = project_cmux_review_receipt(&receipt).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("exact 40- or 64-character lowercase Git object id")
    );
}
