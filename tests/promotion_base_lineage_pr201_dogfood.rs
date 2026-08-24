#[allow(dead_code)]
#[path = "../src/promotion_base_lineage.rs"]
mod promotion_base_lineage;
#[allow(dead_code)]
#[path = "../src/promotion_change_set.rs"]
mod promotion_change_set;
#[allow(dead_code)]
#[path = "../src/promotion_receipt.rs"]
mod promotion_receipt;

use std::process::Command;

use promotion_base_lineage::{
    PROMOTION_BASE_LINEAGE_SCHEMA_VERSION, PromotionBaseLineageRequest, PromotionBaseRange,
    PromotionBaseRelation, PromotionCompatibilityObjectKind, PromotionCompatibilityObjectState,
    evaluate_promotion_base_lineage,
};
use promotion_change_set::{PromotionChangeSetEntry, fingerprint_promotion_change_set};
use promotion_receipt::{
    CurrentPromotionState, PROMOTION_RECEIPT_SCHEMA_VERSION, PromotionReceiptDisposition,
    PromotionReceiptReason, PromotionReceiptRequest, TestedPromotionState,
};

const MERGE_BASE: &str = "c2133131038f33a98f7bc7d206ca6e4284be420a";
const TESTED_BASE: &str = "d3482178aee3cba7af496abd3adab8ac162639cb";
const TESTED_BASE_TREE: &str = "2206ff89408e902354494cac853e0311c2143f57";
const TESTED_HEAD: &str = "1614cc2ae82df50ec3c8b5c4a9e428ad01c1d50f";
const TESTED_TREE: &str = "a90a3317c50d5d7d693b948cc9414315056c628f";
const TESTED_CHECK: &str = "github-actions:ci/32250242114";

const CURRENT_BASE: &str = "79c5c64d2cd17869715c0818e0bf86bdad5b7322";
const CURRENT_BASE_TREE: &str = "cb9fdb279cdff8fdf6362d317166f7d82df51e87";
const CURRENT_HEAD: &str = "3cf9090dfb474adaac6ab773c357627c37c3f9e6";
const CURRENT_TREE: &str = "889e34a998fe268986718bf21e72263503a1a05b";
const CURRENT_CHECK: &str = "github-actions:ci/32270569205";

const BRANCH_PATH: &str = "tests/known_stale_observation_frontier.rs";
const BRANCH_BLOB: &str = "c5945b4cfee5f6ea43f782d0c5b68fa8a9125ef4";
const APPLICABILITY: &str = "src/applicability.rs";
const DISCRIMINATOR_OBSERVATION: &str = "src/discriminator_observation.rs";
const OBSERVATION_FRONTIER: &str = "src/observation_frontier.rs";
const CI_POLICY: &str = ".github/workflows/ci.yml";

