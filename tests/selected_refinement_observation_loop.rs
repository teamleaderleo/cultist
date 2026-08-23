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
    REFINEMENT_OBSERVATION_REQUIREMENT_SCHEMA_VERSION, RefinementObservationRequirementRequest,
    evaluate_selected_observation_requirements,
};

const OBSERVATIONS: &[u8] =
    include_bytes!("../research/discriminator-observations/cultist-v1.json");
const FOCUSED_OXC_OBSERVATION: &[u8] = include_bytes!(
    "../research/refinement-observation-requirements/oxc-focused-edit-class-v1.json"
);
const REFINEMENTS: &[u8] = include_bytes!("../research/refinement-episodes/cultist-v1.json");
const MAPPINGS: &[u8] =
    include_bytes!("../research/refinement-observation-requirements/cultist-v1.json");

fn compiled_oxc_requirement() -> observation_frontier::ObservationRequirement {
    let request = RefinementObservationRequirementRequest {
        schema_version: REFINEMENT_OBSERVATION_REQUIREMENT_SCHEMA_VERSION,
        refinements: parse_refinement_episode_batch(REFINEMENTS).unwrap(),
        mappings: serde_json::from_slice(MAPPINGS).unwrap(),
    };
    let evaluation = evaluate_selected_observation_requirements(&request).unwrap();
    let oxc = evaluation
        .selected
        .iter()
        .find(|selected| selected.episode_id == "history/oxc-edit-class-v1")
        .expect("retained selected Oxc refinement");
    assert_eq!(oxc.candidate_id, "syntax-changing-current-cohort");
    assert!(oxc.missing_discriminator_refs.is_empty());
    assert_eq!(oxc.requirements.len(), 1);
    oxc.requirements[0].clone()
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
fn retained_selected_oxc_requirement_reaches_current_only_with_exact_subject_observation() {
    let requirement = compiled_oxc_requirement();
    assert_eq!(requirement.discriminator_id, "edit_class");
    assert_eq!(
        requirement.subject_ref,
        "oxc-project/oxc@228e8e0f85c0e7aeded02c5e27fd810004d3b41a:crates/oxc_linter/src/rules.rs"
    );

    let original = observations_with_focused_oxc();
    let exact = original
        .observations
        .iter()
        .find(|observation| {
            observation.discriminator_id == requirement.discriminator_id
                && observation.subject_ref == requirement.subject_ref
        })
        .expect("focused Oxc observation fixture")
        .clone();

    let mut withheld = original.clone();
    withheld
        .observations
        .retain(|observation| observation.observation_id != exact.observation_id);

    let mut wrong_path = exact.clone();
    wrong_path.observation_id = "selected-loop:wrong-path-edit-class".to_string();
    wrong_path.subject_ref =
        "oxc-project/oxc@228e8e0f85c0e7aeded02c5e27fd810004d3b41a:crates/oxc_linter/src/other.rs"
            .to_string();
    wrong_path.source_receipt = "research:selected-loop:wrong-path".to_string();
    withheld.observations.push(wrong_path);

    let mut pinned_head = exact.clone();
    pinned_head.observation_id = "selected-loop:pinned-head-control".to_string();
    pinned_head.subject_ref =
        "oxc-project/oxc@8783524015b1e6ff1c39ccf426df0bb07cbbc588:crates/oxc_linter/src/rules.rs"
            .to_string();
    pinned_head.source_receipt = "github-actions:run/32258599172#pinned-head-control".to_string();
    pinned_head.value_state = DiscriminatorValueState::Unknown {
        reason_ref: "rust-edit-class:anchor-unchanged:pinned-head-control".to_string(),
    };
    withheld.observations.push(pinned_head);

    let missing = evaluate_observation_frontiers(&ObservationFrontierRequest {
        schema_version: OBSERVATION_FRONTIER_SCHEMA_VERSION,
        requirements: vec![requirement.clone()],
        observations: withheld.clone(),
    })
    .unwrap();
    assert_eq!(missing.frontiers.len(), 1);
    assert_eq!(
        missing.frontiers[0].status,
        ObservationFrontierStatus::Missing
    );
    assert!(missing.frontiers[0].current.is_empty());
    assert_eq!(missing.frontiers[0].other_subject.len(), 2);

    let mut produced = withheld;
    produced.observations.push(exact);
    let current = evaluate_observation_frontiers(&ObservationFrontierRequest {
        schema_version: OBSERVATION_FRONTIER_SCHEMA_VERSION,
        requirements: vec![requirement],
        observations: produced,
    })
    .unwrap();
    assert_eq!(current.frontiers.len(), 1);
    assert_eq!(
        current.frontiers[0].status,
        ObservationFrontierStatus::Current
    );
    assert_eq!(current.frontiers[0].current.len(), 1);
    assert_eq!(current.frontiers[0].current[0].value_ref, "syntax_changed");
    assert_eq!(current.frontiers[0].other_subject.len(), 2);
}
