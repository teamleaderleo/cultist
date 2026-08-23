#![allow(dead_code)]

#[path = "../src/discriminator_observation.rs"]
mod discriminator_observation;
#[path = "../src/observation_frontier.rs"]
mod observation_frontier;
#[path = "../src/refinement_episode.rs"]
mod refinement_episode;
#[path = "../src/refinement_observation_requirement.rs"]
mod refinement_observation_requirement;

use discriminator_observation::{DiscriminatorValueState, parse_discriminator_observation_batch};
use observation_frontier::{
    OBSERVATION_FRONTIER_SCHEMA_VERSION, ObservationFrontierRequest, ObservationFrontierStatus,
    evaluate_observation_frontiers,
};
use refinement_episode::parse_refinement_episode_batch;
use refinement_observation_requirement::{
    MAX_REFINEMENT_OBSERVATION_REQUIREMENT_REQUEST_BYTES,
    REFINEMENT_OBSERVATION_REQUIREMENT_SCHEMA_VERSION, RefinementObservationRequirementBatch,
    RefinementObservationRequirementMapping, RefinementObservationRequirementRequest,
    evaluate_selected_observation_requirements, parse_refinement_observation_requirement_request,
};

const OBSERVATIONS: &[u8] =
    include_bytes!("../research/discriminator-observations/cultist-v1.json");
const FOCUSED_OXC_OBSERVATION: &[u8] = include_bytes!(
    "../research/refinement-observation-requirements/oxc-focused-edit-class-v1.json"
);
const REFINEMENTS: &[u8] = include_bytes!("../research/refinement-episodes/cultist-v1.json");
const MAPPINGS: &[u8] =
    include_bytes!("../research/refinement-observation-requirements/cultist-v1.json");

fn request() -> RefinementObservationRequirementRequest {
    RefinementObservationRequirementRequest {
        schema_version: REFINEMENT_OBSERVATION_REQUIREMENT_SCHEMA_VERSION,
        refinements: parse_refinement_episode_batch(REFINEMENTS).unwrap(),
        mappings: serde_json::from_slice(MAPPINGS).unwrap(),
    }
}

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

#[test]
fn retained_selected_candidates_compile_exact_observation_requirements() {
    let evaluation = evaluate_selected_observation_requirements(&request()).unwrap();
    assert_eq!(evaluation.selected.len(), 3);
    assert!(
        evaluation
            .selected
            .iter()
            .all(|selected| selected.missing_discriminator_refs.is_empty())
    );

    let justification = evaluation
        .selected
        .iter()
        .find(|selected| selected.episode_id == "justification/open-obligation-v1")
        .unwrap();
    assert_eq!(justification.requirements.len(), 1);
    assert_eq!(
        justification.requirements[0].subject_ref,
        "refinement:justification/open-obligation-v1"
    );

    let oxc = evaluation
        .selected
        .iter()
        .find(|selected| selected.episode_id == "history/oxc-edit-class-v1")
        .unwrap();
    assert_eq!(oxc.candidate_id, "syntax-changing-current-cohort");
    assert_eq!(oxc.requirements.len(), 1);
    assert_eq!(oxc.requirements[0].discriminator_id, "edit_class");
    assert_eq!(
        oxc.requirements[0].subject_ref,
        "oxc-project/oxc@228e8e0f85c0e7aeded02c5e27fd810004d3b41a:crates/oxc_linter/src/rules.rs"
    );

    let project_memory = evaluation
        .selected
        .iter()
        .find(|selected| selected.episode_id == "project-memory/primary-case-contract-collision-v1")
        .unwrap();
    assert_eq!(project_memory.requirements.len(), 2);
    assert!(project_memory.requirements.iter().all(|requirement| {
        requirement.subject_ref == "refinement:project-memory/primary-case-contract-collision-v1"
    }));
}

#[test]
fn compiled_selected_requirements_are_current_in_retained_observation_corpus() {
    let observations = observations_with_focused_oxc();
    let evaluation = evaluate_selected_observation_requirements(&request()).unwrap();

    for selected in evaluation.selected {
        let frontiers = evaluate_observation_frontiers(&ObservationFrontierRequest {
            schema_version: OBSERVATION_FRONTIER_SCHEMA_VERSION,
            requirements: selected.requirements,
            observations: observations.clone(),
        })
        .unwrap();
        assert!(
            frontiers
                .frontiers
                .iter()
                .all(|frontier| frontier.status == ObservationFrontierStatus::Current),
            "selected refinement {} has a noncurrent exact requirement",
            selected.episode_id
        );
    }
}

