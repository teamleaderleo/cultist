#![allow(dead_code)]

#[path = "../src/active_changes.rs"]
mod active_changes;
#[path = "../src/applicability.rs"]
mod applicability;
#[path = "../src/finding.rs"]
mod finding;
#[path = "../src/provider_snapshot_applicability.rs"]
mod provider_snapshot_applicability;

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use active_changes::{
    ProviderCurrentWorkContext, build_active_inventory_analysis_report,
    build_active_inventory_analysis_report_with_provider_current,
};
use finding::ClaimKind;
use serde_json::json;

const REPOSITORY: &str = "owner/repo";
const FROZEN_HEAD: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const MOVED_HEAD: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

fn test_root(name: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "cultist-active-inventory-provider-current-{name}-{}-{nonce}",
        std::process::id()
    ));
    fs::create_dir_all(&root).unwrap();
    root
}

fn frozen_inventory(root: &Path) -> PathBuf {
    let path = root.join("inventory.json");
    let document = json!({
        "schema_version": 1,
        "source": "test:provider-current-gate",
        "observed_at": "2026-08-22T19:06:00Z",
        "current": {
            "id": "#10",
            "kind": "pull_request",
            "title": "current work",
            "url": "https://example.invalid/pull/10",
            "head_ref": "feature/current",
            "head_sha": FROZEN_HEAD,
            "updated_at": "2026-08-22T19:05:00Z",
            "draft": false,
            "activity": "confirmed_active",
            "changed_paths": ["src/lib.rs"]
        },
        "active_work": [{
            "id": "#20",
            "kind": "pull_request",
            "title": "other work",
            "url": "https://example.invalid/pull/20",
            "head_ref": "feature/other",
            "head_sha": "cccccccccccccccccccccccccccccccccccccccc",
            "updated_at": "2026-08-22T19:05:30Z",
            "draft": false,
            "activity": "confirmed_active",
            "changed_paths": ["src/lib.rs"]
        }]
    });
    fs::write(&path, serde_json::to_vec_pretty(&document).unwrap()).unwrap();
    path
}

fn context(repository: &str, work_id: &str, head_sha: Option<&str>) -> ProviderCurrentWorkContext {
    ProviderCurrentWorkContext {
        repository: repository.to_string(),
        work_id: work_id.to_string(),
        head_sha: head_sha.map(str::to_string),
    }
}

fn has_strong_overlap(report: &finding::AnalysisReport) -> bool {
    report.findings.iter().any(|finding| {
        finding.kind == "preflight-inventory-path-overlap"
            && finding.title == "Active-change path overlap"
    })
}

#[test]
fn matching_provider_current_coordinate_preserves_normal_collision_analysis() {
    let root = test_root("matching");
    let inventory = frozen_inventory(&root);
    let report = build_active_inventory_analysis_report_with_provider_current(
        &root,
        &inventory,
        None,
        REPOSITORY,
        &context(REPOSITORY, "#10", Some(FROZEN_HEAD)),
    )
    .unwrap();

    assert!(has_strong_overlap(&report));
    assert!(report.claims.iter().any(|claim| {
        claim.kind == ClaimKind::Derived
            && claim
                .message
                .contains("provider-current context matches frozen work")
    }));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn moved_provider_current_head_gates_strong_collision() {
    let root = test_root("moved");
    let inventory = frozen_inventory(&root);
    let report = build_active_inventory_analysis_report_with_provider_current(
        &root,
        &inventory,
        None,
        REPOSITORY,
        &context(REPOSITORY, "#10", Some(MOVED_HEAD)),
    )
    .unwrap();

    assert!(!has_strong_overlap(&report));
    let finding = report
        .findings
        .iter()
        .find(|finding| finding.kind == "preflight-inventory-current-work-applicability-invalid")
        .unwrap();
    assert!(
        finding
            .claims
            .iter()
            .any(|claim| claim.kind == ClaimKind::Derived)
    );
    assert!(
        finding
            .question
            .as_deref()
            .is_some_and(|question| { question.contains("Refresh the active-work inventory") })
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn unavailable_provider_current_head_preserves_unknown_and_gates_collision() {
    let root = test_root("unknown");
    let inventory = frozen_inventory(&root);
    let report = build_active_inventory_analysis_report_with_provider_current(
        &root,
        &inventory,
        None,
        REPOSITORY,
        &context(REPOSITORY, "#10", None),
    )
    .unwrap();

    assert!(!has_strong_overlap(&report));
    let finding = report
        .findings
        .iter()
        .find(|finding| finding.kind == "preflight-inventory-current-work-applicability-unknown")
        .unwrap();
    assert!(
        finding
            .claims
            .iter()
            .any(|claim| claim.kind == ClaimKind::Unknown)
    );
    assert!(
        finding
            .claims
            .iter()
            .flat_map(|claim| &claim.evidence)
            .any(|evidence| { evidence.message.contains("head=`<unavailable>`") })
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn mismatched_provider_work_identity_is_invalid_even_when_head_matches() {
    let root = test_root("wrong-work");
    let inventory = frozen_inventory(&root);
    let report = build_active_inventory_analysis_report_with_provider_current(
        &root,
        &inventory,
        None,
        REPOSITORY,
        &context(REPOSITORY, "#999", Some(FROZEN_HEAD)),
    )
    .unwrap();

    assert!(!has_strong_overlap(&report));
    assert!(report.findings.iter().any(|finding| {
        finding.kind == "preflight-inventory-current-work-applicability-invalid"
    }));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn mismatched_provider_repository_is_invalid_even_when_work_and_head_match() {
    let root = test_root("wrong-repository");
    let inventory = frozen_inventory(&root);
    let report = build_active_inventory_analysis_report_with_provider_current(
        &root,
        &inventory,
        None,
        REPOSITORY,
        &context("other/repo", "#10", Some(FROZEN_HEAD)),
    )
    .unwrap();

    assert!(!has_strong_overlap(&report));
    let finding = report
        .findings
        .iter()
        .find(|finding| finding.kind == "preflight-inventory-current-work-applicability-invalid")
        .unwrap();
    assert!(
        finding
            .claims
            .iter()
            .flat_map(|claim| &claim.evidence)
            .any(|evidence| evidence.message.contains("repository=`other/repo`"))
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn legacy_entrypoint_keeps_schema_v1_behavior_without_new_context() {
    let root = test_root("legacy");
    let inventory = frozen_inventory(&root);
    let report = build_active_inventory_analysis_report(&root, &inventory, None).unwrap();

    assert!(has_strong_overlap(&report));
    fs::remove_dir_all(root).unwrap();
}
