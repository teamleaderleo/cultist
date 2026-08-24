use std::error::Error;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::discriminator_observation::{
    DiscriminatorObservationBatch, validate_discriminator_observation_batch,
};
use crate::observation_frontier::{
    OBSERVATION_FRONTIER_SCHEMA_VERSION, ObservationFrontierReceipt, ObservationFrontierRequest,
    ObservationFrontierStatus, ObservationRequirement, evaluate_observation_frontiers,
};
use crate::refinement_episode::{
    RefinementEpisodeBatch, RefinementStatus, ReplayResult, validate_refinement_episode_batch,
};
use crate::refinement_observation_requirement::{
    REFINEMENT_OBSERVATION_REQUIREMENT_SCHEMA_VERSION, RefinementObservationRequirementBatch,
    RefinementObservationRequirementMapping, RefinementObservationRequirementRequest,
    evaluate_selected_observation_requirements,
};

pub const REFINEMENT_CANDIDATE_READINESS_SCHEMA_VERSION: u32 = 1;
pub const MAX_REFINEMENT_CANDIDATE_READINESS_REQUEST_BYTES: usize = 1024 * 1024;

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RefinementCandidateReadinessRequest {
    pub schema_version: u32,
    pub refinements: RefinementEpisodeBatch,
    pub mappings: RefinementObservationRequirementBatch,
    pub observations: DiscriminatorObservationBatch,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CandidateEvidenceStatus {
    Current,
    Blocked,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RefinementCandidateReadiness {
    pub episode_id: String,
    pub candidate_id: String,
    pub is_selected_transition: bool,
    pub replay_status: RefinementStatus,
    pub replay_result: ReplayResult,
    pub evidence_status: CandidateEvidenceStatus,
    pub requirements: Vec<ObservationRequirement>,
    pub requirement_mappings: Vec<RefinementObservationRequirementMapping>,
    pub requirement_frontiers: Vec<ObservationFrontierReceipt>,
    pub missing_requirement_mappings: Vec<String>,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RefinementCandidateReadinessEvaluation {
    pub schema_version: u32,
    pub candidates: Vec<RefinementCandidateReadiness>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct RefinementCandidateReadinessError {
    message: String,
}

impl RefinementCandidateReadinessError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for RefinementCandidateReadinessError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for RefinementCandidateReadinessError {}

pub fn parse_refinement_candidate_readiness_request(
    bytes: &[u8],
) -> Result<RefinementCandidateReadinessRequest, RefinementCandidateReadinessError> {
    if bytes.len() > MAX_REFINEMENT_CANDIDATE_READINESS_REQUEST_BYTES {
        return Err(RefinementCandidateReadinessError::new(format!(
            "refinement candidate readiness request exceeds the {MAX_REFINEMENT_CANDIDATE_READINESS_REQUEST_BYTES}-byte limit"
        )));
    }
    let request: RefinementCandidateReadinessRequest =
        serde_json::from_slice(bytes).map_err(|error| {
            RefinementCandidateReadinessError::new(format!(
                "invalid refinement candidate readiness JSON: {error}"
            ))
        })?;
    validate_request(&request)?;
    Ok(request)
}

pub fn evaluate_refinement_candidate_readiness(
    request: &RefinementCandidateReadinessRequest,
) -> Result<RefinementCandidateReadinessEvaluation, RefinementCandidateReadinessError> {
    validate_request(request)?;

    let mut candidates = Vec::new();
    for episode in &request.refinements.episodes {
        for candidate in &episode.candidate_refinements {
            let mut requirements = Vec::new();
            let mut requirement_mappings = Vec::new();
            let mut missing_requirement_mappings = Vec::new();

            for discriminator_id in &candidate.discriminator_refs {
                if let Some(mapping) = request.mappings.mappings.iter().find(|mapping| {
                    mapping.episode_id == episode.id
                        && mapping.candidate_id == candidate.id
                        && mapping.discriminator_id == *discriminator_id
                }) {
                    requirements.push(ObservationRequirement {
                        discriminator_id: discriminator_id.clone(),
                        subject_ref: mapping.subject_ref.clone(),
                    });
                    requirement_mappings.push(mapping.clone());
                } else {
                    missing_requirement_mappings.push(discriminator_id.clone());
                }
            }

            requirements.sort();
            requirement_mappings.sort_by(|left, right| left.id.cmp(&right.id));
            missing_requirement_mappings.sort();

            let requirement_frontiers = if requirements.is_empty() {
                Vec::new()
            } else {
                evaluate_observation_frontiers(&ObservationFrontierRequest {
                    schema_version: OBSERVATION_FRONTIER_SCHEMA_VERSION,
                    requirements: requirements.clone(),
                    observations: request.observations.clone(),
                })
                .map_err(|error| {
                    RefinementCandidateReadinessError::new(format!(
                        "candidate {} frontier evaluation failed: {error}",
                        candidate.id
                    ))
                })?
                .frontiers
            };

            let evidence_status = if missing_requirement_mappings.is_empty()
                && !requirement_frontiers.is_empty()
                && requirement_frontiers
                    .iter()
                    .all(|frontier| frontier.status == ObservationFrontierStatus::Current)
            {
                CandidateEvidenceStatus::Current
            } else {
                CandidateEvidenceStatus::Blocked
            };

            candidates.push(RefinementCandidateReadiness {
                episode_id: episode.id.clone(),
                candidate_id: candidate.id.clone(),
                is_selected_transition: episode.selected_transition.as_deref()
                    == Some(candidate.id.as_str()),
                replay_status: candidate.status,
                replay_result: candidate.replay_result,
                evidence_status,
                requirements,
                requirement_mappings,
                requirement_frontiers,
                missing_requirement_mappings,
            });
        }
    }

    candidates.sort_by(|left, right| {
        left.episode_id
            .cmp(&right.episode_id)
            .then_with(|| left.candidate_id.cmp(&right.candidate_id))
    });

    Ok(RefinementCandidateReadinessEvaluation {
        schema_version: REFINEMENT_CANDIDATE_READINESS_SCHEMA_VERSION,
        candidates,
    })
}

fn validate_request(
    request: &RefinementCandidateReadinessRequest,
) -> Result<(), RefinementCandidateReadinessError> {
    if request.schema_version != REFINEMENT_CANDIDATE_READINESS_SCHEMA_VERSION {
        return Err(RefinementCandidateReadinessError::new(format!(
            "unsupported refinement candidate readiness schema {}; expected {REFINEMENT_CANDIDATE_READINESS_SCHEMA_VERSION}",
            request.schema_version
        )));
    }

    validate_refinement_episode_batch(&request.refinements).map_err(|error| {
        RefinementCandidateReadinessError::new(format!("invalid refinement episode batch: {error}"))
    })?;

    // Reuse #231 as the owning validator for mapping identity, uniqueness, and
    // candidate/discriminator membership before this composition reads mappings.
    let mapping_request = RefinementObservationRequirementRequest {
        schema_version: REFINEMENT_OBSERVATION_REQUIREMENT_SCHEMA_VERSION,
        refinements: request.refinements.clone(),
        mappings: request.mappings.clone(),
    };
    evaluate_selected_observation_requirements(&mapping_request).map_err(|error| {
        RefinementCandidateReadinessError::new(format!(
            "invalid refinement observation requirement mappings: {error}"
        ))
    })?;

    validate_discriminator_observation_batch(&request.observations).map_err(|error| {
        RefinementCandidateReadinessError::new(format!(
            "invalid discriminator observation batch: {error}"
        ))
    })?;

    Ok(())
}
