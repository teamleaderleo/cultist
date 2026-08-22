#![allow(dead_code)]

#[path = "../src/active_changes.rs"]
mod active_changes;
#[path = "../src/applicability.rs"]
mod applicability;
#[path = "../src/finding.rs"]
mod finding;
#[path = "../src/provider_snapshot_applicability.rs"]
mod provider_snapshot_applicability;

use std::fmt::Write as _;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use provider_snapshot_applicability::{
    ProviderSnapshotIdentity, evaluate_provider_snapshot,
};
use serde::Serialize;
use serde_json::json;
use sha2::{Digest, Sha256};

mod selection_contract {
    #![allow(dead_code)]

    include!("provider_selection_contract.rs");

    pub(super) fn frozen_identity() -> String {
        selection_identity(&baseline()).unwrap()
    }
}

mod work_fact_contract {
    #![allow(dead_code)]

    include!("provider_work_fact_contract.rs");

    const HEAD_10: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const HEAD_20: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    const HEAD_30: &str = "cccccccccccccccccccccccccccccccccccccccc";

    fn frozen_work() -> Vec<WorkInput<'static>> {
        vec![
            WorkInput {
                id: "pull/10",
                head_sha: HEAD_10,
                activity: ActivityInput::ConfirmedActive,
                changed_paths: vec!["src/lib.rs"],
            },
            WorkInput {
                id: "pull/20",
                head_sha: HEAD_20,
                activity: ActivityInput::ConfirmedActive,
                changed_paths: vec!["src/lib.rs"],
            },
        ]
    }

    pub(super) fn frozen_identity() -> String {
        fingerprint(&frozen_work(), &[]).unwrap()
    }

    pub(super) fn current_with_new_work_identity() -> String {
        let mut work = frozen_work();
        work.push(WorkInput {
            id: "pull/30",
            head_sha: HEAD_30,
            activity: ActivityInput::ConfirmedActive,
            changed_paths: vec!["src/new.rs"],
        });
        fingerprint(&work, &[]).unwrap()
    }
}

const SNAPSHOT_COMPOSITION_SCHEMA_VERSION: u32 = 0;

#[derive(Serialize)]
struct ProviderSnapshotDocument<'a> {
    schema_version: u32,
    selection_identity: &'a str,
    work_fact_identity: &'a str,
}

fn snapshot_identity(selection_identity: &str, work_fact_identity: &str) -> String {
    let document = ProviderSnapshotDocument {
        schema_version: SNAPSHOT_COMPOSITION_SCHEMA_VERSION,
        selection_identity,
        work_fact_identity,
    };
    let bytes = serde_json::to_vec(&document).unwrap();
    let digest = Sha256::digest(bytes);
    let mut encoded = String::with_capacity(digest.len() * 2);
    for byte in digest {
        write!(&mut encoded, "{byte:02x}").expect("writing to String cannot fail");
    }
    encoded
}

fn product_identity(work_fact_identity: &str) -> ProviderSnapshotIdentity {
    let digest = snapshot_identity(
        &selection_contract::frozen_identity(),
        work_fact_identity,
    );
    ProviderSnapshotIdentity::parse(format!("sha256:{digest}")).unwrap()
}

fn required_snapshot() -> ProviderSnapshotIdentity {
    product_identity(&work_fact_contract::frozen_identity())
}

fn current_snapshot_with_new_work() -> ProviderSnapshotIdentity {
    product_identity(&work_fact_contract::current_with_new_work_identity())
}

fn test_root() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "cultist-active-inventory-provider-snapshot-gap-{}-{nonce}",
        std::process::id()
    ));
    fs::create_dir_all(&root).unwrap();
    root
}

fn frozen_inventory(root: &std::path::Path) -> PathBuf {
    let path = root.join("inventory.json");
    let document = json!({
        "schema_version": 1,
        "source": "test:provider-snapshot-gap",
        "observed_at": "2026-08-22T19:10:00Z",
        "current": {
            "id": "pull/10",
            "kind": "pull_request",
            "title": "current work",
            "url": "https://example.invalid/pull/10",
            "head_ref": "feature/current",
            "head_sha": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "updated_at": "2026-08-22T19:09:00Z",
            "draft": false,
            "activity": "confirmed_active",
            "changed_paths": ["src/lib.rs"]
        },
        "active_work": [{
            "id": "pull/20",
            "kind": "pull_request",
            "title": "other work",
            "url": "https://example.invalid/pull/20",
            "head_ref": "feature/other",
            "head_sha": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            "updated_at": "2026-08-22T19:09:30Z",
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
    let report = active_changes::build_active_inventory_analysis_report(&root, &inventory, None)
        .unwrap();

    assert!(report.findings.iter().any(|finding| {
        finding.kind == "preflight-inventory-path-overlap"
            && finding.title == "Active-change path overlap"
    }));

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn new_provider_work_invalidates_snapshot_without_gating_frozen_inventory_collision() {
    let required = required_snapshot();
    let current = current_snapshot_with_new_work();
    let applicability = evaluate_provider_snapshot(&required, Some(&current));

    assert_eq!(applicability.status, applicability::ApplicabilityStatus::Invalid);
    analyzer_still_emits_strong_overlap();
}

#[test]
fn unavailable_provider_snapshot_does_not_preserve_unknown_in_consumer() {
    let required = required_snapshot();
    let applicability = evaluate_provider_snapshot(&required, None);

    assert_eq!(applicability.status, applicability::ApplicabilityStatus::Unknown);
    analyzer_still_emits_strong_overlap();
}

#[test]
fn exact_same_provider_snapshot_applies_as_control() {
    let required = required_snapshot();
    let current = required_snapshot();
    let applicability = evaluate_provider_snapshot(&required, Some(&current));

    assert_eq!(applicability.status, applicability::ApplicabilityStatus::Applies);
}
