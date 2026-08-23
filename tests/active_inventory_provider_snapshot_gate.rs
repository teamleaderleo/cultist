// Connector-authored reanchor: semantic controls below are unchanged.
#[allow(dead_code)]
#[path = "../src/active_changes.rs"]
mod active_changes;
#[allow(dead_code)]
#[path = "../src/active_inventory_provider_snapshot.rs"]
mod active_inventory_provider_snapshot;
#[allow(dead_code)]
#[path = "../src/applicability.rs"]
mod applicability;
#[allow(dead_code)]
#[path = "../src/finding.rs"]
mod finding;
#[allow(dead_code)]
#[path = "../src/provider_snapshot_applicability.rs"]
mod provider_snapshot_applicability;

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use active_changes::build_active_inventory_analysis_report;
use active_inventory_provider_snapshot::build_active_inventory_analysis_report_with_provider_snapshot;
use provider_snapshot_applicability::ProviderSnapshotIdentity;
use serde_json::{Value, json};

fn snapshot(byte: char) -> ProviderSnapshotIdentity {
    ProviderSnapshotIdentity::parse(format!("sha256:{}", byte.to_string().repeat(64))).unwrap()
}

fn work(id: &str, sha: &str, path: &str) -> Value {
    json!({
        "id": id,
        "kind": "pull_request",
        "title": format!("Work {id}"),
        "url": format!("https://example.invalid/{id}"),
        "head_ref": format!("branch-{id}"),
        "head_sha": sha,
        "updated_at": "2026-08-23T00:00:00Z",
        "draft": false,
        "activity": "confirmed_active",
        "changed_paths": [path],
    })
}

fn inventory(required: Option<&ProviderSnapshotIdentity>) -> Value {
    let mut value = json!({
        "schema_version": 1,
        "source": "test_provider_snapshot",
        "observed_at": "2026-08-23T00:01:00Z",
        "current": work("#10", "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", "src/lib.rs"),
        "active_work": [
            work("#10", "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", "src/lib.rs"),
            work("#20", "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb", "src/lib.rs")
        ],
        "coordination_edges": [],
    });
    if let Some(required) = required {
        value["provider_snapshot_identity"] = json!(required.as_str());
    }
    value
}

fn unique_temp_file(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "cultist-provider-snapshot-{name}-{}-{nanos}.json",
        std::process::id()
    ))
}

fn write_inventory(name: &str, value: &Value) -> PathBuf {
    let path = unique_temp_file(name);
    fs::write(&path, serde_json::to_vec_pretty(value).unwrap()).unwrap();
    path
}

fn has_kind(report: &finding::AnalysisReport, kind: &str) -> bool {
    report.findings.iter().any(|finding| finding.kind == kind)
}

#[test]
fn matching_provider_snapshot_allows_current_routing_findings() {
    let required = snapshot('a');
    let path = write_inventory("applies", &inventory(Some(&required)));
    let report = build_active_inventory_analysis_report_with_provider_snapshot(
        Path::new("/repo"),
        &path,
        None,
        Some(&required),
    )
    .unwrap();
    fs::remove_file(path).unwrap();

    assert!(has_kind(&report, "preflight-inventory-path-overlap"));
    assert!(!has_kind(
        &report,
        "preflight-inventory-provider-snapshot-invalid"
    ));
}

#[test]
fn moved_provider_snapshot_withholds_strong_collision_findings() {
    let required = snapshot('a');
    let current = snapshot('b');
    let path = write_inventory("invalid", &inventory(Some(&required)));
    let report = build_active_inventory_analysis_report_with_provider_snapshot(
        Path::new("/repo"),
        &path,
        None,
        Some(&current),
    )
    .unwrap();
    fs::remove_file(path).unwrap();

    assert!(!has_kind(&report, "preflight-inventory-path-overlap"));
    assert!(has_kind(
        &report,
        "preflight-inventory-provider-snapshot-invalid"
    ));
}

#[test]
fn unavailable_current_provider_snapshot_withholds_strong_collision_findings() {
    let required = snapshot('a');
    let path = write_inventory("unknown", &inventory(Some(&required)));
    let report = build_active_inventory_analysis_report_with_provider_snapshot(
        Path::new("/repo"),
        &path,
        None,
        None,
    )
    .unwrap();
    fs::remove_file(path).unwrap();

    assert!(!has_kind(&report, "preflight-inventory-path-overlap"));
    assert!(has_kind(
        &report,
        "preflight-inventory-provider-snapshot-unknown"
    ));
}

#[test]
fn legacy_unbound_inventory_keeps_existing_behavior_without_current_context() {
    let path = write_inventory("legacy", &inventory(None));
    let report = build_active_inventory_analysis_report_with_provider_snapshot(
        Path::new("/repo"),
        &path,
        None,
        None,
    )
    .unwrap();
    fs::remove_file(path).unwrap();

    assert!(has_kind(&report, "preflight-inventory-path-overlap"));
}

#[test]
fn current_snapshot_cannot_be_paired_with_unbound_inventory() {
    let current = snapshot('a');
    let path = write_inventory("unbound-current", &inventory(None));
    let error = build_active_inventory_analysis_report_with_provider_snapshot(
        Path::new("/repo"),
        &path,
        None,
        Some(&current),
    )
    .unwrap_err();
    fs::remove_file(path).unwrap();

    assert!(error.to_string().contains("does not bind"));
}

#[test]
fn ungated_entrypoint_rejects_snapshot_bound_inventory() {
    let required = snapshot('a');
    let path = write_inventory("ungated", &inventory(Some(&required)));
    let error =
        build_active_inventory_analysis_report(Path::new("/repo"), &path, None).unwrap_err();
    fs::remove_file(path).unwrap();

    assert!(
        error
            .to_string()
            .contains("requires an explicit current provider snapshot")
    );
}

#[test]
fn explicit_null_provider_snapshot_fails_closed_in_gated_path() {
    let mut value = inventory(None);
    value["provider_snapshot_identity"] = Value::Null;
    let path = write_inventory("null-gated", &value);
    let error = build_active_inventory_analysis_report_with_provider_snapshot(
        Path::new("/repo"),
        &path,
        None,
        None,
    )
    .unwrap_err();
    fs::remove_file(path).unwrap();

    assert!(error.to_string().contains("must be a canonical"));
}

#[test]
fn explicit_null_provider_snapshot_cannot_bypass_ungated_entrypoint() {
    let mut value = inventory(None);
    value["provider_snapshot_identity"] = Value::Null;
    let path = write_inventory("null-ungated", &value);
    let error =
        build_active_inventory_analysis_report(Path::new("/repo"), &path, None).unwrap_err();
    fs::remove_file(path).unwrap();

    assert!(
        error
            .to_string()
            .contains("requires an explicit current provider snapshot")
    );
}

#[test]
fn malformed_bound_provider_snapshot_fails_closed() {
    let mut value = inventory(None);
    value["provider_snapshot_identity"] = json!("sha256:ABC");
    let path = write_inventory("malformed", &value);
    let error = build_active_inventory_analysis_report_with_provider_snapshot(
        Path::new("/repo"),
        &path,
        None,
        None,
    )
    .unwrap_err();
    fs::remove_file(path).unwrap();

    assert!(error.to_string().contains("lowercase hexadecimal"));
}
