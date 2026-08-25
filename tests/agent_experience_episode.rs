#![allow(dead_code)]

use std::collections::BTreeSet;

#[path = "../src/agent_experience_episode.rs"]
mod agent_experience_episode;

use agent_experience_episode::{
    DiscriminatorKind, ExperienceRole, LessonStatus, MAX_AGENT_EXPERIENCE_BATCH_BYTES,
    PersistenceKind, parse_agent_experience_batch, validate_agent_experience_batch,
};

const CORPUS: &[u8] =
    include_bytes!("../research/agent-experience-episodes/sol-luna-dogfood-v1.json");

#[test]
fn retained_dogfood_corpus_preserves_distinct_experience_roles() {
    let batch = parse_agent_experience_batch(CORPUS).unwrap();
    assert_eq!(batch.episodes.len(), 9);

    let roles = batch
        .episodes
        .iter()
        .flat_map(|episode| episode.roles.iter().copied())
        .collect::<BTreeSet<_>>();

    for role in [
        ExperienceRole::ContextBriefDefect,
        ExperienceRole::EnvironmentDefect,
        ExperienceRole::WorkerCapabilityDefect,
        ExperienceRole::ReviewMiss,
        ExperienceRole::IntegrationOnlyDefect,
        ExperienceRole::RejectedLesson,
        ExperienceRole::PromotedDeterministicCheck,
        ExperienceRole::CounterexampleToRoutingHeuristic,
        ExperienceRole::CrossRepositoryReusableTechnique,
        ExperienceRole::OperatorIntervention,
        ExperienceRole::BehavioralNullResult,
    ] {
        assert!(roles.contains(&role), "missing retained role {role:?}");
    }
}

#[test]
fn required_command_episode_preserves_exact_cost_and_promoted_check() {
    let batch = parse_agent_experience_batch(CORPUS).unwrap();
    let episode = batch
        .episodes
        .iter()
        .find(|episode| episode.id == "stensibly/run-05a-required-command-preflight")
        .unwrap();

    assert_eq!(episode.cost.as_ref().unwrap().input_tokens, Some(632_503));
    assert!(episode.roles.contains(&ExperienceRole::EnvironmentDefect));
    assert!(
        episode
            .roles
            .contains(&ExperienceRole::PromotedDeterministicCheck)
    );
    assert!(episode.lessons.iter().any(|lesson| {
        lesson.id == "required-command-preflight" && lesson.status == LessonStatus::Promoted
    }));
    assert!(episode.persistence.iter().any(|artifact| {
        artifact.id == "require-command" && artifact.kind == PersistenceKind::DeterministicCheck
    }));
}

#[test]
fn integration_episode_stays_distinct_from_review_miss() {
    let batch = parse_agent_experience_batch(CORPUS).unwrap();
    let episode = batch
        .episodes
        .iter()
        .find(|episode| episode.id == "stensibly/run-01-integration-only-typecheck")
        .unwrap();

    assert!(episode.roles.contains(&ExperienceRole::EnvironmentDefect));
    assert!(
        episode
            .roles
            .contains(&ExperienceRole::IntegrationOnlyDefect)
    );
    assert!(!episode.roles.contains(&ExperienceRole::ReviewMiss));
    assert_eq!(episode.cost.as_ref().unwrap().repair_turns, Some(1));
    assert!(episode.discriminators.iter().any(|discriminator| {
        discriminator.id == "local-typecheck-unavailable"
            && discriminator.kind == DiscriminatorKind::Applicability
    }));
}

#[test]
fn palisade_audit_preserves_a_real_review_miss() {
    let batch = parse_agent_experience_batch(CORPUS).unwrap();
    let episode = batch
        .episodes
        .iter()
        .find(|episode| episode.id == "stensibly/pr1661-palisade-review-miss")
        .unwrap();

    assert!(episode.roles.contains(&ExperienceRole::ReviewMiss));
    assert!(!episode.roles.contains(&ExperienceRole::IntegrationOnlyDefect));
    assert!(episode.lessons.iter().any(|lesson| {
        lesson.id == "focused-review-proves-unattended-lifecycle"
            && lesson.status == LessonStatus::Rejected
    }));
    assert!(episode.discriminators.iter().any(|discriminator| {
        discriminator.id == "palisade-defects-found"
            && discriminator.kind == DiscriminatorKind::Outcome
            && discriminator.value == "4"
    }));
}

#[test]
fn rejected_and_weakened_lessons_survive_beside_promoted_ones() {
    let batch = parse_agent_experience_batch(CORPUS).unwrap();

    let capability = batch
        .episodes
        .iter()
        .find(|episode| episode.id == "stensibly/run-03-git-metadata-capability")
        .unwrap();
    assert!(capability.lessons.iter().any(|lesson| {
        lesson.id == "workspace-write-implies-git-write" && lesson.status == LessonStatus::Rejected
    }));
    assert!(capability.lessons.iter().any(|lesson| {
        lesson.id == "separate-git-authority" && lesson.status == LessonStatus::Promoted
    }));

    let routing = batch
        .episodes
        .iter()
        .find(|episode| episode.id == "stensibly/run-05-review-effort-counterexample")
        .unwrap();
    assert!(routing.lessons.iter().any(|lesson| {
        lesson.id == "risk-alone-routes-high" && lesson.status == LessonStatus::Weakened
    }));
    assert!(routing.lessons.iter().any(|lesson| {
        lesson.id == "ambiguity-discrimination-routing" && lesson.status == LessonStatus::Candidate
    }));
    assert_eq!(routing.cost.as_ref().unwrap().input_tokens, Some(632_503));
}

