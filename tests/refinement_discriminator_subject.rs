#![allow(dead_code)]

use std::collections::BTreeMap;

#[path = "../src/discriminator_observation.rs"]
mod discriminator_observation;
#[path = "../src/observation_frontier.rs"]
mod observation_frontier;
#[path = "../src/refinement_episode.rs"]
mod refinement_episode;

use discriminator_observation::{
    DiscriminatorValueState, enumerate_discriminator_partitions,
    parse_discriminator_observation_batch,
};
use observation_frontier::{
    OBSERVATION_FRONTIER_SCHEMA_VERSION, ObservationFrontierRequest, ObservationFrontierStatus,
    ObservationRequirement, evaluate_observation_frontiers,
};
use refinement_episode::parse_refinement_episode_batch;

const OBSERVATIONS: &[u8] =
    include_bytes!("../research/discriminator-observations/cultist-v1.json");
const FOCUSED_OXC_OBSERVATION: &[u8] = include_bytes!(
    "../research/refinement-observation-requirements/oxc-focused-edit-class-v1.json"
);
const REFINEMENTS: &[u8] = include_bytes!("../research/refinement-episodes/cultist-v1.json");

fn focused_oxc_observation() -> discriminator_observation::DiscriminatorObservation {
    let batch = parse_discriminator_observation_batch(FOCUSED_OXC_OBSERVATION).unwrap();
    assert_eq!(batch.observations.len(), 1);
    batch.observations[0].clone()
}

#[test]
fn id_only_refinement_coverage_can_accept_wrong_subject_while_exact_frontier_is_missing() {
    let original = parse_discriminator_observation_batch(OBSERVATIONS).unwrap();
    let exact = focused_oxc_observation();
    assert_eq!(
        exact.subject_ref,
        "oxc-project/oxc@228e8e0f85c0e7aeded02c5e27fd810004d3b41a:crates/oxc_linter/src/rules.rs"
    );

    let refinements = parse_refinement_episode_batch(REFINEMENTS).unwrap();
    let episode = refinements
        .episodes
        .iter()
        .find(|episode| episode.id == "history/oxc-edit-class-v1")
        .expect("retained Oxc refinement episode");
    let selected_id = episode
        .selected_transition
        .as_ref()
        .expect("retained Oxc transition is selected");
    let selected = episode
        .candidate_refinements
        .iter()
        .find(|candidate| candidate.id == *selected_id)
        .expect("selected candidate exists");
    assert_eq!(selected.discriminator_refs, vec!["edit_class"]);

    let mut wrong_subject = exact.clone();
    wrong_subject.observation_id = "history/oxc-edit-class-v1:wrong-subject-control".to_string();
    wrong_subject.subject_ref =
        "oxc-project/oxc@228e8e0f85c0e7aeded02c5e27fd810004d3b41a:crates/oxc_linter/src/other.rs"
            .to_string();
    wrong_subject.source_receipt = "research:wrong-subject-control".to_string();
    wrong_subject.value_state = DiscriminatorValueState::Known {
        value_ref: "syntax_changed".to_string(),
    };

    let mut observations = original.clone();
    observations
        .observations
        .retain(|observation| observation.discriminator_id != "edit_class");
    observations.observations.push(wrong_subject);

    // This is the current #187 cross-object sufficiency species: availability
    // is collapsed to discriminator ID before selected candidate refs are checked.
    let enumeration = enumerate_discriminator_partitions(&observations).unwrap();
    let current_by_id = enumeration
        .discriminators
        .iter()
        .map(|discriminator| {
            (
                discriminator.discriminator_id.as_str(),
                !discriminator.known_partitions.is_empty(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    for discriminator_id in &selected.discriminator_refs {
        assert_eq!(
            current_by_id.get(discriminator_id.as_str()),
            Some(&true),
            "ID-only coverage currently treats any current subject as sufficient"
        );
    }

    // The exact v2 frontier disagrees: the earned Oxc subject has no current
    // observation, and the same discriminator on another subject stays visible.
    let evaluation = evaluate_observation_frontiers(&ObservationFrontierRequest {
        schema_version: OBSERVATION_FRONTIER_SCHEMA_VERSION,
        requirements: vec![ObservationRequirement {
            discriminator_id: "edit_class".to_string(),
            subject_ref: exact.subject_ref.clone(),
        }],
        observations,
    })
    .unwrap();
    let frontier = &evaluation.frontiers[0];
    assert_eq!(frontier.status, ObservationFrontierStatus::Missing);
    assert!(frontier.current.is_empty());
    assert_eq!(frontier.other_subject.len(), 1);
    assert_eq!(
        frontier.other_subject[0].subject_ref,
        "oxc-project/oxc@228e8e0f85c0e7aeded02c5e27fd810004d3b41a:crates/oxc_linter/src/other.rs"
    );
}

#[test]
fn exact_focused_subject_is_current_control() {
    let exact = focused_oxc_observation();
    let observations = discriminator_observation::DiscriminatorObservationBatch {
        schema_version: discriminator_observation::DISCRIMINATOR_OBSERVATION_SCHEMA_VERSION,
        observations: vec![exact.clone()],
    };
    let evaluation = evaluate_observation_frontiers(&ObservationFrontierRequest {
        schema_version: OBSERVATION_FRONTIER_SCHEMA_VERSION,
        requirements: vec![ObservationRequirement {
            discriminator_id: exact.discriminator_id.clone(),
            subject_ref: exact.subject_ref.clone(),
        }],
        observations,
    })
    .unwrap();
    assert_eq!(
        evaluation.frontiers[0].status,
        ObservationFrontierStatus::Current
    );
    assert_eq!(evaluation.frontiers[0].other_subject.len(), 0);
}
