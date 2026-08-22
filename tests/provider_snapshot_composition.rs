#[allow(dead_code)]
#[path = "../src/applicability.rs"]
mod applicability;

use std::fmt::Write as _;

use applicability::{
    APPLICABILITY_SCHEMA_VERSION, ApplicabilityQuery, ApplicabilityStatus, EvaluationContext,
    EvidenceRequirements, evaluate_query,
};
use serde::Serialize;
use sha2::{Digest, Sha256};

const SNAPSHOT_COMPOSITION_SCHEMA_VERSION: u32 = 0;
const REPOSITORY_MAIN_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const REPOSITORY_MAIN_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

mod selection_contract {
    #![allow(dead_code)]

    include!("provider_selection_contract.rs");

    pub(super) fn baseline_identity() -> String {
        selection_identity(&baseline()).unwrap()
    }

    pub(super) fn exclude_drafts_identity() -> String {
        let mut input = baseline();
        input.draft_policy = DraftPolicyInput::Exclude;
        selection_identity(&input).unwrap()
    }

    pub(super) fn other_repository_identity() -> String {
        let mut input = baseline();
        input.collection = "teamleaderleo/other";
        selection_identity(&input).unwrap()
    }
}

mod work_fact_contract {
    #![allow(dead_code)]

    include!("provider_work_fact_contract.rs");

    pub(super) fn baseline_identity() -> String {
        fingerprint(&baseline_work(), &[]).unwrap()
    }

    pub(super) fn new_work_identity() -> String {
        let mut work = baseline_work();
        work.push(WorkInput {
            id: "pull/627",
            head_sha: "769ded20439efe0567d4553141598cfd3965a013",
            activity: ActivityInput::ConfirmedActive,
            changed_paths: vec!["tests/test_research_610_strict_carrier.py"],
        });
        fingerprint(&work, &[]).unwrap()
    }

    pub(super) fn changed_head_identity() -> String {
        let mut work = baseline_work();
        work[0].head_sha = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        fingerprint(&work, &[]).unwrap()
    }

    pub(super) fn unresolved_activity_identity() -> String {
        let mut work = baseline_work();
        work[0].activity = ActivityInput::Unresolved;
        fingerprint(&work, &[]).unwrap()
    }

    pub(super) fn semantic_edge_identity() -> String {
        fingerprint(&baseline_work(), &[edge("provider:pull/604")]).unwrap()
    }

    pub(super) fn moved_source_identity() -> String {
        fingerprint(&baseline_work(), &[edge("provider:reviewed-metadata")]).unwrap()
    }
}

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

fn current_snapshot_identity(
    selection_identity: Option<&str>,
    work_fact_identity: Option<&str>,
) -> Option<String> {
    match (selection_identity, work_fact_identity) {
        (Some(selection), Some(work)) => Some(snapshot_identity(selection, work)),
        _ => None,
    }
}

fn baseline_snapshot_identity() -> String {
    snapshot_identity(
        &selection_contract::baseline_identity(),
        &work_fact_contract::baseline_identity(),
    )
}

fn evaluate_snapshot(required: &str, current: Option<&str>) -> ApplicabilityStatus {
    match current {
        Some(actual) if actual == required => ApplicabilityStatus::Applies,
        Some(_) => ApplicabilityStatus::Invalid,
        None => ApplicabilityStatus::Unknown,
    }
}

#[test]
fn same_composed_provider_snapshot_applies() {
    let required = baseline_snapshot_identity();
    let current = baseline_snapshot_identity();

    assert_eq!(
        evaluate_snapshot(&required, Some(&current)),
        ApplicabilityStatus::Applies
    );
}

#[test]
fn new_provider_work_invalidates_while_repository_revision_is_unchanged() {
    let required_main = REPOSITORY_MAIN_A;
    let current_main = REPOSITORY_MAIN_A;
    assert_eq!(required_main, current_main);

    let required = baseline_snapshot_identity();
    let current = snapshot_identity(
        &selection_contract::baseline_identity(),
        &work_fact_contract::new_work_identity(),
    );

    assert_eq!(
        evaluate_snapshot(&required, Some(&current)),
        ApplicabilityStatus::Invalid
    );
}

#[test]
fn selection_contract_change_invalidates_even_when_realized_work_facts_match() {
    let work = work_fact_contract::baseline_identity();
    let required = snapshot_identity(&selection_contract::baseline_identity(), &work);
    let current = snapshot_identity(&selection_contract::exclude_drafts_identity(), &work);

    assert_eq!(
        evaluate_snapshot(&required, Some(&current)),
        ApplicabilityStatus::Invalid
    );
}