fn git_output(args: &[&str]) -> String {
    let output = Command::new("git").args(args).output().unwrap();
    assert!(
        output.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap().trim().to_string()
}

fn tree_sha(commit: &str) -> String {
    let spec = format!("{commit}^{{tree}}");
    git_output(&["rev-parse", &spec])
}

fn object_sha(commit: &str, path: &str) -> String {
    let spec = format!("{commit}:{path}");
    git_output(&["rev-parse", &spec])
}

fn changed_paths(base: &str, head: &str) -> Vec<String> {
    let output = git_output(&["diff", "--name-only", base, head, "--"]);
    if output.is_empty() {
        Vec::new()
    } else {
        output.lines().map(str::to_string).collect()
    }
}

fn commit_count(base: &str, head: &str) -> usize {
    let range = format!("{base}..{head}");
    git_output(&["rev-list", "--count", &range])
        .parse()
        .unwrap()
}

fn compatibility_state(
    path: &str,
    kind: PromotionCompatibilityObjectKind,
) -> PromotionCompatibilityObjectState {
    PromotionCompatibilityObjectState {
        path: path.to_string(),
        kind,
        tested_object_sha: Some(object_sha(TESTED_BASE, path)),
        current_object_sha: Some(object_sha(CURRENT_BASE, path)),
    }
}

fn request() -> PromotionBaseLineageRequest {
    assert_eq!(tree_sha(TESTED_BASE), TESTED_BASE_TREE);
    assert_eq!(tree_sha(TESTED_HEAD), TESTED_TREE);
    assert_eq!(tree_sha(CURRENT_BASE), CURRENT_BASE_TREE);
    assert_eq!(tree_sha(CURRENT_HEAD), CURRENT_TREE);
    assert_eq!(object_sha(TESTED_HEAD, BRANCH_PATH), BRANCH_BLOB);
    assert_eq!(object_sha(CURRENT_HEAD, BRANCH_PATH), BRANCH_BLOB);

    let tested_change_set = fingerprint_promotion_change_set(&[PromotionChangeSetEntry {
        path: BRANCH_PATH.to_string(),
        blob_sha: BRANCH_BLOB.to_string(),
    }])
    .unwrap();
    let current_change_set = fingerprint_promotion_change_set(&[PromotionChangeSetEntry {
        path: BRANCH_PATH.to_string(),
        blob_sha: BRANCH_BLOB.to_string(),
    }])
    .unwrap();
    assert_eq!(tested_change_set, current_change_set);

    let promotion = PromotionReceiptRequest {
        schema_version: PROMOTION_RECEIPT_SCHEMA_VERSION,
        tested: TestedPromotionState {
            head_sha: TESTED_HEAD.to_string(),
            tree_sha: TESTED_TREE.to_string(),
            change_set_sha256: tested_change_set,
            base_sha: TESTED_BASE.to_string(),
            base_tree_sha: TESTED_BASE_TREE.to_string(),
            effective_merge_tree_sha: TESTED_TREE.to_string(),
            successful_check_refs: vec![TESTED_CHECK.to_string()],
        },
        current: CurrentPromotionState {
            head_sha: CURRENT_HEAD.to_string(),
            tree_sha: CURRENT_TREE.to_string(),
            change_set_sha256: current_change_set,
            base_sha: CURRENT_BASE.to_string(),
            base_tree_sha: CURRENT_BASE_TREE.to_string(),
            effective_merge_tree_sha: CURRENT_TREE.to_string(),
            mergeable: true,
            conflict: false,
        },
        branch_changed_paths: vec![BRANCH_PATH.to_string()],
        consumed_contract_paths: vec![
            APPLICABILITY.to_string(),
            DISCRIMINATOR_OBSERVATION.to_string(),
            OBSERVATION_FRONTIER.to_string(),
        ],
        applicable_policy_paths: vec![CI_POLICY.to_string()],
        intervening_commits: Vec::new(),
        compatibility_scope_complete: false,
    };

    PromotionBaseLineageRequest {
        schema_version: PROMOTION_BASE_LINEAGE_SCHEMA_VERSION,
        promotion,
        merge_base_sha: MERGE_BASE.to_string(),
        tested_base_only: Some(PromotionBaseRange {
            base_sha: MERGE_BASE.to_string(),
            head_sha: TESTED_BASE.to_string(),
            commit_count: commit_count(MERGE_BASE, TESTED_BASE),
            changed_paths: changed_paths(MERGE_BASE, TESTED_BASE),
        }),
        current_base_only: Some(PromotionBaseRange {
            base_sha: MERGE_BASE.to_string(),
            head_sha: CURRENT_BASE.to_string(),
            commit_count: commit_count(MERGE_BASE, CURRENT_BASE),
            changed_paths: changed_paths(MERGE_BASE, CURRENT_BASE),
        }),
        compatibility_objects: vec![
            compatibility_state(
                APPLICABILITY,
                PromotionCompatibilityObjectKind::ConsumedContract,
            ),
            compatibility_state(
                DISCRIMINATOR_OBSERVATION,
                PromotionCompatibilityObjectKind::ConsumedContract,
            ),
            compatibility_state(
                OBSERVATION_FRONTIER,
                PromotionCompatibilityObjectKind::ConsumedContract,
            ),
            compatibility_state(
                CI_POLICY,
                PromotionCompatibilityObjectKind::ApplicablePolicy,
            ),
        ],
        base_path_receipts_complete: true,
    }
}

#[test]
fn pr_201_divergent_reanchor_was_required_by_ci_policy_change() {
    let request = request();
    assert_eq!(request.tested_base_only.as_ref().unwrap().commit_count, 6);
    assert_eq!(request.current_base_only.as_ref().unwrap().commit_count, 37);

    let evaluation = evaluate_promotion_base_lineage(&request).unwrap();
    assert_eq!(evaluation.base_relation, PromotionBaseRelation::Diverged);
    assert_eq!(
        evaluation.disposition,
        PromotionReceiptDisposition::RerunRequired
    );
    assert_eq!(
        evaluation.reasons,
        vec![PromotionReceiptReason::ApplicablePolicyOverlap]
    );
    assert!(evaluation.branch_path_overlaps.is_empty());
    assert_eq!(evaluation.compatibility_changes.len(), 1);
    assert_eq!(evaluation.compatibility_changes[0].path, CI_POLICY);
    assert_eq!(
        evaluation.compatibility_changes[0].kind,
        PromotionCompatibilityObjectKind::ApplicablePolicy
    );

    for contract in evaluation
        .compatibility_objects
        .iter()
        .filter(|state| state.kind == PromotionCompatibilityObjectKind::ConsumedContract)
    {
        assert_eq!(contract.tested_object_sha, contract.current_object_sha);
    }
    assert_eq!(evaluation.successful_check_refs, vec![TESTED_CHECK]);
    assert_ne!(TESTED_CHECK, CURRENT_CHECK);
}

#[test]
fn pr_201_divergence_without_the_policy_change_would_remain_unknown() {
    let mut request = request();
    let policy = request
        .compatibility_objects
        .iter_mut()
        .find(|state| state.kind == PromotionCompatibilityObjectKind::ApplicablePolicy)
        .unwrap();
    policy.current_object_sha = policy.tested_object_sha.clone();

    let evaluation = evaluate_promotion_base_lineage(&request).unwrap();
    assert_eq!(evaluation.base_relation, PromotionBaseRelation::Diverged);
    assert_eq!(
        evaluation.disposition,
        PromotionReceiptDisposition::InspectSemanticOverlap
    );
    assert_eq!(
        evaluation.reasons,
        vec![PromotionReceiptReason::SemanticIndependenceUnknown]
    );
    assert!(evaluation.branch_path_overlaps.is_empty());
    assert!(evaluation.compatibility_changes.is_empty());
}
