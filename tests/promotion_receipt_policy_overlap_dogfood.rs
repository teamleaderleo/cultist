#[allow(dead_code)]
#[path = "../src/promotion_change_set.rs"]
mod promotion_change_set;
#[allow(dead_code)]
#[path = "../src/promotion_receipt.rs"]
mod promotion_receipt;

use promotion_change_set::{PromotionChangeSetEntry, fingerprint_promotion_change_set};
use promotion_receipt::{
    CurrentPromotionState, InterveningCommit, PROMOTION_RECEIPT_SCHEMA_VERSION,
    PromotionPathOverlapKind, PromotionReceiptDisposition, PromotionReceiptReason,
    PromotionReceiptRequest, TestedPromotionState, evaluate_promotion_receipt,
};

const OLD_HEAD: &str = "12fc7ea0b031e7128d0d72cc861f2d1ad78b4e30";
const OLD_TREE: &str = "1da2cc89d48b81bf3b9e10696651de9a4006e1d7";
const OLD_BASE: &str = "c84e151714d3d9a53a0723dbbf384e0df7e12242";
const OLD_BASE_TREE: &str = "28e49529d627463c8fa2eef550c1970cede5c1ac";
const OLD_MERGE_TREE: &str = "1da2cc89d48b81bf3b9e10696651de9a4006e1d7";

const CURRENT_HEAD: &str = "18c204c39c38f60e43d25e9ddc2a325e07ac7f0a";
const CURRENT_TREE: &str = "d3942887c7f9b8ecfa3b6f4fe0cf46213eda08aa";
const CURRENT_BASE: &str = "81cb89864a01cc2254c7744e9bd6518425e64458";
const CURRENT_BASE_TREE: &str = "99513f270a58d0775c6ff7dc8752fdb7999a049f";
const CURRENT_MERGE_TREE: &str = "d3942887c7f9b8ecfa3b6f4fe0cf46213eda08aa";

const INTERVENING_CI_COMMIT: &str = "81cb89864a01cc2254c7744e9bd6518425e64458";
const ACTIVE_WORK_SCRIPT: &str = "scripts/active_work_heads_up_snapshot_ci.py";
const ACTIVE_WORK_CONTROL: &str = "scripts/active_work_heads_up_snapshot_ci_test.py";

fn entry(path: &str, blob_sha: &str) -> PromotionChangeSetEntry {
    PromotionChangeSetEntry {
        path: path.to_string(),
        blob_sha: blob_sha.to_string(),
    }
}

fn readiness_payload() -> Vec<PromotionChangeSetEntry> {
    vec![
        entry(
            "src/refinement_candidate_readiness.rs",
            "c47cd53d308edd48a77740a1841c1a714ef57452",
        ),
        entry(
            "tests/refinement_candidate_readiness.rs",
            "f0e074138365a620e64cb96949d048a96865f4c8",
        ),
        entry(
            "examples/refinement_candidate_readiness.rs",
            "e61b4d5a2a1b6b418fb49f0a2c057aca6e464379",
        ),
        entry(
            "research/refinement-candidate-readiness.md",
            "7e98dcb78df6507a20793e287d7b822d6d593dae",
        ),
    ]
}

fn real_request(applicable_policy_paths: Vec<String>) -> PromotionReceiptRequest {
    let tested_payload = readiness_payload();
    let mut current_payload = readiness_payload();
    current_payload.reverse();
    let tested_change_set = fingerprint_promotion_change_set(&tested_payload).unwrap();
    let current_change_set = fingerprint_promotion_change_set(&current_payload).unwrap();
    assert_eq!(tested_change_set, current_change_set);

    PromotionReceiptRequest {
        schema_version: PROMOTION_RECEIPT_SCHEMA_VERSION,
        tested: TestedPromotionState {
            head_sha: OLD_HEAD.to_string(),
            tree_sha: OLD_TREE.to_string(),
            change_set_sha256: tested_change_set,
            base_sha: OLD_BASE.to_string(),
            base_tree_sha: OLD_BASE_TREE.to_string(),
            effective_merge_tree_sha: OLD_MERGE_TREE.to_string(),
            successful_check_refs: vec![
                "github-actions:ci/32661728874".to_string(),
                "github-actions:provenance/32661728878".to_string(),
            ],
        },
        current: CurrentPromotionState {
            head_sha: CURRENT_HEAD.to_string(),
            tree_sha: CURRENT_TREE.to_string(),
            change_set_sha256: current_change_set,
            base_sha: CURRENT_BASE.to_string(),
            base_tree_sha: CURRENT_BASE_TREE.to_string(),
            effective_merge_tree_sha: CURRENT_MERGE_TREE.to_string(),
            mergeable: true,
            conflict: false,
        },
        branch_changed_paths: readiness_payload()
            .into_iter()
            .map(|entry| entry.path)
            .collect(),
        consumed_contract_paths: vec![
            "src/discriminator_observation.rs".to_string(),
            "src/observation_frontier.rs".to_string(),
            "src/refinement_episode.rs".to_string(),
            "src/refinement_observation_requirement.rs".to_string(),
            "research/discriminator-observations/cultist-v1.json".to_string(),
            "research/refinement-episodes/cultist-v1.json".to_string(),
            "research/refinement-observation-requirements/cultist-v1.json".to_string(),
            "research/refinement-observation-requirements/oxc-focused-edit-class-v1.json"
                .to_string(),
        ],
        applicable_policy_paths,
        intervening_commits: vec![InterveningCommit {
            sha: INTERVENING_CI_COMMIT.to_string(),
            changed_paths: vec![
                ACTIVE_WORK_SCRIPT.to_string(),
                ACTIVE_WORK_CONTROL.to_string(),
            ],
        }],
        compatibility_scope_complete: false,
    }
}

#[test]
fn pr_344_reanchor_was_required_by_executed_ci_policy_overlap() {
    let request = real_request(vec![
        ACTIVE_WORK_SCRIPT.to_string(),
        ACTIVE_WORK_CONTROL.to_string(),
    ]);

    assert_ne!(request.tested.head_sha, request.current.head_sha);
    assert_ne!(request.tested.base_tree_sha, request.current.base_tree_sha);
    assert_ne!(
        request.tested.effective_merge_tree_sha,
        request.current.effective_merge_tree_sha
    );
    assert_eq!(
        request.tested.change_set_sha256,
        request.current.change_set_sha256
    );

    let evaluation = evaluate_promotion_receipt(&request).unwrap();
    assert_eq!(
        evaluation.disposition,
        PromotionReceiptDisposition::RerunRequired
    );
    assert_eq!(
        evaluation.reasons,
        vec![PromotionReceiptReason::ApplicablePolicyOverlap]
    );
    assert_eq!(evaluation.overlaps.len(), 2);
    assert!(
        evaluation
            .overlaps
            .iter()
            .all(|overlap| overlap.kind == PromotionPathOverlapKind::ApplicablePolicy)
    );
    assert_eq!(
        evaluation.intervening_commit_shas,
        vec![INTERVENING_CI_COMMIT]
    );
}

#[test]
fn byte_identical_payload_without_complete_policy_scope_stays_unknown() {
    let request = real_request(Vec::new());
    let evaluation = evaluate_promotion_receipt(&request).unwrap();

    assert_eq!(
        evaluation.disposition,
        PromotionReceiptDisposition::InspectSemanticOverlap
    );
    assert_eq!(
        evaluation.reasons,
        vec![PromotionReceiptReason::SemanticIndependenceUnknown]
    );
    assert!(evaluation.overlaps.is_empty());
}
