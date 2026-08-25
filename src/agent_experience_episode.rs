use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

use serde::{Deserialize, Serialize};

pub const AGENT_EXPERIENCE_SCHEMA_VERSION: u32 = 1;
pub const MAX_AGENT_EXPERIENCE_BATCH_BYTES: usize = 512 * 1024;
const MAX_EPISODES: usize = 128;
const MAX_ITEMS: usize = 256;
const MAX_ID_BYTES: usize = 512;
const MAX_TEXT_BYTES: usize = 16 * 1024;

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AgentExperienceBatch {
    pub schema_version: u32,
    pub episodes: Vec<AgentExperienceEpisode>,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AgentExperienceEpisode {
    pub id: String,
    pub repository: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revision: Option<String>,
    pub work: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failure_class: Option<String>,
    pub roles: Vec<ExperienceRole>,
    pub discriminators: Vec<ExperienceDiscriminator>,
    pub evidence_refs: Vec<String>,
    #[serde(default)]
    pub counterexample_refs: Vec<String>,
    #[serde(default)]
    pub interventions: Vec<OperatorIntervention>,
    pub lessons: Vec<ExperienceLesson>,
    #[serde(default)]
    pub persistence: Vec<PersistenceArtifact>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost: Option<ExactCost>,
    #[serde(default)]
    pub behavioral_evaluation_refs: Vec<String>,
    pub validated_repositories: Vec<String>,
    pub automatic_policy_authority: bool,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExperienceRole {
    ContextBriefDefect,
    EnvironmentDefect,
    WorkerCapabilityDefect,
    ReviewMiss,
    IntegrationOnlyDefect,
    RejectedLesson,
    PromotedDeterministicCheck,
    CounterexampleToRoutingHeuristic,
    CrossRepositoryReusableTechnique,
    OperatorIntervention,
    BehavioralNullResult,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExperienceDiscriminator {
    pub id: String,
    pub kind: DiscriminatorKind,
    pub key: String,
    pub value: String,
    pub source_ref: String,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DiscriminatorKind {
    Applicability,
    Outcome,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OperatorIntervention {
    pub actor: String,
    pub action: String,
    pub source_ref: String,
    pub outcome: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExperienceLesson {
    pub id: String,
    pub statement: String,
    pub status: LessonStatus,
    pub discriminator_refs: Vec<String>,
    #[serde(default)]
    pub counterexample_refs: Vec<String>,
    #[serde(default)]
    pub persistence_refs: Vec<String>,
    pub source_refs: Vec<String>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LessonStatus {
    Candidate,
    Retained,
    Weakened,
    Rejected,
    Promoted,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PersistenceArtifact {
    pub id: String,
    pub kind: PersistenceKind,
    pub reference: String,
    pub effect: String,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PersistenceKind {
    ResearchReceipt,
    BriefContract,
    ReviewCue,
    DeterministicCheck,
    CounterexampleTest,
    DecisionRecord,
    OperatingGuidance,
    ReusableTechnique,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExactCost {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_tokens: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_tokens: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning_tokens: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hosted_ci_runs: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repair_turns: Option<u64>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct AgentExperienceError {
    message: String,
}

impl AgentExperienceError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for AgentExperienceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for AgentExperienceError {}

pub fn parse_agent_experience_batch(
    bytes: &[u8],
) -> Result<AgentExperienceBatch, AgentExperienceError> {
    if bytes.len() > MAX_AGENT_EXPERIENCE_BATCH_BYTES {
        return Err(AgentExperienceError::new(format!(
            "agent experience batch exceeds the {MAX_AGENT_EXPERIENCE_BATCH_BYTES}-byte limit"
        )));
    }

    let batch: AgentExperienceBatch = serde_json::from_slice(bytes).map_err(|error| {
        AgentExperienceError::new(format!("invalid agent experience JSON: {error}"))
    })?;
    validate_agent_experience_batch(&batch)?;
    Ok(batch)
}

pub fn validate_agent_experience_batch(
    batch: &AgentExperienceBatch,
) -> Result<(), AgentExperienceError> {
    if batch.schema_version != AGENT_EXPERIENCE_SCHEMA_VERSION {
        return Err(AgentExperienceError::new(format!(
            "unsupported agent experience schema {}; expected {AGENT_EXPERIENCE_SCHEMA_VERSION}",
            batch.schema_version
        )));
    }
    if batch.episodes.is_empty() || batch.episodes.len() > MAX_EPISODES {
        return Err(AgentExperienceError::new(
            "agent experience batch must contain a bounded non-empty episode set",
        ));
    }

    let mut ids = BTreeSet::new();
    for episode in &batch.episodes {
        validate_episode(episode)?;
        if !ids.insert(episode.id.as_str()) {
            return Err(AgentExperienceError::new(format!(
                "duplicate agent experience episode id {}",
                episode.id
            )));
        }
    }
    Ok(())
}

fn validate_episode(episode: &AgentExperienceEpisode) -> Result<(), AgentExperienceError> {
    validate_atom(&episode.id, "episode id", MAX_ID_BYTES)?;
    validate_repository(&episode.repository, "repository")?;
    if let Some(revision) = &episode.revision {
        validate_revision(revision)?;
    }
    validate_text(&episode.work, "work", MAX_TEXT_BYTES)?;
    if let Some(failure_class) = &episode.failure_class {
        validate_atom(failure_class, "failure_class", MAX_ID_BYTES)?;
    }

    validate_reference_set(&episode.evidence_refs, "evidence_refs", false)?;
    validate_reference_set(&episode.counterexample_refs, "counterexample_refs", true)?;
    validate_reference_set(
        &episode.behavioral_evaluation_refs,
        "behavioral_evaluation_refs",
        true,
    )?;

    if episode.automatic_policy_authority {
        return Err(AgentExperienceError::new(
            "agent experience episodes cannot grant automatic policy authority",
        ));
    }

    if episode.roles.is_empty() || episode.roles.len() > MAX_ITEMS {
        return Err(AgentExperienceError::new(
            "roles must be bounded and non-empty",
        ));
    }
    if episode.roles.iter().copied().collect::<BTreeSet<_>>().len() != episode.roles.len() {
        return Err(AgentExperienceError::new("roles must be unique"));
    }

    if episode.discriminators.is_empty() || episode.discriminators.len() > MAX_ITEMS {
        return Err(AgentExperienceError::new(
            "discriminators must be bounded and non-empty",
        ));
    }
    let mut discriminators = BTreeMap::new();
    for discriminator in &episode.discriminators {
        validate_atom(&discriminator.id, "discriminator id", MAX_ID_BYTES)?;
        validate_atom(&discriminator.key, "discriminator key", MAX_ID_BYTES)?;
        validate_text(&discriminator.value, "discriminator value", MAX_TEXT_BYTES)?;
        validate_text(
            &discriminator.source_ref,
            "discriminator source_ref",
            MAX_TEXT_BYTES,
        )?;
        if discriminators
            .insert(discriminator.id.as_str(), discriminator)
            .is_some()
        {
            return Err(AgentExperienceError::new(format!(
                "duplicate discriminator id {}",
                discriminator.id
            )));
        }
    }

    if episode.interventions.len() > MAX_ITEMS {
        return Err(AgentExperienceError::new("too many interventions"));
    }
    for intervention in &episode.interventions {
        validate_text(&intervention.actor, "intervention actor", MAX_TEXT_BYTES)?;
        validate_text(&intervention.action, "intervention action", MAX_TEXT_BYTES)?;
        validate_text(
            &intervention.source_ref,
            "intervention source_ref",
            MAX_TEXT_BYTES,
        )?;
        validate_text(
            &intervention.outcome,
            "intervention outcome",
            MAX_TEXT_BYTES,
        )?;
    }

    if episode.persistence.len() > MAX_ITEMS {
        return Err(AgentExperienceError::new("too many persistence artifacts"));
    }
    let mut persistence = BTreeMap::new();
    for artifact in &episode.persistence {
        validate_atom(&artifact.id, "persistence id", MAX_ID_BYTES)?;
        validate_text(&artifact.reference, "persistence reference", MAX_TEXT_BYTES)?;
        validate_text(&artifact.effect, "persistence effect", MAX_TEXT_BYTES)?;
        if persistence.insert(artifact.id.as_str(), artifact).is_some() {
            return Err(AgentExperienceError::new(format!(
                "duplicate persistence id {}",
                artifact.id
            )));
        }
    }

    if episode.lessons.is_empty() || episode.lessons.len() > MAX_ITEMS {
        return Err(AgentExperienceError::new(
            "lessons must be bounded and non-empty",
        ));
    }
    let mut lesson_ids = BTreeSet::new();
    for lesson in &episode.lessons {
        validate_lesson(lesson, &discriminators, &persistence)?;
        if !lesson_ids.insert(lesson.id.as_str()) {
            return Err(AgentExperienceError::new(format!(
                "duplicate lesson id {}",
                lesson.id
            )));
        }
    }

    validate_repository_set(&episode.validated_repositories, "validated_repositories")?;
    if !episode
        .validated_repositories
        .iter()
        .any(|repository| repository == &episode.repository)
    {
        return Err(AgentExperienceError::new(
            "validated_repositories must include the episode repository",
        ));
    }

    if let Some(cost) = &episode.cost
        && cost.input_tokens.is_none()
        && cost.output_tokens.is_none()
        && cost.reasoning_tokens.is_none()
        && cost.hosted_ci_runs.is_none()
        && cost.repair_turns.is_none()
    {
        return Err(AgentExperienceError::new(
            "cost must preserve at least one exact observed quantity",
        ));
    }

    validate_role_contracts(episode, &persistence)
}

fn validate_lesson(
    lesson: &ExperienceLesson,
    discriminators: &BTreeMap<&str, &ExperienceDiscriminator>,
    persistence: &BTreeMap<&str, &PersistenceArtifact>,
) -> Result<(), AgentExperienceError> {
    validate_atom(&lesson.id, "lesson id", MAX_ID_BYTES)?;
    validate_text(&lesson.statement, "lesson statement", MAX_TEXT_BYTES)?;
    validate_reference_set(&lesson.source_refs, "lesson source_refs", false)?;
    validate_reference_set(
        &lesson.counterexample_refs,
        "lesson counterexample_refs",
        true,
    )?;

    if lesson.discriminator_refs.is_empty() || lesson.discriminator_refs.len() > MAX_ITEMS {
        return Err(AgentExperienceError::new(format!(
            "lesson {} must name bounded applicability discriminators",
            lesson.id
        )));
    }
    let mut seen_discriminators = BTreeSet::new();
    for discriminator_ref in &lesson.discriminator_refs {
        validate_atom(discriminator_ref, "lesson discriminator_ref", MAX_ID_BYTES)?;
        if !discriminators.contains_key(discriminator_ref.as_str()) {
            return Err(AgentExperienceError::new(format!(
                "lesson {} references unknown discriminator {}",
                lesson.id, discriminator_ref
            )));
        }
        if !seen_discriminators.insert(discriminator_ref.as_str()) {
            return Err(AgentExperienceError::new(format!(
                "lesson {} repeats discriminator {}",
                lesson.id, discriminator_ref
            )));
        }
    }
    if !lesson.discriminator_refs.iter().any(|reference| {
        discriminators
            .get(reference.as_str())
            .is_some_and(|discriminator| discriminator.kind == DiscriminatorKind::Applicability)
    }) {
        return Err(AgentExperienceError::new(format!(
            "lesson {} must name at least one applicability discriminator",
            lesson.id
        )));
    }

    if lesson.persistence_refs.len() > MAX_ITEMS {
        return Err(AgentExperienceError::new(format!(
            "lesson {} has too many persistence_refs",
            lesson.id
        )));
    }
    let mut seen_persistence = BTreeSet::new();
    for persistence_ref in &lesson.persistence_refs {
        validate_atom(persistence_ref, "lesson persistence_ref", MAX_ID_BYTES)?;
        if !persistence.contains_key(persistence_ref.as_str()) {
            return Err(AgentExperienceError::new(format!(
                "lesson {} references unknown persistence artifact {}",
                lesson.id, persistence_ref
            )));
        }
        if !seen_persistence.insert(persistence_ref.as_str()) {
            return Err(AgentExperienceError::new(format!(
                "lesson {} repeats persistence artifact {}",
                lesson.id, persistence_ref
            )));
        }
    }

    if lesson.status == LessonStatus::Promoted && lesson.persistence_refs.is_empty() {
        return Err(AgentExperienceError::new(format!(
            "promoted lesson {} must name an explicit persistence artifact",
            lesson.id
        )));
    }
    Ok(())
}

fn validate_role_contracts(
    episode: &AgentExperienceEpisode,
    persistence: &BTreeMap<&str, &PersistenceArtifact>,
) -> Result<(), AgentExperienceError> {
    let has_role = |role| episode.roles.contains(&role);

    if has_role(ExperienceRole::OperatorIntervention) && episode.interventions.is_empty() {
        return Err(AgentExperienceError::new(
            "operator_intervention role requires an intervention receipt",
        ));
    }
    if !has_role(ExperienceRole::OperatorIntervention) && !episode.interventions.is_empty() {
        return Err(AgentExperienceError::new(
            "intervention receipts require the operator_intervention role",
        ));
    }

    if has_role(ExperienceRole::RejectedLesson)
        && !episode
            .lessons
            .iter()
            .any(|lesson| lesson.status == LessonStatus::Rejected)
    {
        return Err(AgentExperienceError::new(
            "rejected_lesson role requires a rejected candidate lesson",
        ));
    }

    if has_role(ExperienceRole::PromotedDeterministicCheck) {
        let deterministic_ids = persistence
            .iter()
            .filter_map(|(id, artifact)| {
                (artifact.kind == PersistenceKind::DeterministicCheck).then_some(*id)
            })
            .collect::<BTreeSet<_>>();
        if deterministic_ids.is_empty() {
            return Err(AgentExperienceError::new(
                "promoted_deterministic_check role requires a deterministic_check artifact",
            ));
        }
        let promoted_lesson_points_to_check = episode.lessons.iter().any(|lesson| {
            lesson.status == LessonStatus::Promoted
                && lesson
                    .persistence_refs
                    .iter()
                    .any(|reference| deterministic_ids.contains(reference.as_str()))
        });
        if !promoted_lesson_points_to_check {
            return Err(AgentExperienceError::new(
                "promoted_deterministic_check role requires a promoted lesson to reference the check",
            ));
        }
    }

    if has_role(ExperienceRole::CounterexampleToRoutingHeuristic) {
        if episode.counterexample_refs.is_empty() {
            return Err(AgentExperienceError::new(
                "routing heuristic counterexample requires counterexample_refs",
            ));
        }
        if !episode.lessons.iter().any(|lesson| {
            matches!(
                lesson.status,
                LessonStatus::Weakened | LessonStatus::Rejected
            )
        }) {
            return Err(AgentExperienceError::new(
                "routing heuristic counterexample requires a weakened or rejected lesson",
            ));
        }
    }

    if has_role(ExperienceRole::CrossRepositoryReusableTechnique) {
        let repositories = episode
            .validated_repositories
            .iter()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        if repositories.len() < 2 {
            return Err(AgentExperienceError::new(
                "cross-repository reusable technique requires at least two validated repositories",
            ));
        }
        if !episode
            .persistence
            .iter()
            .any(|artifact| artifact.kind == PersistenceKind::ReusableTechnique)
        {
            return Err(AgentExperienceError::new(
                "cross-repository reusable technique requires a reusable_technique artifact",
            ));
        }
    }

    if has_role(ExperienceRole::BehavioralNullResult)
        && episode.behavioral_evaluation_refs.is_empty()
    {
        return Err(AgentExperienceError::new(
            "behavioral_null_result role requires behavioral_evaluation_refs",
        ));
    }

    Ok(())
}

fn validate_reference_set(
    values: &[String],
    field: &str,
    allow_empty: bool,
) -> Result<(), AgentExperienceError> {
    if (!allow_empty && values.is_empty()) || values.len() > MAX_ITEMS {
        return Err(AgentExperienceError::new(format!(
            "{field} must be bounded{}",
            if allow_empty { "" } else { " and non-empty" }
        )));
    }
    let mut seen = BTreeSet::new();
    for value in values {
        validate_text(value, field, MAX_TEXT_BYTES)?;
        if !seen.insert(value.as_str()) {
            return Err(AgentExperienceError::new(format!(
                "{field} must not contain duplicates"
            )));
        }
    }
    Ok(())
}

fn validate_repository_set(values: &[String], field: &str) -> Result<(), AgentExperienceError> {
    if values.is_empty() || values.len() > MAX_ITEMS {
        return Err(AgentExperienceError::new(format!(
            "{field} must be bounded and non-empty"
        )));
    }
    let mut seen = BTreeSet::new();
    for value in values {
        validate_repository(value, field)?;
        if !seen.insert(value.as_str()) {
            return Err(AgentExperienceError::new(format!(
                "{field} must not contain duplicates"
            )));
        }
    }
    Ok(())
}

fn validate_repository(value: &str, field: &str) -> Result<(), AgentExperienceError> {
    validate_atom(value, field, MAX_ID_BYTES)?;
    let mut components = value.split('/');
    let owner = components.next().unwrap_or_default();
    let repository = components.next().unwrap_or_default();
    if owner.is_empty() || repository.is_empty() || components.next().is_some() {
        return Err(AgentExperienceError::new(format!(
            "{field} must use owner/repository syntax"
        )));
    }
    Ok(())
}

fn validate_revision(value: &str) -> Result<(), AgentExperienceError> {
    if value.len() != 40 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(AgentExperienceError::new(
            "revision must be an exact 40-hex Git revision",
        ));
    }
    Ok(())
}

fn validate_atom(value: &str, field: &str, max_bytes: usize) -> Result<(), AgentExperienceError> {
    validate_text(value, field, max_bytes)?;
    if value.chars().any(char::is_whitespace) {
        return Err(AgentExperienceError::new(format!(
            "{field} must be a whitespace-free atom"
        )));
    }
    Ok(())
}

fn validate_text(value: &str, field: &str, max_bytes: usize) -> Result<(), AgentExperienceError> {
    if value.is_empty() || value.len() > max_bytes || value.trim() != value {
        return Err(AgentExperienceError::new(format!(
            "{field} must be non-empty, trimmed, and at most {max_bytes} bytes"
        )));
    }
    if value.chars().any(|character| {
        character == '\0' || (character.is_control() && character != '\n' && character != '\t')
    }) {
        return Err(AgentExperienceError::new(format!(
            "{field} contains unsupported control characters"
        )));
    }
    Ok(())
}