#[test]
fn wrong_subject_edit_class_cannot_satisfy_compiled_oxc_requirement() {
    let mut observations = observations_with_focused_oxc();
    let exact = observations
        .observations
        .iter_mut()
        .find(|observation| observation.discriminator_id == "edit_class")
        .unwrap();
    exact.observation_id = "history/oxc-edit-class-v1:wrong-subject-repair-control".to_string();
    exact.subject_ref =
        "oxc-project/oxc@228e8e0f85c0e7aeded02c5e27fd810004d3b41a:crates/oxc_linter/src/other.rs"
            .to_string();
    exact.source_receipt = "research:wrong-subject-repair-control".to_string();
    exact.value_state = DiscriminatorValueState::Known {
        value_ref: "syntax_changed".to_string(),
    };

    let compiled = evaluate_selected_observation_requirements(&request()).unwrap();
    let oxc = compiled
        .selected
        .into_iter()
        .find(|selected| selected.episode_id == "history/oxc-edit-class-v1")
        .unwrap();
    let frontiers = evaluate_observation_frontiers(&ObservationFrontierRequest {
        schema_version: OBSERVATION_FRONTIER_SCHEMA_VERSION,
        requirements: oxc.requirements,
        observations,
    })
    .unwrap();
    assert_eq!(
        frontiers.frontiers[0].status,
        ObservationFrontierStatus::Missing
    );
    assert_eq!(frontiers.frontiers[0].other_subject.len(), 1);
}

#[test]
fn missing_candidate_subject_mapping_stays_explicit() {
    let mut request = request();
    request.mappings.mappings.retain(|mapping| {
        !(mapping.episode_id == "history/oxc-edit-class-v1"
            && mapping.candidate_id == "syntax-changing-current-cohort"
            && mapping.discriminator_id == "edit_class")
    });

    let evaluation = evaluate_selected_observation_requirements(&request).unwrap();
    let oxc = evaluation
        .selected
        .iter()
        .find(|selected| selected.episode_id == "history/oxc-edit-class-v1")
        .unwrap();
    assert!(oxc.requirements.is_empty());
    assert!(oxc.mappings.is_empty());
    assert_eq!(oxc.missing_discriminator_refs, vec!["edit_class"]);
}

#[test]
fn mapping_for_discriminator_not_required_by_candidate_rejects() {
    let mut request = request();
    request
        .mappings
        .mappings
        .push(RefinementObservationRequirementMapping {
            id: "invalid-extra-discriminator".to_string(),
            episode_id: "history/oxc-edit-class-v1".to_string(),
            candidate_id: "syntax-changing-current-cohort".to_string(),
            discriminator_id: "commit_identity".to_string(),
            subject_ref: "commit:irrelevant".to_string(),
            source_receipt: "research:invalid-control".to_string(),
        });
    let error = evaluate_selected_observation_requirements(&request).unwrap_err();
    assert!(error.to_string().contains("is not required by candidate"));
}

#[test]
fn duplicate_candidate_discriminator_mapping_rejects() {
    let mut request = request();
    let original = request
        .mappings
        .mappings
        .iter()
        .find(|mapping| mapping.episode_id == "history/oxc-edit-class-v1")
        .unwrap()
        .clone();
    let mut duplicate = original;
    duplicate.id = "duplicate-oxc-edit-class".to_string();
    duplicate.subject_ref = "commit:another-subject".to_string();
    request.mappings.mappings.push(duplicate);
    let error = evaluate_selected_observation_requirements(&request).unwrap_err();
    assert!(error.to_string().contains("multiple subject mappings"));
}

#[test]
fn same_discriminator_can_have_candidate_specific_subject_mapping() {
    let mut request = request();
    request
        .mappings
        .mappings
        .push(RefinementObservationRequirementMapping {
            id: "history/oxc-edit-class-v1:reverse-control:edit-class".to_string(),
            episode_id: "history/oxc-edit-class-v1".to_string(),
            candidate_id: "reverse-edit-class-control".to_string(),
            discriminator_id: "edit_class".to_string(),
            subject_ref: "oxc-project/oxc:reverse-generated-history".to_string(),
            source_receipt: "research:rust-syntax-cohort-replay.md#reverse-control".to_string(),
        });

    let evaluation = evaluate_selected_observation_requirements(&request).unwrap();
    let oxc = evaluation
        .selected
        .iter()
        .find(|selected| selected.episode_id == "history/oxc-edit-class-v1")
        .unwrap();
    assert_eq!(oxc.candidate_id, "syntax-changing-current-cohort");
    assert_eq!(
        oxc.requirements[0].subject_ref,
        "oxc-project/oxc@228e8e0f85c0e7aeded02c5e27fd810004d3b41a:crates/oxc_linter/src/rules.rs"
    );
}

#[test]
fn request_round_trip_and_byte_bound_are_explicit() {
    let request = request();
    let encoded = serde_json::to_vec(&request).unwrap();
    let decoded = parse_refinement_observation_requirement_request(&encoded).unwrap();
    assert_eq!(decoded, request);

    let oversized = vec![b' '; MAX_REFINEMENT_OBSERVATION_REQUIREMENT_REQUEST_BYTES + 1];
    let error = parse_refinement_observation_requirement_request(&oversized).unwrap_err();
    assert!(error.to_string().contains("exceeds"));
}

#[test]
fn empty_mapping_batch_is_valid_and_exposes_every_selected_requirement_as_missing() {
    let mut request = request();
    request.mappings = RefinementObservationRequirementBatch {
        schema_version: REFINEMENT_OBSERVATION_REQUIREMENT_SCHEMA_VERSION,
        mappings: Vec::new(),
    };
    let evaluation = evaluate_selected_observation_requirements(&request).unwrap();
    assert_eq!(evaluation.selected.len(), 3);
    assert!(evaluation.selected.iter().all(|selected| {
        selected.requirements.is_empty()
            && selected.mappings.is_empty()
            && !selected.missing_discriminator_refs.is_empty()
    }));
}
