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

use applicability::{ApplicabilityStatus, EvaluationContext, EvidenceRequirements};
use durable_obligation::DiscriminatorKey;
use evidence_planner::{
    EvidencePlanStatus, EvidenceProbe, ProbeCandidateStatus, ProbeCost, ProbeEffect,
    ProbeSelectionPolicy,
};
use observation_frontier::{
    CurrentObservationReceipt, NonCurrentObservationReceipt, ObservationFrontierReceipt,
    ObservationFrontierStatus,
};
use observation_probe_bridge::{
    MAX_OBSERVATION_PROBE_PLAN_REQUEST_BYTES, OBSERVATION_PROBE_BRIDGE_SCHEMA_VERSION,
    ObservationProbeBridge, ObservationProbePlanRequest, ObservationProbePlanStatus,
    parse_observation_probe_plan_request, plan_observation_probe,
};

fn requirements(revision: Option<&str>) -> EvidenceRequirements {
    EvidenceRequirements {
        repository: Some("owner/repo".to_string()),
        revision: revision.map(str::to_string),
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

fn probe_key() -> DiscriminatorKey {
    DiscriminatorKey {
        kind: "source_edit_class".to_string(),
        target: "commit:subject-a".to_string(),
    }
}

fn probe(
    id: &str,
    key: DiscriminatorKey,
    revision: Option<&str>,
    effect: ProbeEffect,
    cost: ProbeCost,
) -> EvidenceProbe {
    EvidenceProbe {
        id: id.to_string(),
        produces: key,
        requirements: requirements(revision),
        effect,
        cost,
    }
}

fn bridge(subject_ref: &str, revision: &str) -> ObservationProbeBridge {
    ObservationProbeBridge {
        bridge_id: format!("edit-class-{revision}"),
        observation_discriminator_id: "edit_class".to_string(),
        observation_subject_ref: subject_ref.to_string(),
        probe_discriminator: probe_key(),
        clearing_requirements: requirements(Some(revision)),
        source_receipt: format!("source-adapter:edit-class:{revision}"),
    }
}

fn missing_frontier(subject_ref: &str) -> ObservationFrontierReceipt {
    ObservationFrontierReceipt {
        discriminator_id: "edit_class".to_string(),
        subject_ref: subject_ref.to_string(),
        status: ObservationFrontierStatus::Missing,
        current: Vec::new(),
        unknown: Vec::new(),
        invalid: Vec::new(),
        other_subject: Vec::new(),
    }
}

fn unknown_frontier(subject_ref: &str) -> ObservationFrontierReceipt {
    ObservationFrontierReceipt {
        discriminator_id: "edit_class".to_string(),
        subject_ref: subject_ref.to_string(),
        status: ObservationFrontierStatus::Unknown,
        current: Vec::new(),
        unknown: vec![NonCurrentObservationReceipt {
            observation_id: "obs:unknown".to_string(),
            source_receipt: "source:unknown".to_string(),
            known_value_ref: None,
            value_unknown_reason_ref: Some("classifier:missing-value".to_string()),
            applicability_ref: "applicability:head-a:applies".to_string(),
        }],
        invalid: Vec::new(),
        other_subject: Vec::new(),
    }
}

fn invalid_frontier(subject_ref: &str) -> ObservationFrontierReceipt {
    ObservationFrontierReceipt {
        discriminator_id: "edit_class".to_string(),
        subject_ref: subject_ref.to_string(),
        status: ObservationFrontierStatus::Invalid,
        current: Vec::new(),
        unknown: Vec::new(),
        invalid: vec![NonCurrentObservationReceipt {
            observation_id: "obs:old-head".to_string(),
            source_receipt: "source:old-head".to_string(),
            known_value_ref: Some("syntax_changed".to_string()),
            value_unknown_reason_ref: None,
            applicability_ref: "applicability:old-head->head-b:invalid".to_string(),
        }],
        other_subject: Vec::new(),
    }
}

fn current_frontier(subject_ref: &str) -> ObservationFrontierReceipt {
    ObservationFrontierReceipt {
        discriminator_id: "edit_class".to_string(),
        subject_ref: subject_ref.to_string(),
        status: ObservationFrontierStatus::Current,
        current: vec![CurrentObservationReceipt {
            observation_id: "obs:current".to_string(),
            source_receipt: "source:current".to_string(),
            value_ref: "syntax_changed".to_string(),
            applicability_ref: "applicability:head-a:applies".to_string(),
        }],
        unknown: Vec::new(),
        invalid: Vec::new(),
        other_subject: Vec::new(),
    }
}

fn request(
    frontier: ObservationFrontierReceipt,
    bridges: Vec<ObservationProbeBridge>,
    context: EvaluationContext,
    probes: Vec<EvidenceProbe>,
    allow_effectful: bool,
) -> ObservationProbePlanRequest {
    ObservationProbePlanRequest {
        schema_version: OBSERVATION_PROBE_BRIDGE_SCHEMA_VERSION,
        frontier,
        frontier_requirements: requirements(Some("head-a")),
        bridges,
        context,
        probes,
        allow_effectful,
        policy: ProbeSelectionPolicy::Conservative,
    }
}

#[test]
fn exact_source_bridge_selects_capable_probe_and_rejects_similar_name_as_incapable() {
    let subject = "commit:subject-a";
    let plan = plan_observation_probe(&request(
        missing_frontier(subject),
        vec![bridge(subject, "head-a")],
        context(Some("head-a")),
        vec![
            probe(
                "cheap-similar-name",
                DiscriminatorKey {
                    kind: "edit_class".to_string(),
                    target: subject.to_string(),
                },
                Some("head-a"),
                ProbeEffect::ReadOnly,
                ProbeCost::default(),
            ),
            probe(
                "mapped-source-probe",
                probe_key(),
                Some("head-a"),
                ProbeEffect::ReadOnly,
                ProbeCost {
                    git_subprocesses: 1,
                    ..ProbeCost::default()
                },
            ),
        ],
        false,
    ))
    .unwrap();

    assert_eq!(plan.status, ObservationProbePlanStatus::Planned);
    assert_eq!(plan.applicability_status, ApplicabilityStatus::Applies);
    let evidence = plan.evidence_plan.unwrap();
    assert_eq!(evidence.status, EvidencePlanStatus::Selected);
    assert_eq!(evidence.selected.unwrap().id, "mapped-source-probe");
    assert_eq!(
        evidence
            .candidates
            .iter()
            .find(|candidate| candidate.id == "cheap-similar-name")
            .unwrap()
            .status,
        ProbeCandidateStatus::Incapable
    );
}

#[test]
fn similarly_named_probe_without_explicit_bridge_leaves_frontier_unmapped() {
    let subject = "commit:subject-a";
    let plan = plan_observation_probe(&request(
        missing_frontier(subject),
        Vec::new(),
        context(Some("head-a")),
        vec![probe(
            "looks-related",
            DiscriminatorKey {
                kind: "edit_class".to_string(),
                target: subject.to_string(),
            },
            Some("head-a"),
            ProbeEffect::ReadOnly,
            ProbeCost::default(),
        )],
        false,
    ))
    .unwrap();

    assert_eq!(plan.status, ObservationProbePlanStatus::NoAdmittedMapping);
    assert_eq!(plan.applicability_status, ApplicabilityStatus::Applies);
    assert!(plan.evidence_plan.is_none());
}

#[test]
fn bridge_for_wrong_subject_does_not_map_required_frontier() {
    let plan = plan_observation_probe(&request(
        missing_frontier("commit:subject-a"),
        vec![bridge("commit:subject-b", "head-a")],
        context(Some("head-a")),
        vec![probe(
            "mapped-source-probe",
            probe_key(),
            Some("head-a"),
            ProbeEffect::ReadOnly,
            ProbeCost::default(),
        )],
        false,
    ))
    .unwrap();

    assert_eq!(plan.status, ObservationProbePlanStatus::NoAdmittedMapping);
    assert_eq!(plan.applicability_status, ApplicabilityStatus::Applies);
}

#[test]
fn current_frontier_needs_no_acquisition_plan_while_applicability_still_applies() {
    let subject = "commit:subject-a";
    let plan = plan_observation_probe(&request(
        current_frontier(subject),
        vec![bridge(subject, "head-a")],
        context(Some("head-a")),
        vec![probe(
            "mapped-source-probe",
            probe_key(),
            Some("head-a"),
            ProbeEffect::ReadOnly,
            ProbeCost::default(),
        )],
        false,
    ))
    .unwrap();

    assert_eq!(plan.status, ObservationProbePlanStatus::AlreadyCurrent);
    assert_eq!(plan.applicability_status, ApplicabilityStatus::Applies);
    assert!(plan.evidence_plan.is_none());
}

#[test]
fn current_frontier_with_moved_context_can_plan_refresh() {
    let subject = "commit:subject-a";
    let plan = plan_observation_probe(&request(
        current_frontier(subject),
        vec![bridge(subject, "head-b")],
        context(Some("head-b")),
        vec![probe(
            "refresh-current-head",
            probe_key(),
            Some("head-b"),
            ProbeEffect::ReadOnly,
            ProbeCost::default(),
        )],
        false,
    ))
    .unwrap();

    assert_eq!(plan.frontier_status, ObservationFrontierStatus::Current);
    assert_eq!(plan.applicability_status, ApplicabilityStatus::Invalid);
    assert_eq!(plan.status, ObservationProbePlanStatus::Planned);
    assert_eq!(
        plan.evidence_plan.unwrap().selected.unwrap().id,
        "refresh-current-head"
    );
}

#[test]
fn unknown_value_with_current_coordinate_can_plan_deeper_acquisition() {
    let subject = "commit:subject-a";
    let plan = plan_observation_probe(&request(
        unknown_frontier(subject),
        vec![bridge(subject, "head-a")],
        context(Some("head-a")),
        vec![probe(
            "mapped-source-probe",
            probe_key(),
            Some("head-a"),
            ProbeEffect::ReadOnly,
            ProbeCost::default(),
        )],
        false,
    ))
    .unwrap();

    assert_eq!(plan.frontier_status, ObservationFrontierStatus::Unknown);
    assert_eq!(plan.applicability_status, ApplicabilityStatus::Applies);
    assert_eq!(
        plan.evidence_plan.unwrap().status,
        EvidencePlanStatus::Selected
    );
}

#[test]
fn invalid_old_value_can_plan_current_head_refresh_without_becoming_current() {
    let subject = "commit:subject-a";
    let plan = plan_observation_probe(&request(
        invalid_frontier(subject),
        vec![bridge(subject, "head-b")],
        context(Some("head-b")),
        vec![probe(
            "refresh-current-head",
            probe_key(),
            Some("head-b"),
            ProbeEffect::ReadOnly,
            ProbeCost::default(),
        )],
        false,
    ))
    .unwrap();

    assert_eq!(plan.frontier_status, ObservationFrontierStatus::Invalid);
    assert_eq!(plan.applicability_status, ApplicabilityStatus::Invalid);
    let evidence = plan.evidence_plan.unwrap();
    assert_eq!(evidence.status, EvidencePlanStatus::Selected);
    assert_eq!(evidence.selected.unwrap().id, "refresh-current-head");
}

#[test]
fn missing_current_coordinate_stays_blocked_in_existing_planner() {
    let subject = "commit:subject-a";
    let plan = plan_observation_probe(&request(
        unknown_frontier(subject),
        vec![bridge(subject, "head-a")],
        context(None),
        vec![probe(
            "mapped-source-probe",
            probe_key(),
            Some("head-a"),
            ProbeEffect::ReadOnly,
            ProbeCost::default(),
        )],
        false,
    ))
    .unwrap();

    assert_eq!(plan.applicability_status, ApplicabilityStatus::Unknown);
    let evidence = plan.evidence_plan.unwrap();
    assert_eq!(evidence.status, EvidencePlanStatus::Blocked);
    assert_eq!(
        evidence.candidates[0].status,
        ProbeCandidateStatus::MissingContext
    );
}

#[test]
fn bridge_grants_zero_effect_authority() {
    let subject = "commit:subject-a";
    let plan = plan_observation_probe(&request(
        missing_frontier(subject),
        vec![bridge(subject, "head-a")],
        context(Some("head-a")),
        vec![probe(
            "effectful-source-probe",
            probe_key(),
            Some("head-a"),
            ProbeEffect::Effectful,
            ProbeCost {
                effectful_executions: 1,
                ..ProbeCost::default()
            },
        )],
        false,
    ))
    .unwrap();

    let evidence = plan.evidence_plan.unwrap();
    assert_eq!(evidence.status, EvidencePlanStatus::Blocked);
    assert_eq!(
        evidence.candidates[0].status,
        ProbeCandidateStatus::EffectAuthorityRequired
    );
}

#[test]
fn duplicate_mapping_for_exact_observation_requirement_rejects() {
    let subject = "commit:subject-a";
    let mut duplicate = bridge(subject, "head-a");
    duplicate.bridge_id = "second-bridge".to_string();
    let error = plan_observation_probe(&request(
        missing_frontier(subject),
        vec![bridge(subject, "head-a"), duplicate],
        context(Some("head-a")),
        Vec::new(),
        false,
    ))
    .unwrap_err();

    assert!(error.to_string().contains("multiple admitted bridges"));
}

#[test]
fn incoherent_frontier_receipt_rejects_before_planning() {
    let mut frontier = missing_frontier("commit:subject-a");
    frontier.status = ObservationFrontierStatus::Current;
    let error = plan_observation_probe(&request(
        frontier,
        Vec::new(),
        context(Some("head-a")),
        Vec::new(),
        false,
    ))
    .unwrap_err();

    assert!(error.to_string().contains("status disagrees"));
}

#[test]
fn empty_frontier_requirements_fail_closed() {
    let subject = "commit:subject-a";
    let mut request = request(
        current_frontier(subject),
        Vec::new(),
        context(Some("head-a")),
        Vec::new(),
        false,
    );
    request.frontier_requirements = EvidenceRequirements::default();

    let error = plan_observation_probe(&request).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("at least one explicit applicability requirement")
    );
}

#[test]
fn request_round_trip_and_byte_bound_are_explicit() {
    let subject = "commit:subject-a";
    let request = request(
        missing_frontier(subject),
        vec![bridge(subject, "head-a")],
        context(Some("head-a")),
        vec![probe(
            "mapped-source-probe",
            probe_key(),
            Some("head-a"),
            ProbeEffect::ReadOnly,
            ProbeCost::default(),
        )],
        false,
    );
    let encoded = serde_json::to_vec(&request).unwrap();
    let decoded = parse_observation_probe_plan_request(&encoded).unwrap();
    assert_eq!(decoded, request);

    let oversized = vec![b' '; MAX_OBSERVATION_PROBE_PLAN_REQUEST_BYTES + 1];
    let error = parse_observation_probe_plan_request(&oversized).unwrap_err();
    assert!(error.to_string().contains("exceeds"));
}
