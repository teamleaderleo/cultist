#[allow(dead_code)]
#[path = "../src/promotion_change_set.rs"]
mod promotion_change_set;
#[allow(dead_code)]
#[path = "../src/promotion_receipt.rs"]
mod promotion_receipt;

use promotion_change_set::{PromotionChangeSetEntry, fingerprint_promotion_change_set};
use promotion_receipt::{
    CurrentPromotionState, PROMOTION_RECEIPT_SCHEMA_VERSION, PromotionReceiptDisposition,
    PromotionReceiptReason, PromotionReceiptRequest, TestedPromotionState,
    evaluate_promotion_receipt,
};
use std::process::Command;

const TESTED_HEAD: &str = "e33cc0e736d3d9092cd23782cef4a8ad2cd1935a";
const CURRENT_HEAD: &str = "9299022e76406617c06858d9d6d441c7e13b43b5";
const SHARED_TREE: &str = "1ac85baec7efa0ca5170bdcf54c206406dccc60c";
const SHARED_BASE: &str = "eb9c0eeed395b9e40addc30c408d4ff83f24ab42";
const SHARED_BASE_TREE: &str = "a78e68fb990588239e248e37aec58f5db6987aa5";

fn git(args: &[&str]) -> String {
    let output = Command::new("git")
        .args(args)
        .output()
        .expect("git must execute in hosted CI");
    assert!(output.status.success(), "git {:?} failed: {}", args, String::from_utf8_lossy(&output.stderr));
    String::from_utf8(output.stdout)
        .expect("git output must be UTF-8")
        .trim()
        .to_string()
}

fn entry(path: &str, blob_sha: &str) -> PromotionChangeSetEntry {
    PromotionChangeSetEntry {
        path: path.to_string(),
        blob_sha: blob_sha.to_string(),
    }
}

fn justification_payload() -> Vec<PromotionChangeSetEntry> {
    vec![
        entry(
            "examples/justification_graph.rs",
            "7d62f8b5c8bfc2d52787497dc1a308986926869a",
        ),
        entry(
            "research/justification-graph.md",
            "670c5045b2a12fddf030785b849239dc7521b811",
        ),
        entry(
            "src/justification.rs",
            "281f2bf9c1da5ad84671ad7f8fadd961d6294ff3",
        ),
        entry(
            "tests/justification.rs",
            "9faf58d98bc470d8b260432aba7592ca31064114",
        ),
    ]
}

#[test]
fn pr_156_metadata_only_rewrite_could_reuse_the_successful_receipt() {
    // This is the real #156 final promotion-authority head followed by the later
    // branch-head rewrite. Both commits remain in repository history.
    assert_eq!(git(&["show", "-s", "--format=%T", TESTED_HEAD]), SHARED_TREE);
    assert_eq!(git(&["show", "-s", "--format=%T", CURRENT_HEAD]), SHARED_TREE);
    assert_eq!(git(&["rev-parse", &format!("{TESTED_HEAD}^")]), SHARED_BASE);
    assert_eq!(git(&["rev-parse", &format!("{CURRENT_HEAD}^")]), SHARED_BASE);
    assert_eq!(git(&["show", "-s", "--format=%T", SHARED_BASE]), SHARED_BASE_TREE);

    let tested_payload = justification_payload();
    let mut current_payload = justification_payload();
    current_payload.reverse();
    let tested_change_set = fingerprint_promotion_change_set(&tested_payload).unwrap();
    let current_change_set = fingerprint_promotion_change_set(&current_payload).unwrap();
    assert_eq!(tested_change_set, current_change_set);

    let request = PromotionReceiptRequest {
        schema_version: PROMOTION_RECEIPT_SCHEMA_VERSION,
        tested: TestedPromotionState {
            head_sha: TESTED_HEAD.to_string(),
            tree_sha: SHARED_TREE.to_string(),
            change_set_sha256: tested_change_set,
            base_sha: SHARED_BASE.to_string(),
            base_tree_sha: SHARED_BASE_TREE.to_string(),
            effective_merge_tree_sha: SHARED_TREE.to_string(),
            successful_check_refs: vec![
                "github-actions:ci/32272350023".to_string(),
                "github-actions:provenance/32272350050".to_string(),
            ],
        },
        current: CurrentPromotionState {
            head_sha: CURRENT_HEAD.to_string(),
            tree_sha: SHARED_TREE.to_string(),
            change_set_sha256: current_change_set,
            base_sha: SHARED_BASE.to_string(),
            base_tree_sha: SHARED_BASE_TREE.to_string(),
            effective_merge_tree_sha: SHARED_TREE.to_string(),
            mergeable: true,
            conflict: false,
        },
        branch_changed_paths: justification_payload()
            .into_iter()
            .map(|entry| entry.path)
            .collect(),
        consumed_contract_paths: vec!["src/applicability.rs".to_string()],
        applicable_policy_paths: vec![".github/workflows/ci.yml".to_string()],
        intervening_commits: Vec::new(),
        compatibility_scope_complete: false,
    };

    assert_ne!(request.tested.head_sha, request.current.head_sha);
    assert_eq!(request.tested.tree_sha, request.current.tree_sha);
    assert_eq!(request.tested.base_sha, request.current.base_sha);
    assert_eq!(request.tested.base_tree_sha, request.current.base_tree_sha);
    assert_eq!(
        request.tested.effective_merge_tree_sha,
        request.current.effective_merge_tree_sha
    );

    let evaluation = evaluate_promotion_receipt(&request).unwrap();
    assert_eq!(
        evaluation.disposition,
        PromotionReceiptDisposition::ReceiptReusable
    );
    assert_eq!(
        evaluation.reasons,
        vec![PromotionReceiptReason::ExactEffectiveMergeTreeIdentity]
    );
    assert!(evaluation.overlaps.is_empty());
    assert!(evaluation.intervening_commit_shas.is_empty());
}
