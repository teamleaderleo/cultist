use std::error::Error;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::applicability::{
    APPLICABILITY_SCHEMA_VERSION, ApplicabilityQuery, ApplicabilityStatus, EvaluationContext,
    EvidenceApplicability, EvidenceRequirements, evaluate_query,
};
use crate::cmux_review::{
    CmuxReviewEnvelope, CmuxReviewReceipt, project_cmux_review_receipt,
};

pub const CMUX_REVIEW_APPLICABILITY_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CmuxReviewApplicabilityRequest {
    pub receipt: CmuxReviewReceipt,
    pub current: EvaluationContext,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CmuxReviewApplicabilityProjection {
    pub schema_version: u32,
    pub envelope: CmuxReviewEnvelope,
    pub applicability: EvidenceApplicability,
    pub disposition: CmuxReviewContinuityDisposition,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CmuxReviewContinuityDisposition {
    ReuseExactReview,
    RefreshReview,
    NeedCurrentRevision,
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

    if let Some(revision) = request.current.revision.as_deref() {
        validate_git_object_id(revision)?;
    }

    let applicability = evaluate_query(&ApplicabilityQuery {
        schema_version: APPLICABILITY_SCHEMA_VERSION,
        requirements: EvidenceRequirements {
            revision: Some(request.receipt.source.head_sha.clone()),
            ..EvidenceRequirements::default()
        },
        context: request.current.clone(),
    })
    .map_err(|error| CmuxReviewApplicabilityError::new(error.to_string()))?;

    let disposition = match applicability.status {
        ApplicabilityStatus::Applies => CmuxReviewContinuityDisposition::ReuseExactReview,
        ApplicabilityStatus::Invalid => CmuxReviewContinuityDisposition::RefreshReview,
        ApplicabilityStatus::Unknown => CmuxReviewContinuityDisposition::NeedCurrentRevision,
    };

    Ok(CmuxReviewApplicabilityProjection {
        schema_version: CMUX_REVIEW_APPLICABILITY_SCHEMA_VERSION,
        envelope,
        applicability,
        disposition,
    })
}

fn validate_git_object_id(value: &str) -> Result<(), CmuxReviewApplicabilityError> {
    if !matches!(value.len(), 40 | 64)
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(CmuxReviewApplicabilityError::new(
            "current revision must be an exact 40- or 64-character lowercase Git object id",
        ));
    }
    Ok(())
}
