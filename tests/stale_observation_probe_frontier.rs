#![allow(dead_code)]

#[path = "../src/applicability.rs"]
mod applicability;
#[path = "../src/discriminator_observation.rs"]
mod discriminator_observation;
#[path = "../src/durable_obligation.rs"]
mod durable_obligation;
#[path = "../src/evidence_planner.rs"]
mod evidence_planner;
#[path = "../src/justification.rs"]
mod justification;
#[path = "../src/observation_frontier.rs"]
mod observation_frontier;
#[path = "../src/observation_probe_bridge.rs"]
mod observation_probe_bridge;

use applicability::{
    APPLICABILITY_SCHEMA_VERSION, ApplicabilityQuery, ApplicabilityStatus, EvaluationContext,
    EvidenceRequirements, evaluate_query,
};
use discriminator_observation::{
    DISCRIMINATOR_OBSERVATION_SCHEMA_VERSION, DiscriminatorObservation,
    DiscriminatorObservationBatch, DiscriminatorValueState, ObservationApplicability,
    ObservationApplicabilityStatus,
};
use evidence_planner::ProbeSelectionPolicy;
use observation_frontier::{
    OBSERVATION_FRONTIER_SCHEMA_VERSION, ObservationFrontierRequest, ObservationFrontierStatus,
    ObservationRequirement, evaluate_observation_frontiers,
};
use observation_probe_bridge::{
    OBSERVATION_PROBE_BRIDGE_SCHEMA_VERSION, ObservationProbePlanRequest,
    ObservationProbePlanStatus, plan_observation_probe,
};

fn requirements() -> EvidenceRequirements {
    EvidenceRequirements {
        repository: Some("owner/repo".to_string()),
        revision: Some("head-a".to_string()),
        work: None,
        scope: None,
    }
}

fn context(revision: Option<&str>) -> EvaluationContext {
    EvaluationContext {
        repository: Some("owner/repo".to_string()),
        revision: revision.map(str::to_string),
        work: None,
        path: None,
    }
}

fn shared_applicability(revision: Option<&str>) -> ApplicabilityStatus {
    evaluate_query(&ApplicabilityQuery {
        schema_version: APPLICABILITY_SCHEMA_VERSION,
        requirements: requirements(),
        context: context(revision),
    })
    .unwrap()
    .status
}

fn current_frontier_produced_at_head_a() -> observation_frontier::ObservationFrontierReceipt {
    assert_eq!(
        shared_applicability(Some("head-a")),
        ApplicabilityStatus::Applies
    );

    let request = ObservationFrontierRequest {
        schema_version: OBSERVATION_FRONTIER_SCHEMA_VERSION,
        requirements: vec![ObservationRequirement {
            discriminator_id: "edit_class".to_string(),
            subject_ref: "path:src/lib.rs".to_string(),
        }],
        observations: DiscriminatorObservationBatch {
            schema_version: DISCRIMINATOR_OBSERVATION_SCHEMA_VERSION,
            observations: vec![DiscriminatorObservation {
                observation_id: "obs:edit-class:head-a".to_string(),
                discriminator_id: "edit_class".to_string(),
                subject_ref: "path:src/lib.rs".to_string(),
                source_receipt: "source:edit-class:head-a".to_string(),
                value_state: DiscriminatorValueState::Known {
                    value_ref: "syntax_changed".to_string(),
                },
                applicability: ObservationApplicability {
                    status: ObservationApplicabilityStatus::Applies,
                    receipt_ref: "applicability:owner/repo:head-a:applies".to_string(),
                },
            }],
        },
    };

    let frontier = evaluate_observation_frontiers(&request)
        .unwrap()
        .frontiers
        .remove(0);
    assert_eq!(frontier.status, ObservationFrontierStatus::Current);
    frontier
}

fn consume_frontier(
    frontier: observation_frontier::ObservationFrontierReceipt,
    revision: Option<&str>,
) -> observation_probe_bridge::ObservationProbePlan {
    plan_observation_probe(&ObservationProbePlanRequest {
        schema_version: OBSERVATION_PROBE_BRIDGE_SCHEMA_VERSION,
        frontier,
        bridges: Vec::new(),
        context: context(revision),
        probes: Vec::new(),
        allow_effectful: false,
        policy: ProbeSelectionPolicy::Conservative,
    })
    .unwrap()
}

#[test]
fn current_frontier_short_circuit_outlives_moved_consumption_revision() {
    let frontier = current_frontier_produced_at_head_a();

    assert_eq!(
        shared_applicability(Some("head-b")),
        ApplicabilityStatus::Invalid
    );

    let plan = consume_frontier(frontier, Some("head-b"));
    assert_eq!(plan.frontier_status, ObservationFrontierStatus::Current);
    assert_eq!(plan.status, ObservationProbePlanStatus::AlreadyCurrent);
    assert!(plan.evidence_plan.is_none());
}

#[test]
fn current_frontier_short_circuit_outlives_missing_consumption_revision() {
    let frontier = current_frontier_produced_at_head_a();

    assert_eq!(shared_applicability(None), ApplicabilityStatus::Unknown);

    let plan = consume_frontier(frontier, None);
    assert_eq!(plan.frontier_status, ObservationFrontierStatus::Current);
    assert_eq!(plan.status, ObservationProbePlanStatus::AlreadyCurrent);
    assert!(plan.evidence_plan.is_none());
}
