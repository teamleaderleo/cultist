#[allow(dead_code)]
#[path = "../src/applicability.rs"]
mod applicability;

use applicability::{
    APPLICABILITY_SCHEMA_VERSION, ApplicabilityDimension, ApplicabilityQuery, ApplicabilityStatus,
    DimensionStatus, EvaluationContext, EvidenceRequirements, evaluate_query,
};

const REPOSITORY: &str = "Coreys-Quarry/quarry";
const SNAPSHOT_MAIN: &str = "6978e7522e7045aca16099e193396666b4141092";
const ADVANCED_MAIN: &str = "c9bdb356fdcba66ca34ad57af5f4c89dfddd30c7";
const PR_HEAD: &str = "63eece80df17a97a8544c4d716feca4fad1970ea";
const SYNTHETIC_MERGE_VIEW: &str = "02c30c4991660fc284d09c4e4fcf5d8fdd67f1be";

fn revision_query(required_revision: &str, current_revision: Option<&str>) -> ApplicabilityQuery {
    ApplicabilityQuery {
        schema_version: APPLICABILITY_SCHEMA_VERSION,
        requirements: EvidenceRequirements {
            repository: Some(REPOSITORY.to_string()),
            revision: Some(required_revision.to_string()),
            work: None,
            scope: None,
        },
        context: EvaluationContext {
            repository: Some(REPOSITORY.to_string()),
            revision: current_revision.map(str::to_string),
            work: None,
            path: None,
        },
    }
}

fn work_query(
    required_revision: &str,
    required_work: &str,
    current_revision: &str,
    current_work: &str,
) -> ApplicabilityQuery {
    ApplicabilityQuery {
        schema_version: APPLICABILITY_SCHEMA_VERSION,
        requirements: EvidenceRequirements {
            repository: Some(REPOSITORY.to_string()),
            revision: Some(required_revision.to_string()),
            work: Some(required_work.to_string()),
            scope: None,
        },
        context: EvaluationContext {
            repository: Some(REPOSITORY.to_string()),
            revision: Some(current_revision.to_string()),
            work: Some(current_work.to_string()),
            path: None,
        },
    }
}

#[test]
fn matching_provider_current_revision_keeps_routing_snapshot_applicable() {
    let evaluation = evaluate_query(&revision_query(SNAPSHOT_MAIN, Some(SNAPSHOT_MAIN))).unwrap();

    assert_eq!(evaluation.status, ApplicabilityStatus::Applies);
    assert!(evaluation.dimensions.iter().any(|dimension| {
        dimension.dimension == ApplicabilityDimension::Revision
            && dimension.status == DimensionStatus::Matched
            && dimension.required == SNAPSHOT_MAIN
            && dimension.actual.as_deref() == Some(SNAPSHOT_MAIN)
    }));
}

#[test]
fn provider_main_movement_invalidates_current_routing_snapshot() {
    let evaluation = evaluate_query(&revision_query(SNAPSHOT_MAIN, Some(ADVANCED_MAIN))).unwrap();

    assert_eq!(evaluation.status, ApplicabilityStatus::Invalid);
    assert!(evaluation.dimensions.iter().any(|dimension| {
        dimension.dimension == ApplicabilityDimension::Revision
            && dimension.status == DimensionStatus::Mismatched
            && dimension.required == SNAPSHOT_MAIN
            && dimension.actual.as_deref() == Some(ADVANCED_MAIN)
    }));
}

#[test]
fn missing_provider_current_revision_preserves_unknown() {
    let evaluation = evaluate_query(&revision_query(SNAPSHOT_MAIN, None)).unwrap();

    assert_eq!(evaluation.status, ApplicabilityStatus::Unknown);
    assert!(evaluation.dimensions.iter().any(|dimension| {
        dimension.dimension == ApplicabilityDimension::Revision
            && dimension.status == DimensionStatus::Missing
            && dimension.required == SNAPSHOT_MAIN
            && dimension.actual.is_none()
    }));
}

#[test]
fn provider_work_head_applies_even_when_checkout_uses_a_distinct_merge_view() {
    let provider_context = evaluate_query(&work_query(PR_HEAD, "#604", PR_HEAD, "#604")).unwrap();
    assert_eq!(provider_context.status, ApplicabilityStatus::Applies);

    let checkout_context =
        evaluate_query(&work_query(PR_HEAD, "#604", SYNTHETIC_MERGE_VIEW, "#604")).unwrap();
    assert_eq!(checkout_context.status, ApplicabilityStatus::Invalid);
    assert!(checkout_context.dimensions.iter().any(|dimension| {
        dimension.dimension == ApplicabilityDimension::Revision
            && dimension.status == DimensionStatus::Mismatched
            && dimension.required == PR_HEAD
            && dimension.actual.as_deref() == Some(SYNTHETIC_MERGE_VIEW)
    }));
}

#[test]
fn provider_current_context_does_not_need_a_wall_clock_freshness_rule() {
    let same_coordinate =
        evaluate_query(&revision_query(ADVANCED_MAIN, Some(ADVANCED_MAIN))).unwrap();
    let moved_coordinate =
        evaluate_query(&revision_query(SNAPSHOT_MAIN, Some(ADVANCED_MAIN))).unwrap();

    assert_eq!(same_coordinate.status, ApplicabilityStatus::Applies);
    assert_eq!(moved_coordinate.status, ApplicabilityStatus::Invalid);
}
