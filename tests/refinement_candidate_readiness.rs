#![allow(dead_code)]

#[path = "../src/discriminator_observation.rs"]
mod discriminator_observation;
#[path = "../src/observation_frontier.rs"]
mod observation_frontier;
#[path = "../src/refinement_candidate_readiness.rs"]
mod refinement_candidate_readiness;
#[path = "../src/refinement_episode.rs"]
mod refinement_episode;
#[path = "../src/refinement_observation_requirement.rs"]
mod refinement_observation_requirement;

use discriminator_observation::{
    DISCRIMINATOR_OBSERVATION_SCHEMA_VERSION, DiscriminatorObservation, DiscriminatorValueState,
    ObservationApplicability, ObservationApplicabilityStatus,
    parse_discriminator_observation_batch,
};
use observation_frontier::ObservationFrontierStatus;
use refinement_candidate_readiness::{
    CandidateEvidenceStatus, MAX_REFINEMENT_CANDIDATE_READINESS_REQUEST_BYTES,
    REFINEMENT_CANDIDATE_READINESS_SCHEMA_VERSION, RefinementCandidateReadinessRequest,
    evaluate_refinement_candidate_readiness, parse_refinement_candidate_readiness_request,
};
use refinement_episode::{RefinementStatus, parse_refinement_episode_batch};
use refinement_observation_requirement::{
    REFINEMENT_OBSERVATION_REQUIREMENT_SCHEMA_VERSION, RefinementObservationRequirementBatch,
    RefinementObservationRequirementMapping,
};

const OBSERVATIONS: &[u8] =
    include_bytes!("../research/discriminator-observations/cultist-v1.json");
const FOCUSED_OXC_OBSERVATION: &[u8] = include_bytes!(
    "../research/refinement-observation-requirements/oxc-focused-edit-class-v1.json"
);
const REFINEMENTS: &[u8] = include_bytes!("../research/refinement-episodes/cultist-v1.json");
const MAPPINGS: &[u8] =
    include_bytes!("../research/refinement-observation-requirements/cultist-v1.json");

fn observations_with_focused_oxc() -> discriminator_observation::DiscriminatorObservationBatch {
    let mut observations = parse_discriminator_observation_batch(OBSERVATIONS).unwrap();
    observations
        .observations
        .retain(|observation| observation.discriminator_id != "edit_class");
    let focused = parse_discriminator_observation_batch(FOCUSED_OXC_OBSERVATION).unwrap();
    assert_eq!(focused.observations.len(), 1);
    observations.observations.extend(focused.observations);
    observations
}

fn request() -> RefinementCandidateReadinessRequest {
    RefinementCandidateReadinessRequest {
        schema_version: REFINEMENT_CANDIDATE_READINESS_SCHEMA_VERSION,
        refinements: parse_refinement_episode_batch(REFINEMENTS).unwrap(),
        mappings: serde_json::from_slice(MAPPINGS).unwrap(),
        observations: observations_with_focused_oxc(),
    }
}

fn current_observation(
    observation_id: &str,
    discriminator_id: &str,
    subject_ref: &str,
    value_ref: &str,
) -> DiscriminatorObservation {
    DiscriminatorObservation {
        observation_id: observation_id.to_string(),
        discriminator_id: discriminator_id.to_string(),
        subject_ref: subject_ref.to_string(),
        source_receipt: format!("research:candidate-readiness:{observation_id}"),
        value_state: DiscriminatorValueState::Known {
            value_ref: value_ref.to_string(),
        },
        applicability: ObservationApplicability {
            status: ObservationApplicabilityStatus::Applies,
            receipt_ref: format!("research:candidate-readiness:{observation_id}:applies"),
        },
    }
}

fn add_rejected_oxc_candidate_evidence(request: &mut RefinementCandidateReadinessRequest) {
    let reverse_subject = "refinement:history/oxc-edit-class-v1/reverse-edit-class-control";
    request
        .mappings
        .mappings
        .push(RefinementObservationRequirementMapping {
            id: "history/oxc-edit-class-v1:reverse-edit-class-control:edit-class".to_string(),
            episode_id: "history/oxc-edit-class-v1".to_string(),
            candidate_id: "reverse-edit-class-control".to_string(),
            discriminator_id: "edit_class".to_string(),
            subject_ref: reverse_subject.to_string(),
            source_receipt: "research:candidate-readiness:reverse-subject-control".to_string(),
        });
    request.observations.observations.push(current_observation(
        "candidate-readiness:reverse-edit-class",
        "edit_class",
        reverse_subject,
        "syntax_changed",
    ));

    let singleton_subject = "refinement:history/oxc-edit-class-v1/singleton-commit-partition";
    request
        .mappings
        .mappings
        .push(RefinementObservationRequirementMapping {
            id: "history/oxc-edit-class-v1:singleton-commit-partition:commit-identity".to_string(),
            episode_id: "history/oxc-edit-class-v1".to_string(),
            candidate_id: "singleton-commit-partition".to_string(),
            discriminator_id: "commit_identity".to_string(),
            subject_ref: singleton_subject.to_string(),
            source_receipt: "research:candidate-readiness:singleton-subject-control".to_string(),
        });
    request.observations.observations.push(current_observation(
        "candidate-readiness:singleton-commit-identity",
        "commit_identity",
        singleton_subject,
        "228e8e0f85c0e7aeded02c5e27fd810004d3b41a",
    ));
}

