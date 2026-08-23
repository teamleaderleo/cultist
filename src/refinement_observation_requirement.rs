use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::observation_frontier::ObservationRequirement;
use crate::refinement_episode::{RefinementEpisodeBatch, validate_refinement_episode_batch};

pub const REFINEMENT_OBSERVATION_REQUIREMENT_SCHEMA_VERSION: u32 = 1;
pub const MAX_REFINEMENT_OBSERVATION_REQUIREMENT_REQUEST_BYTES: usize = 512 * 1024;
const MAX_MAPPINGS: usize = 512;
const MAX_ID_BYTES: usize = 512;
const MAX_REFERENCE_BYTES: usize = 4096;

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RefinementObservationRequirementMapping {
    pub id: String,
    pub episode_id: String,
    pub candidate_id: String,
    pub discriminator_id: String,
    pub subject_ref: String,
    pub source_receipt: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RefinementObservationRequirementBatch {
    pub schema_version: u32,
    pub mappings: Vec<RefinementObservationRequirementMapping>,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RefinementObservationRequirementRequest {
    pub schema_version: u32,
    pub refinements: RefinementEpisodeBatch,
    pub mappings: RefinementObservationRequirementBatch,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SelectedRefinementObservationRequirements {
    pub episode_id: String,
    pub candidate_id: String,
    pub requirements: Vec<ObservationRequirement>,
    pub mappings: Vec<RefinementObservationRequirementMapping>,
    pub missing_discriminator_refs: Vec<String>,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RefinementObservationRequirementEvaluation {
    pub schema_version: u32,
    pub selected: Vec<SelectedRefinementObservationRequirements>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct RefinementObservationRequirementError {
    message: String,
}

impl RefinementObservationRequirementError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for RefinementObservationRequirementError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for RefinementObservationRequirementError {}

pub fn parse_refinement_observation_requirement_request(
    bytes: &[u8],
) -> Result<RefinementObservationRequirementRequest, RefinementObservationRequirementError> {
    if bytes.len() > MAX_REFINEMENT_OBSERVATION_REQUIREMENT_REQUEST_BYTES {
        return Err(RefinementObservationRequirementError::new(format!(
            "refinement observation requirement request exceeds the {MAX_REFINEMENT_OBSERVATION_REQUIREMENT_REQUEST_BYTES}-byte limit"
        )));
    }
    let request: RefinementObservationRequirementRequest =
        serde_json::from_slice(bytes).map_err(|error| {
            RefinementObservationRequirementError::new(format!(
                "invalid refinement observation requirement JSON: {error}"
            ))
        })?;
    validate_request(&request)?;
    Ok(request)
}

pub fn evaluate_selected_observation_requirements(
    request: &RefinementObservationRequirementRequest,
) -> Result<RefinementObservationRequirementEvaluation, RefinementObservationRequirementError> {
    validate_request(request)?;

    let mut selected = Vec::new();
    for episode in &request.refinements.episodes {
        let Some(selected_id) = episode.selected_transition.as_ref() else {
            continue;
        };
        let candidate = episode
            .candidate_refinements
            .iter()
            .find(|candidate| candidate.id == *selected_id)
            .ok_or_else(|| {
                RefinementObservationRequirementError::new(format!(
                    "selected candidate {selected_id} is missing from episode {}",
                    episode.id
                ))
            })?;

        let mut requirements = Vec::new();
        let mut mappings = Vec::new();
        let mut missing_discriminator_refs = Vec::new();
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
                mappings.push(mapping.clone());
            } else {
                missing_discriminator_refs.push(discriminator_id.clone());
            }
        }

        requirements.sort_by(|left, right| {
            left.discriminator_id
                .cmp(&right.discriminator_id)
                .then_with(|| left.subject_ref.cmp(&right.subject_ref))
        });
        mappings.sort_by(|left, right| left.id.cmp(&right.id));
        missing_discriminator_refs.sort();
        selected.push(SelectedRefinementObservationRequirements {
            episode_id: episode.id.clone(),
            candidate_id: candidate.id.clone(),
            requirements,
            mappings,
            missing_discriminator_refs,
        });
    }
    selected.sort_by(|left, right| left.episode_id.cmp(&right.episode_id));

    Ok(RefinementObservationRequirementEvaluation {
        schema_version: REFINEMENT_OBSERVATION_REQUIREMENT_SCHEMA_VERSION,
        selected,
    })
}

fn validate_request(
    request: &RefinementObservationRequirementRequest,
) -> Result<(), RefinementObservationRequirementError> {
    if request.schema_version != REFINEMENT_OBSERVATION_REQUIREMENT_SCHEMA_VERSION {
        return Err(RefinementObservationRequirementError::new(format!(
            "unsupported refinement observation requirement schema {}; expected {REFINEMENT_OBSERVATION_REQUIREMENT_SCHEMA_VERSION}",
            request.schema_version
        )));
    }
    validate_refinement_episode_batch(&request.refinements).map_err(|error| {
        RefinementObservationRequirementError::new(format!(
            "invalid refinement episode batch: {error}"
        ))
    })?;
    validate_mapping_batch(&request.mappings)?;
    validate_mapping_references(&request.refinements, &request.mappings)
}

fn validate_mapping_batch(
    batch: &RefinementObservationRequirementBatch,
) -> Result<(), RefinementObservationRequirementError> {
    if batch.schema_version != REFINEMENT_OBSERVATION_REQUIREMENT_SCHEMA_VERSION {
        return Err(RefinementObservationRequirementError::new(format!(
            "unsupported refinement observation mapping schema {}; expected {REFINEMENT_OBSERVATION_REQUIREMENT_SCHEMA_VERSION}",
            batch.schema_version
        )));
    }
    if batch.mappings.len() > MAX_MAPPINGS {
        return Err(RefinementObservationRequirementError::new(
            "refinement observation mappings exceed the admitted boundary",
        ));
    }

    let mut ids = BTreeSet::new();
    let mut keys = BTreeSet::new();
    for mapping in &batch.mappings {
        validate_atom(&mapping.id, "mapping id", MAX_ID_BYTES)?;
        validate_atom(&mapping.episode_id, "mapping episode_id", MAX_ID_BYTES)?;
        validate_atom(&mapping.candidate_id, "mapping candidate_id", MAX_ID_BYTES)?;
        validate_atom(
            &mapping.discriminator_id,
            "mapping discriminator_id",
            MAX_ID_BYTES,
        )?;
        validate_atom(
            &mapping.subject_ref,
            "mapping subject_ref",
            MAX_REFERENCE_BYTES,
        )?;
        validate_atom(
            &mapping.source_receipt,
            "mapping source_receipt",
            MAX_REFERENCE_BYTES,
        )?;
        if !ids.insert(mapping.id.clone()) {
            return Err(RefinementObservationRequirementError::new(format!(
                "duplicate refinement observation mapping id {}",
                mapping.id
            )));
        }
        let key = (
            mapping.episode_id.clone(),
            mapping.candidate_id.clone(),
            mapping.discriminator_id.clone(),
        );
        if !keys.insert(key) {
            return Err(RefinementObservationRequirementError::new(format!(
                "multiple subject mappings for refinement {} / {} / {}",
                mapping.episode_id, mapping.candidate_id, mapping.discriminator_id
            )));
        }
    }
    Ok(())
}

fn validate_mapping_references(
    refinements: &RefinementEpisodeBatch,
    mappings: &RefinementObservationRequirementBatch,
) -> Result<(), RefinementObservationRequirementError> {
    for mapping in &mappings.mappings {
        let episode = refinements
            .episodes
            .iter()
            .find(|episode| episode.id == mapping.episode_id)
            .ok_or_else(|| {
                RefinementObservationRequirementError::new(format!(
                    "mapping {} references missing episode {}",
                    mapping.id, mapping.episode_id
                ))
            })?;
        let candidate = episode
            .candidate_refinements
            .iter()
            .find(|candidate| candidate.id == mapping.candidate_id)
            .ok_or_else(|| {
                RefinementObservationRequirementError::new(format!(
                    "mapping {} references missing candidate {} in episode {}",
                    mapping.id, mapping.candidate_id, mapping.episode_id
                ))
            })?;
        if !candidate
            .discriminator_refs
            .iter()
            .any(|discriminator| discriminator == &mapping.discriminator_id)
        {
            return Err(RefinementObservationRequirementError::new(format!(
                "mapping {} discriminator {} is not required by candidate {}",
                mapping.id, mapping.discriminator_id, mapping.candidate_id
            )));
        }
    }
    Ok(())
}

fn validate_atom(
    value: &str,
    field: &str,
    max_bytes: usize,
) -> Result<(), RefinementObservationRequirementError> {
    if value.is_empty()
        || value.trim() != value
        || value.len() > max_bytes
        || value.contains('\0')
        || value.contains(['\n', '\r'])
    {
        return Err(RefinementObservationRequirementError::new(format!(
            "{field} must be bounded canonical single-line text"
        )));
    }
    Ok(())
}