#[test]
fn provider_scope_change_invalidates_even_when_local_work_facts_match() {
    let work = work_fact_contract::baseline_identity();
    let required = snapshot_identity(&selection_contract::baseline_identity(), &work);
    let current = snapshot_identity(&selection_contract::other_repository_identity(), &work);

    assert_eq!(
        evaluate_snapshot(&required, Some(&current)),
        ApplicabilityStatus::Invalid
    );
}

#[test]
fn head_and_declared_activity_movement_each_invalidate_snapshot_identity() {
    let selection = selection_contract::baseline_identity();
    let required = snapshot_identity(&selection, &work_fact_contract::baseline_identity());
    let changed_head = snapshot_identity(&selection, &work_fact_contract::changed_head_identity());
    let changed_activity = snapshot_identity(
        &selection,
        &work_fact_contract::unresolved_activity_identity(),
    );

    assert_eq!(
        evaluate_snapshot(&required, Some(&changed_head)),
        ApplicabilityStatus::Invalid
    );
    assert_eq!(
        evaluate_snapshot(&required, Some(&changed_activity)),
        ApplicabilityStatus::Invalid
    );
}

#[test]
fn coordination_provenance_movement_preserves_composed_snapshot_identity() {
    let selection = selection_contract::baseline_identity();
    let required = snapshot_identity(&selection, &work_fact_contract::semantic_edge_identity());
    let current = snapshot_identity(&selection, &work_fact_contract::moved_source_identity());

    assert_eq!(required, current);
    assert_eq!(
        evaluate_snapshot(&required, Some(&current)),
        ApplicabilityStatus::Applies
    );
}

#[test]
fn unavailable_current_provider_snapshot_is_unknown() {
    let required = baseline_snapshot_identity();

    assert_eq!(
        evaluate_snapshot(&required, None),
        ApplicabilityStatus::Unknown
    );
}

#[test]
fn missing_current_selection_or_work_fact_component_is_unknown() {
    let required = baseline_snapshot_identity();
    let selection = selection_contract::baseline_identity();
    let work = work_fact_contract::baseline_identity();

    let missing_selection = current_snapshot_identity(None, Some(&work));
    let missing_work = current_snapshot_identity(Some(&selection), None);

    assert_eq!(
        evaluate_snapshot(&required, missing_selection.as_deref()),
        ApplicabilityStatus::Unknown
    );
    assert_eq!(
        evaluate_snapshot(&required, missing_work.as_deref()),
        ApplicabilityStatus::Unknown
    );
}

#[test]
fn repository_revision_applicability_remains_a_separate_dimension() {
    let provider_required = baseline_snapshot_identity();
    let provider_current = baseline_snapshot_identity();
    assert_eq!(
        evaluate_snapshot(&provider_required, Some(&provider_current)),
        ApplicabilityStatus::Applies
    );

    let revision_query = ApplicabilityQuery {
        schema_version: APPLICABILITY_SCHEMA_VERSION,
        requirements: EvidenceRequirements {
            revision: Some(REPOSITORY_MAIN_A.to_string()),
            ..EvidenceRequirements::default()
        },
        context: EvaluationContext {
            revision: Some(REPOSITORY_MAIN_B.to_string()),
            ..EvaluationContext::default()
        },
    };
    let revision = evaluate_query(&revision_query).unwrap();

    assert_eq!(revision.status, ApplicabilityStatus::Invalid);
}

#[test]
fn missing_repository_revision_does_not_relabel_provider_snapshot_unknown() {
    let provider_required = baseline_snapshot_identity();
    let provider_current = baseline_snapshot_identity();
    assert_eq!(
        evaluate_snapshot(&provider_required, Some(&provider_current)),
        ApplicabilityStatus::Applies
    );

    let revision_query = ApplicabilityQuery {
        schema_version: APPLICABILITY_SCHEMA_VERSION,
        requirements: EvidenceRequirements {
            revision: Some(REPOSITORY_MAIN_A.to_string()),
            ..EvidenceRequirements::default()
        },
        context: EvaluationContext::default(),
    };
    let revision = evaluate_query(&revision_query).unwrap();

    assert_eq!(revision.status, ApplicabilityStatus::Unknown);
}