#[test]
fn current_evidence_does_not_rescue_replay_rejected_oxc_candidates() {
    let mut request = request();
    add_rejected_oxc_candidate_evidence(&mut request);
    let evaluation = evaluate_refinement_candidate_readiness(&request).unwrap();
    let oxc = evaluation
        .candidates
        .iter()
        .filter(|candidate| candidate.episode_id == "history/oxc-edit-class-v1")
        .collect::<Vec<_>>();
    assert_eq!(oxc.len(), 3);

    let selected = oxc
        .iter()
        .copied()
        .find(|candidate| candidate.candidate_id == "syntax-changing-current-cohort")
        .unwrap();
    assert_eq!(selected.replay_status, RefinementStatus::Weakened);
    assert_eq!(selected.evidence_status, CandidateEvidenceStatus::Current);
    assert!(selected.is_selected_transition);

    let reverse = oxc
        .iter()
        .copied()
        .find(|candidate| candidate.candidate_id == "reverse-edit-class-control")
        .unwrap();
    assert_eq!(
        reverse.replay_status,
        RefinementStatus::RejectedNoImprovement
    );
    assert_eq!(reverse.evidence_status, CandidateEvidenceStatus::Current);
    assert!(!reverse.is_selected_transition);
    assert_eq!(reverse.requirement_frontiers.len(), 1);
    assert_eq!(
        reverse.requirement_frontiers[0].status,
        ObservationFrontierStatus::Current
    );

    let singleton = oxc
        .iter()
        .copied()
        .find(|candidate| candidate.candidate_id == "singleton-commit-partition")
        .unwrap();
    assert_eq!(singleton.replay_status, RefinementStatus::RejectedOverfit);
    assert_eq!(singleton.evidence_status, CandidateEvidenceStatus::Current);
    assert!(!singleton.is_selected_transition);
    assert_eq!(singleton.requirement_frontiers.len(), 1);
    assert_eq!(
        singleton.requirement_frontiers[0].status,
        ObservationFrontierStatus::Current
    );
}

#[test]
fn replay_survivor_stays_evidence_blocked_when_exact_observation_is_missing() {
    let mut request = request();
    let exact = request
        .observations
        .observations
        .iter()
        .find(|observation| {
            observation.discriminator_id == "edit_class"
                && observation.subject_ref
                    == "oxc-project/oxc@228e8e0f85c0e7aeded02c5e27fd810004d3b41a:crates/oxc_linter/src/rules.rs"
        })
        .unwrap()
        .clone();
    request
        .observations
        .observations
        .retain(|observation| observation.observation_id != exact.observation_id);
    request.observations.observations.push(current_observation(
        "candidate-readiness:wrong-subject-edit-class",
        "edit_class",
        "oxc-project/oxc@228e8e0f85c0e7aeded02c5e27fd810004d3b41a:crates/oxc_linter/src/other.rs",
        "syntax_changed",
    ));

    let evaluation = evaluate_refinement_candidate_readiness(&request).unwrap();
    let selected = evaluation
        .candidates
        .iter()
        .find(|candidate| {
            candidate.episode_id == "history/oxc-edit-class-v1"
                && candidate.candidate_id == "syntax-changing-current-cohort"
        })
        .unwrap();
    assert_eq!(selected.replay_status, RefinementStatus::Weakened);
    assert_eq!(selected.evidence_status, CandidateEvidenceStatus::Blocked);
    assert!(selected.is_selected_transition);
    assert!(selected.missing_requirement_mappings.is_empty());
    assert_eq!(selected.requirement_frontiers.len(), 1);
    assert_eq!(
        selected.requirement_frontiers[0].status,
        ObservationFrontierStatus::Missing
    );
    assert_eq!(selected.requirement_frontiers[0].other_subject.len(), 1);
}

