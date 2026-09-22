use std::error::Error;
use std::fmt;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::applicability::{
    APPLICABILITY_SCHEMA_VERSION, ApplicabilityQuery, ApplicabilityStatus, EvaluationContext,
    EvidenceApplicability, EvidenceRequirements, evaluate_query,
};
use crate::cmux_review::{
    CmuxReviewEnvelope, CmuxReviewReceipt, CmuxReviewSource, project_cmux_review_receipt,
};

pub const CMUX_REVIEW_APPLICABILITY_SCHEMA_VERSION: u32 = 1;
pub const CMUX_REVIEW_SOURCE_FINGERPRINT_SCHEME: &str = "cmux-review-source-sha256-v1";

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CmuxReviewApplicabilityRequest {
    pub receipt: CmuxReviewReceipt,
    pub current: CmuxReviewCurrentContext,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CmuxReviewCurrentContext {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<CmuxReviewSource>,
    pub policy_version: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ruleset_sha256: Option<String>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CmuxReviewApplicabilityProjection {
    pub schema_version: u32,
    pub envelope: CmuxReviewEnvelope,
    pub reviewed_source_fingerprint: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_source_fingerprint: Option<String>,
    pub source_applicability: EvidenceApplicability,
    pub policy_version_status: CmuxReviewCoordinateStatus,
    pub ruleset_status: CmuxReviewCoordinateStatus,
    pub disposition: CmuxReviewContinuityDisposition,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CmuxReviewCoordinateStatus {
    Matched,
    Mismatched,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CmuxReviewContinuityDisposition {
    ReuseExactReview,
    RefreshReview,
    NeedCurrentSource,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CmuxReviewApplicabilityError {
    message: String,
}

impl CmuxReviewApplicabilityError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for CmuxReviewApplicabilityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for CmuxReviewApplicabilityError {}

pub fn project_cmux_review_for_context(
    request: &CmuxReviewApplicabilityRequest,
) -> Result<CmuxReviewApplicabilityProjection, CmuxReviewApplicabilityError> {
    let envelope = project_cmux_review_receipt(&request.receipt)
        .map_err(|error| CmuxReviewApplicabilityError::new(error.to_string()))?;

    validate_policy_version(&request.current.policy_version)?;
    validate_ruleset(request.current.ruleset_sha256.as_deref())?;

    let reviewed_source_fingerprint = fingerprint_source(&request.receipt.source)?;
    let current_source_fingerprint = request
        .current
        .source
        .as_ref()
        .map(fingerprint_source)
        .transpose()?;

    let source_applicability = evaluate_query(&ApplicabilityQuery {
        schema_version: APPLICABILITY_SCHEMA_VERSION,
        requirements: EvidenceRequirements {
            revision: Some(reviewed_source_fingerprint.clone()),
            ..EvidenceRequirements::default()
        },
        context: EvaluationContext {
            revision: current_source_fingerprint.clone(),
            ..EvaluationContext::default()
        },
    })
    .map_err(|error| CmuxReviewApplicabilityError::new(error.to_string()))?;

    let policy_version_status = if request.current.policy_version == request.receipt.policy_version
    {
        CmuxReviewCoordinateStatus::Matched
    } else {
        CmuxReviewCoordinateStatus::Mismatched
    };
    let ruleset_status = if request.current.ruleset_sha256 == request.receipt.ruleset_sha256 {
        CmuxReviewCoordinateStatus::Matched
    } else {
        CmuxReviewCoordinateStatus::Mismatched
    };

    let disposition = if source_applicability.status == ApplicabilityStatus::Unknown {
        CmuxReviewContinuityDisposition::NeedCurrentSource
    } else if source_applicability.status == ApplicabilityStatus::Invalid
        || policy_version_status == CmuxReviewCoordinateStatus::Mismatched
        || ruleset_status == CmuxReviewCoordinateStatus::Mismatched
    {
        CmuxReviewContinuityDisposition::RefreshReview
    } else {
        CmuxReviewContinuityDisposition::ReuseExactReview
    };

    Ok(CmuxReviewApplicabilityProjection {
        schema_version: CMUX_REVIEW_APPLICABILITY_SCHEMA_VERSION,
        envelope,
        reviewed_source_fingerprint,
        current_source_fingerprint,
        source_applicability,
        policy_version_status,
        ruleset_status,
        disposition,
    })
}

pub fn fingerprint_source(
    source: &CmuxReviewSource,
) -> Result<String, CmuxReviewApplicabilityError> {
    validate_source(source)?;

    let mut hasher = Sha256::new();
    hasher.update(CMUX_REVIEW_SOURCE_FINGERPRINT_SCHEME.as_bytes());
    hasher.update([0]);
    hasher.update(source.base_sha.as_bytes());
    hasher.update([0]);
    hasher.update(source.head_sha.as_bytes());
    hasher.update([0]);
    hasher.update(source.diff_sha256.as_bytes());
    hasher.update([0]);
    hasher.update(if source.working_tree_dirty {
        b"1"
    } else {
        b"0"
    });

    let digest = hasher.finalize();
    let mut hex = String::with_capacity(64);
    for byte in digest {
        hex.push(hex_digit(byte >> 4));
        hex.push(hex_digit(byte & 0x0f));
    }
    Ok(format!("{CMUX_REVIEW_SOURCE_FINGERPRINT_SCHEME}:{hex}"))
}

fn validate_source(source: &CmuxReviewSource) -> Result<(), CmuxReviewApplicabilityError> {
    validate_git_object_id(&source.base_sha, "current source base_sha")?;
    validate_git_object_id(&source.head_sha, "current source head_sha")?;
    validate_sha256(&source.diff_sha256, "current source diff_sha256")
}

fn validate_policy_version(value: &str) -> Result<(), CmuxReviewApplicabilityError> {
    if value.is_empty() || value.trim() != value || value.contains('\0') {
        return Err(CmuxReviewApplicabilityError::new(
            "current policy_version must be a non-empty canonical string",
        ));
    }
    Ok(())
}

fn validate_ruleset(value: Option<&str>) -> Result<(), CmuxReviewApplicabilityError> {
    if let Some(value) = value {
        validate_sha256(value, "current ruleset_sha256")?;
    }
    Ok(())
}

fn validate_git_object_id(value: &str, field: &str) -> Result<(), CmuxReviewApplicabilityError> {
    if !matches!(value.len(), 40 | 64) || !is_lower_hex(value) {
        return Err(CmuxReviewApplicabilityError::new(format!(
            "{field} must be an exact 40- or 64-character lowercase Git object id"
        )));
    }
    Ok(())
}

fn validate_sha256(value: &str, field: &str) -> Result<(), CmuxReviewApplicabilityError> {
    if value.len() != 64 || !is_lower_hex(value) {
        return Err(CmuxReviewApplicabilityError::new(format!(
            "{field} must be an exact 64-character lowercase sha256"
        )));
    }
    Ok(())
}

fn is_lower_hex(value: &str) -> bool {
    value
        .bytes()
        .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn hex_digit(nibble: u8) -> char {
    match nibble {
        0..=9 => char::from(b'0' + nibble),
        10..=15 => char::from(b'a' + (nibble - 10)),
        _ => unreachable!("nibble is masked to four bits"),
    }
}
