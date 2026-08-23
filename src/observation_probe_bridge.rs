use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::applicability::{
    APPLICABILITY_SCHEMA_VERSION, ApplicabilityQuery, ApplicabilityStatus, EvaluationContext,
    EvidenceRequirements, evaluate_query,
};
use crate::durable_obligation::{
    ClearingCondition, DURABLE_OBLIGATION_SCHEMA_VERSION, DiscriminatorKey, DurableObligation,
};
use crate::evidence_planner::{
    EVIDENCE_PLANNER_SCHEMA_VERSION, EvidencePlan, EvidenceProbe, ProbePlanRequest,
    ProbeSelectionPolicy, plan_evidence,
};
use crate::observation_frontier::{ObservationFrontierReceipt, ObservationFrontierStatus};

pub const OBSERVATION_PROBE_BRIDGE_SCHEMA_VERSION: u32 = 2;
pub const MAX_OBSERVATION_PROBE_PLAN_REQUEST_BYTES: usize = 1024 * 1024;
const MAX_BRIDGES: usize = 128;
const MAX_ID_BYTES: usize = 256;
const MAX_REFERENCE_BYTES: usize = 4096;

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ObservationProbeBridge {
    pub bridge_id: String,
    pub observation_discriminator_id: String,
    pub observation_subject_ref: String,
    pub probe_discriminator: DiscriminatorKey,
    pub clearing_requirements: EvidenceRequirements,
    pub source_receipt: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ObservationProbePlanRequest {
    pub schema_version: u32,
    pub frontier: ObservationFrontierReceipt,
    pub frontier_requirements: EvidenceRequirements,
    pub bridges: Vec<ObservationProbeBridge>,
    pub context: EvaluationContext,
    pub probes: Vec<EvidenceProbe>,
    pub allow_effectful: bool,
    pub policy: ProbeSelectionPolicy,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ObservationProbePlanStatus {
    AlreadyCurrent,
    NoAdmittedMapping,
    Planned,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ObservationProbePlan {
    pub schema_version: u32,
    pub observation_discriminator_id: String,
    pub observation_subject_ref: String,
    pub frontier_status: ObservationFrontierStatus,
    pub applicability_status: ApplicabilityStatus,
    pub status: ObservationProbePlanStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bridge_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bridge_source_receipt: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence_plan: Option<EvidencePlan>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ObservationProbeBridgeError {
    message: String,
}

impl ObservationProbeBridgeError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for ObservationProbeBridgeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for ObservationProbeBridgeError {}

pub fn parse_observation_probe_plan_request(
    bytes: &[u8],
) -> Result<ObservationProbePlanRequest, ObservationProbeBridgeError> {
    if bytes.len() > MAX_OBSERVATION_PROBE_PLAN_REQUEST_BYTES {
        return Err(ObservationProbeBridgeError::new(format!(
            "observation probe plan request exceeds the {MAX_OBSERVATION_PROBE_PLAN_REQUEST_BYTES}-byte limit"
        )));
    }
    let request: ObservationProbePlanRequest = serde_json::from_slice(bytes).map_err(|error| {
        ObservationProbeBridgeError::new(format!("invalid observation probe plan JSON: {error}"))
    })?;
    validate_request(&request)?;
    Ok(request)
}

pub fn plan_observation_probe(
    request: &ObservationProbePlanRequest,
) -> Result<ObservationProbePlan, ObservationProbeBridgeError> {
    validate_request(request)?;

    let frontier = &request.frontier;
    let applicability_status = evaluate_frontier_applicability(request)?;
    if frontier.status == ObservationFrontierStatus::Current
        && applicability_status == ApplicabilityStatus::Applies
    {
        return Ok(base_plan(
            frontier,
            applicability_status,
            ObservationProbePlanStatus::AlreadyCurrent,
        ));
    }

    let matching = request
        .bridges
        .iter()
        .filter(|bridge| bridge_matches_frontier(bridge, frontier))
        .collect::<Vec<_>>();

    let Some(bridge) = matching.first().copied() else {
        return Ok(base_plan(
            frontier,
            applicability_status,
            ObservationProbePlanStatus::NoAdmittedMapping,
        ));
    };

    let obligation = DurableObligation {
        schema_version: DURABLE_OBLIGATION_SCHEMA_VERSION,
        id: format!("observation-bridge:{}", bridge.bridge_id),
        question: format!(
            "Acquire current observation through bridge {}",
            bridge.bridge_id
        ),
        subject: bridge.clearing_requirements.clone(),
        established_evidence: Vec::new(),
        missing_discriminator: bridge.probe_discriminator.clone(),
        clearing_conditions: vec![ClearingCondition {
            discriminator: bridge.probe_discriminator.clone(),
            requirements: bridge.clearing_requirements.clone(),
        }],
    };

    let evidence_plan = plan_evidence(&ProbePlanRequest {
        schema_version: EVIDENCE_PLANNER_SCHEMA_VERSION,
        obligation,
        context: request.context.clone(),
        probes: request.probes.clone(),
        allow_effectful: request.allow_effectful,
        policy: request.policy,
    })
    .map_err(|error| {
        ObservationProbeBridgeError::new(format!("evidence planning failed: {error}"))
    })?;

    Ok(ObservationProbePlan {
        schema_version: OBSERVATION_PROBE_BRIDGE_SCHEMA_VERSION,
        observation_discriminator_id: frontier.discriminator_id.clone(),
        observation_subject_ref: frontier.subject_ref.clone(),
        frontier_status: frontier.status,
        applicability_status,
        status: ObservationProbePlanStatus::Planned,
        bridge_id: Some(bridge.bridge_id.clone()),
        bridge_source_receipt: Some(bridge.source_receipt.clone()),
        evidence_plan: Some(evidence_plan),
    })
}

fn evaluate_frontier_applicability(
    request: &ObservationProbePlanRequest,
) -> Result<ApplicabilityStatus, ObservationProbeBridgeError> {
    evaluate_query(&ApplicabilityQuery {
        schema_version: APPLICABILITY_SCHEMA_VERSION,
        requirements: request.frontier_requirements.clone(),
        context: request.context.clone(),
    })
    .map(|evaluation| evaluation.status)
    .map_err(|error| {
        ObservationProbeBridgeError::new(format!(
            "frontier applicability evaluation failed: {error}"
        ))
    })
}

fn base_plan(
    frontier: &ObservationFrontierReceipt,
    applicability_status: ApplicabilityStatus,
    status: ObservationProbePlanStatus,
) -> ObservationProbePlan {
    ObservationProbePlan {
        schema_version: OBSERVATION_PROBE_BRIDGE_SCHEMA_VERSION,
        observation_discriminator_id: frontier.discriminator_id.clone(),
        observation_subject_ref: frontier.subject_ref.clone(),
        frontier_status: frontier.status,
        applicability_status,
        status,
        bridge_id: None,
        bridge_source_receipt: None,
        evidence_plan: None,
    }
}

fn bridge_matches_frontier(
    bridge: &ObservationProbeBridge,
    frontier: &ObservationFrontierReceipt,
) -> bool {
    bridge.observation_discriminator_id == frontier.discriminator_id
        && bridge.observation_subject_ref == frontier.subject_ref
}

fn validate_request(
    request: &ObservationProbePlanRequest,
) -> Result<(), ObservationProbeBridgeError> {
    if request.schema_version != OBSERVATION_PROBE_BRIDGE_SCHEMA_VERSION {
        return Err(ObservationProbeBridgeError::new(format!(
            "unsupported observation probe bridge schema {}; expected {OBSERVATION_PROBE_BRIDGE_SCHEMA_VERSION}",
            request.schema_version
        )));
    }
    validate_frontier(&request.frontier)?;
    evaluate_frontier_applicability(request)?;
    if request.bridges.len() > MAX_BRIDGES {
        return Err(ObservationProbeBridgeError::new(
            "observation probe bridges exceed the admitted boundary",
        ));
    }

    let mut bridge_ids = BTreeSet::new();
    let mut mappings = BTreeSet::new();
    for bridge in &request.bridges {
        validate_bridge(bridge)?;
        if !bridge_ids.insert(bridge.bridge_id.clone()) {
            return Err(ObservationProbeBridgeError::new(format!(
                "duplicate observation probe bridge id {}",
                bridge.bridge_id
            )));
        }
        let mapping = (
            bridge.observation_discriminator_id.clone(),
            bridge.observation_subject_ref.clone(),
        );
        if !mappings.insert(mapping) {
            return Err(ObservationProbeBridgeError::new(format!(
                "multiple admitted bridges for observation {} @ {}",
                bridge.observation_discriminator_id, bridge.observation_subject_ref
            )));
        }
    }
    Ok(())
}

fn validate_frontier(
    frontier: &ObservationFrontierReceipt,
) -> Result<(), ObservationProbeBridgeError> {
    validate_atom(
        &frontier.discriminator_id,
        "frontier discriminator_id",
        MAX_ID_BYTES,
    )?;
    validate_atom(
        &frontier.subject_ref,
        "frontier subject_ref",
        MAX_REFERENCE_BYTES,
    )?;

    let coherent = match frontier.status {
        ObservationFrontierStatus::Current => !frontier.current.is_empty(),
        ObservationFrontierStatus::Unknown => {
            frontier.current.is_empty() && !frontier.unknown.is_empty()
        }
        ObservationFrontierStatus::Invalid => {
            frontier.current.is_empty()
                && frontier.unknown.is_empty()
                && !frontier.invalid.is_empty()
        }
        ObservationFrontierStatus::Missing => {
            frontier.current.is_empty()
                && frontier.unknown.is_empty()
                && frontier.invalid.is_empty()
        }
    };
    if !coherent {
        return Err(ObservationProbeBridgeError::new(
            "observation frontier status disagrees with its current/unknown/invalid receipts",
        ));
    }
    Ok(())
}

fn validate_bridge(bridge: &ObservationProbeBridge) -> Result<(), ObservationProbeBridgeError> {
    validate_atom(&bridge.bridge_id, "bridge id", MAX_ID_BYTES)?;
    validate_atom(
        &bridge.observation_discriminator_id,
        "bridge observation_discriminator_id",
        MAX_ID_BYTES,
    )?;
    validate_atom(
        &bridge.observation_subject_ref,
        "bridge observation_subject_ref",
        MAX_REFERENCE_BYTES,
    )?;
    validate_atom(
        &bridge.probe_discriminator.kind,
        "bridge probe discriminator kind",
        MAX_ID_BYTES,
    )?;
    validate_atom(
        &bridge.probe_discriminator.target,
        "bridge probe discriminator target",
        MAX_REFERENCE_BYTES,
    )?;
    validate_atom(
        &bridge.source_receipt,
        "bridge source_receipt",
        MAX_REFERENCE_BYTES,
    )?;
    if bridge.clearing_requirements.repository.is_none()
        && bridge.clearing_requirements.revision.is_none()
        && bridge.clearing_requirements.work.is_none()
        && bridge.clearing_requirements.scope.is_none()
    {
        return Err(ObservationProbeBridgeError::new(
            "bridge clearing_requirements must carry at least one applicability coordinate",
        ));
    }
    Ok(())
}

fn validate_atom(
    value: &str,
    field: &str,
    max_bytes: usize,
) -> Result<(), ObservationProbeBridgeError> {
    if value.is_empty()
        || value.trim() != value
        || value.len() > max_bytes
        || value.contains('\0')
        || value.contains(['\n', '\r'])
    {
        return Err(ObservationProbeBridgeError::new(format!(
            "{field} must be bounded canonical single-line text"
        )));
    }
    Ok(())
}