#[test]
fn missing_exact_subject_mapping_blocks_without_changing_replay_status() {
    let mut request = request();
    request.mappings.mappings.retain(|mapping| {
        !(mapping.episode_id == "history/oxc-edit-class-v1"
            && mapping.candidate_id == "syntax-changing-current-cohort"
            && mapping.discriminator_id == "edit_class")
    });

    let evaluation = evaluate_refinement_candidate_readiness(&request).unwrap();
    let selected = evaluation
        .candidates
        .iter()
        .find(|candidate| {
            candidate.episode_id == "history/oxc-edit-class-v1"
                && candidate.candidate_id == "syntax-changing-current-cohort"
        })
        .unwrap();
    assert_eq!(selected.replay_status, RefinementStatus::Weakened);
    assert_eq!(selected.evidence_status, CandidateEvidenceStatus::Blocked);
    assert!(selected.requirements.is_empty());
    assert!(selected.requirement_frontiers.is_empty());
    assert_eq!(selected.missing_requirement_mappings, vec!["edit_class"]);
}

#[test]
fn retained_unmapped_rejected_candidates_are_explicitly_blocked() {
    let evaluation = evaluate_refinement_candidate_readiness(&request()).unwrap();
    let reverse = evaluation
        .candidates
        .iter()
        .find(|candidate| {
            candidate.episode_id == "history/oxc-edit-class-v1"
                && candidate.candidate_id == "reverse-edit-class-control"
        })
        .unwrap();
    assert_eq!(
        reverse.replay_status,
        RefinementStatus::RejectedNoImprovement
    );
    assert_eq!(reverse.evidence_status, CandidateEvidenceStatus::Blocked);
    assert_eq!(reverse.missing_requirement_mappings, vec!["edit_class"]);

    let singleton = evaluation
        .candidates
        .iter()
        .find(|candidate| {
            candidate.episode_id == "history/oxc-edit-class-v1"
                && candidate.candidate_id == "singleton-commit-partition"
        })
        .unwrap();
    assert_eq!(singleton.replay_status, RefinementStatus::RejectedOverfit);
    assert_eq!(singleton.evidence_status, CandidateEvidenceStatus::Blocked);
    assert_eq!(
        singleton.missing_requirement_mappings,
        vec!["commit_identity"]
    );
}

#[test]
fn all_retained_candidates_keep_their_replay_receipts() {
    let evaluation = evaluate_refinement_candidate_readiness(&request()).unwrap();
    let source_candidates = parse_refinement_episode_batch(REFINEMENTS)
        .unwrap()
        .episodes
        .into_iter()
        .flat_map(|episode| {
            let episode_id = episode.id;
            episode
                .candidate_refinements
                .into_iter()
                .map(move |candidate| (episode_id.clone(), candidate))
        })
        .collect::<Vec<_>>();
    assert_eq!(evaluation.candidates.len(), source_candidates.len());
    for readiness in &evaluation.candidates {
        let (_, source) = source_candidates
            .iter()
            .find(|(episode_id, candidate)| {
                episode_id == &readiness.episode_id && candidate.id == readiness.candidate_id
            })
            .unwrap();
        assert_eq!(readiness.replay_status, source.status);
        assert_eq!(readiness.replay_result, source.replay_result);
    }
}

#[test]
fn request_round_trip_and_byte_bound_are_explicit() {
    let request = request();
    let encoded = serde_json::to_vec(&request).unwrap();
    let decoded = parse_refinement_candidate_readiness_request(&encoded).unwrap();
    assert_eq!(decoded, request);

    let oversized = vec![b' '; MAX_REFINEMENT_CANDIDATE_READINESS_REQUEST_BYTES + 1];
    let error = parse_refinement_candidate_readiness_request(&oversized).unwrap_err();
    assert!(error.to_string().contains("exceeds"));
}

#[test]
fn malformed_mapping_is_rejected_by_the_existing_requirement_validator() {
    let mut request = request();
    request.mappings = RefinementObservationRequirementBatch {
        schema_version: REFINEMENT_OBSERVATION_REQUIREMENT_SCHEMA_VERSION,
        mappings: request.mappings.mappings,
    };
    request
        .mappings
        .mappings
        .push(RefinementObservationRequirementMapping {
            id: "candidate-readiness:bad-mapping".to_string(),
            episode_id: "history/oxc-edit-class-v1".to_string(),
            candidate_id: "syntax-changing-current-cohort".to_string(),
            discriminator_id: "commit_identity".to_string(),
            subject_ref: "research:bad-subject".to_string(),
            source_receipt: "research:bad-mapping".to_string(),
        });
    let error = evaluate_refinement_candidate_readiness(&request).unwrap_err();
    assert!(error.to_string().contains("is not required by candidate"));
}

#[test]
fn observation_schema_is_still_v2() {
    let request = request();
    assert_eq!(
        request.observations.schema_version,
        DISCRIMINATOR_OBSERVATION_SCHEMA_VERSION
    );
    assert_eq!(
        request.mappings.schema_version,
        REFINEMENT_OBSERVATION_REQUIREMENT_SCHEMA_VERSION
    );
}