#[test]
fn operator_rejection_can_persist_as_counterexample_test_without_promotion() {
    let batch = parse_agent_experience_batch(CORPUS).unwrap();
    let episode = batch
        .episodes
        .iter()
        .find(|episode| episode.id == "stensibly/run-03h-combined-failure-rejected-proposal")
        .unwrap();

    assert!(
        episode
            .roles
            .contains(&ExperienceRole::OperatorIntervention)
    );
    assert!(episode.roles.contains(&ExperienceRole::RejectedLesson));
    assert_eq!(episode.interventions.len(), 1);
    assert!(
        episode
            .lessons
            .iter()
            .all(|lesson| { lesson.status != LessonStatus::Promoted })
    );
    assert!(
        episode
            .persistence
            .iter()
            .any(|artifact| { artifact.kind == PersistenceKind::CounterexampleTest })
    );
}

#[test]
fn cross_repository_technique_keeps_integration_counterexample() {
    let batch = parse_agent_experience_batch(CORPUS).unwrap();
    let episode = batch
        .episodes
        .iter()
        .find(|episode| episode.id == "portfolio/exact-head-independent-review-technique")
        .unwrap();

    assert_eq!(episode.validated_repositories.len(), 4);
    assert!(
        episode
            .roles
            .contains(&ExperienceRole::CrossRepositoryReusableTechnique)
    );
    assert!(!episode.counterexample_refs.is_empty());
}

#[test]
fn retained_luna_null_result_does_not_promote_treatment_equivalence() {
    let batch = parse_agent_experience_batch(CORPUS).unwrap();
    let episode = batch
        .episodes
        .iter()
        .find(|episode| episode.id == "cultist/luna-max-guard-detail-null")
        .unwrap();

    assert!(
        episode
            .roles
            .contains(&ExperienceRole::BehavioralNullResult)
    );
    assert!(!episode.behavioral_evaluation_refs.is_empty());
    assert!(episode.lessons.iter().any(|lesson| {
        lesson.id == "detail-always-changes-first-action" && lesson.status == LessonStatus::Rejected
    }));
    assert!(episode.lessons.iter().any(|lesson| {
        lesson.id == "test-leaner-control" && lesson.status == LessonStatus::Candidate
    }));
}

#[test]
fn automatic_policy_authority_is_rejected() {
    let mut batch = parse_agent_experience_batch(CORPUS).unwrap();
    batch.episodes[0].automatic_policy_authority = true;
    let error = validate_agent_experience_batch(&batch).unwrap_err();
    assert!(error.to_string().contains("automatic policy authority"));
}

#[test]
fn promoted_check_role_requires_a_real_deterministic_artifact() {
    let mut batch = parse_agent_experience_batch(CORPUS).unwrap();
    let episode = batch
        .episodes
        .iter_mut()
        .find(|episode| episode.id == "stensibly/run-05a-required-command-preflight")
        .unwrap();
    episode.persistence[0].kind = PersistenceKind::ResearchReceipt;

    let error = validate_agent_experience_batch(&batch).unwrap_err();
    assert!(error.to_string().contains("deterministic_check artifact"));
}

#[test]
fn lesson_requires_an_applicability_discriminator() {
    let mut batch = parse_agent_experience_batch(CORPUS).unwrap();
    let episode = batch
        .episodes
        .iter_mut()
        .find(|episode| episode.id == "cultist/luna-max-guard-detail-null")
        .unwrap();
    episode.lessons[0].discriminator_refs = vec!["same-first-action".to_string()];

    let error = validate_agent_experience_batch(&batch).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("at least one applicability discriminator")
    );
}

#[test]
fn cross_repository_role_requires_multiple_validated_repositories() {
    let mut batch = parse_agent_experience_batch(CORPUS).unwrap();
    let episode = batch
        .episodes
        .iter_mut()
        .find(|episode| episode.id == "portfolio/exact-head-independent-review-technique")
        .unwrap();
    episode.validated_repositories = vec!["teamleaderleo/stensibly".to_string()];

    let error = validate_agent_experience_batch(&batch).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("at least two validated repositories")
    );
}

#[test]
fn intervention_receipts_require_the_matching_role() {
    let mut batch = parse_agent_experience_batch(CORPUS).unwrap();
    batch.episodes[0]
        .roles
        .retain(|role| *role != ExperienceRole::OperatorIntervention);

    let error = validate_agent_experience_batch(&batch).unwrap_err();
    assert!(error.to_string().contains("intervention receipts require"));
}

#[test]
fn oversized_batch_fails_before_json_parsing() {
    let bytes = vec![b' '; MAX_AGENT_EXPERIENCE_BATCH_BYTES + 1];
    let error = parse_agent_experience_batch(&bytes).unwrap_err();
    assert!(error.to_string().contains("exceeds"));
}
