#![allow(dead_code)]

#[path = "../src/active_changes.rs"]
mod active_changes;
#[path = "../src/applicability.rs"]
mod applicability;
#[path = "../src/finding.rs"]
mod finding;

use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use applicability::{
    APPLICABILITY_SCHEMA_VERSION, ApplicabilityQuery, ApplicabilityStatus, EvaluationContext,
    EvidenceRequirements, evaluate_query,
};
use serde_json::json;

const REPOSITORY: &str = "owner/repo";
const WORK: &str = "#10";
const FROZEN_HEAD: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const MOVED_HEAD: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

fn provider_applicability(current_head: Option<&str>) -> ApplicabilityStatus {
    evaluate_query(&ApplicabilityQuery {
        schema_version: APPLICABILITY_SCHEMA_VERSION,
        requirements: EvidenceRequirements {
            repository: Some(REPOSITORY.to_string()),
            revision: Some(FROZEN_HEAD.to_string()),
            work: Some(WORK.to_string()),
            scope: None,
        },
        context: EvaluationContext {
            repository: Some(REPOSITORY.to_string()),
            revision: current_head.map(str::to_string),
            work: Some(WORK.to_string()),
            path: None,
        },
    })
    .unwrap()
    .status
}

fn test_root() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "cultist-active-inventory-provider-current-gap-{}-{nonce}",
        std::process::id()
    ));
    fs::create_dir_all(&root).unwrap();
    root
}

fn frozen_inventory(root: &std::path::Path) -> PathBuf {
    let path = root.join("inventory.json");
    let document = json!({
        "schema_version": 1,
        "source": "test:provider-current-gap",
        "observed_at": "2026-08-22T18:58:00Z",
        "current": {
            "id": WORK,
            "kind": "pull_request",
            "title": "current work",
            "url": "https://example.invalid/pull/10",
            "head_ref": "feature/current",
            "head_sha": FROZEN_HEAD,
            "updated_at": "2026-08-22T18:57:00Z",
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
            "updated_at": "2026-08-22T18:57:30Z",
            "draft": false,
            "activity": "confirmed_active",
            "changed_paths": ["src/lib.rs"]
        }]
    });
    fs::write(&path, serde_json::to_vec_pretty(&document).unwrap()).unwrap();
    path
}

fn analyzer_still_emits_strong_overlap() {
    let root = test_root();
    let inventory = frozen_inventory(&root);
    let report =
        active_changes::build_active_inventory_analysis_report(&root, &inventory, None).unwrap();

    assert!(report.findings.iter().any(|finding| {
        finding.kind == "preflight-inventory-path-overlap"
            && finding.title == "Active-change path overlap"
    }));

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn invalid_provider_current_head_does_not_gate_frozen_inventory_collision() {
    assert_eq!(
        provider_applicability(Some(MOVED_HEAD)),
        ApplicabilityStatus::Invalid
    );

    analyzer_still_emits_strong_overlap();
}

#[test]
fn missing_provider_current_head_does_not_preserve_unknown_in_consumer() {
    assert_eq!(provider_applicability(None), ApplicabilityStatus::Unknown);

    analyzer_still_emits_strong_overlap();
}
