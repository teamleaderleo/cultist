use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::finding::ClaimKind;

pub const CMUX_REVIEW_RECEIPT_SCHEMA_VERSION: u32 = 1;
pub const CMUX_REVIEW_ENVELOPE_SCHEMA_VERSION: u32 = 1;
pub const MAX_CMUX_REVIEW_RECEIPT_BYTES: usize = 1024 * 1024;

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CmuxReviewReceipt {
    pub schema_version: u32,
    pub policy_version: String,
    pub repository_root: String,
    pub source: CmuxReviewSource,
    pub brief: CmuxReviewBrief,
    pub summary: CmuxReviewSummary,
    pub findings: Vec<CmuxReviewFinding>,
    pub created_at: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CmuxReviewSource {
    pub base_sha: String,
    pub head_sha: String,
    pub diff_sha256: String,
    pub working_tree_dirty: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ruleset_sha256: Option<String>,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CmuxReviewBrief {
    pub intent: String,
    pub requirements: Vec<CmuxReviewRequirement>,
    pub out_of_scope_changes: Vec<String>,
    pub behavior_changed: Vec<String>,
    pub risk_areas: Vec<CmuxReviewRiskArea>,
    pub file_groups: Vec<CmuxReviewFileGroup>,
    pub reading_order: Vec<String>,
    pub safeguards: Vec<String>,
    pub coverage_gaps: Vec<String>,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CmuxReviewRequirement {
    pub requirement: String,
    pub status: CmuxReviewRequirementStatus,
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CmuxReviewRequirementStatus {
    Satisfied,
    Missing,
    Uncertain,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CmuxReviewRiskArea {
    pub level: CmuxReviewRiskLevel,
    pub area: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CmuxReviewRiskLevel {
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CmuxReviewFileGroup {
    pub label: String,
    pub paths: Vec<String>,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CmuxReviewSummary {
    pub hypotheses_investigated: usize,
    pub suppressed: usize,
    pub refuted: usize,
    pub verified: usize,
    pub human_judgment: usize,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CmuxReviewFinding {
    pub id: String,
    pub title: String,
    pub severity: CmuxReviewSeverity,
    pub claims: Vec<CmuxReviewClaim>,
    pub failure_mode: String,
    pub paths: Vec<String>,
    pub discovery_sources: Vec<String>,
    pub challenge: CmuxReviewChallenge,
    pub verification: CmuxReviewVerification,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repair: Option<CmuxReviewRepair>,
    pub disposition: CmuxReviewDisposition,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Deserialize, Serialize)]
pub enum CmuxReviewSeverity {
    P0,
    P1,
    P2,
    P3,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CmuxReviewClaim {
    pub kind: ClaimKind,
    pub message: String,
    pub evidence: Vec<CmuxReviewEvidence>,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CmuxReviewEvidence {
    pub kind: CmuxReviewEvidenceKind,
    pub summary: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line: Option<usize>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CmuxReviewEvidenceKind {
    CodePath,
    Guard,
    Test,
    Build,
    StaticAnalysis,
    Reproduction,
    RuntimeTrace,
    UiAutomation,
    Counterexample,
    Human,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CmuxReviewChallenge {
    pub disposition: CmuxReviewChallengeDisposition,
    pub evidence: Vec<CmuxReviewEvidence>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CmuxReviewChallengeDisposition {
    Refuted,
    SurvivesChallenge,
    Uncertain,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CmuxReviewVerification {
    pub result: CmuxReviewVerificationResult,
    pub evidence: Vec<CmuxReviewEvidence>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CmuxReviewVerificationResult {
    Reproduced,
    SupportedStatic,
    NotReproduced,
    Blocked,
    HumanJudgment,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CmuxReviewRepair {
    pub attempted: bool,
    pub result: CmuxReviewRepairResult,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub after_source: Option<CmuxReviewSource>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verification: Option<CmuxReviewPostRepairVerification>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CmuxReviewPostRepairVerification {
    pub result: CmuxReviewPostRepairVerificationResult,
    pub evidence: Vec<CmuxReviewEvidence>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CmuxReviewPostRepairVerificationResult {
    Passed,
    Failed,
    Blocked,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CmuxReviewRepairResult {
    Fixed,
    Failed,
    Deferred,
    NotAttempted,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CmuxReviewDisposition {
    Repaired,
    Refuted,
    AcceptedRisk,
    Superseded,
    HumanRequired,
    Unresolved,
    Suppressed,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CmuxReviewEnvelope {
    pub schema_version: u32,
    pub policy_version: String,
    pub source: CmuxReviewSource,
    pub created_at: String,
    pub intent: String,
    pub requirements: CmuxReviewRequirementSummary,
    pub attention: Vec<CmuxReviewAttentionItem>,
    pub frontier: Vec<CmuxReviewFrontierItem>,
    pub quiet: Vec<CmuxReviewQuietItem>,
    pub coverage_gaps: Vec<String>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CmuxReviewRequirementSummary {
    pub satisfied: usize,
    pub missing: usize,
    pub uncertain: usize,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CmuxReviewAttentionItem {
    pub id: String,
    pub title: String,
    pub severity: CmuxReviewSeverity,
    pub disposition: CmuxReviewDisposition,
    pub paths: Vec<String>,
    pub claims: Vec<CmuxReviewClaim>,
    pub challenge: CmuxReviewChallengeDisposition,
    pub verification: CmuxReviewVerificationResult,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repair: Option<CmuxReviewRepair>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CmuxReviewFrontierItem {
    pub finding_id: String,
    pub kind: CmuxReviewFrontierKind,
    pub message: String,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CmuxReviewFrontierKind {
    UnknownClaim,
    ChallengeUncertain,
    VerificationBlocked,
    HumanJudgment,
    HumanRequired,
    Unresolved,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CmuxReviewQuietItem {
    pub id: String,
    pub title: String,
    pub disposition: CmuxReviewDisposition,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CmuxReviewError {
    message: String,
}

impl CmuxReviewError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for CmuxReviewError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for CmuxReviewError {}

pub fn parse_cmux_review_receipt(bytes: &[u8]) -> Result<CmuxReviewReceipt, CmuxReviewError> {
    if bytes.len() > MAX_CMUX_REVIEW_RECEIPT_BYTES {
        return Err(CmuxReviewError::new(format!(
            "cmux review receipt exceeds the {MAX_CMUX_REVIEW_RECEIPT_BYTES}-byte limit"
        )));
    }
    serde_json::from_slice(bytes)
        .map_err(|error| CmuxReviewError::new(format!("invalid cmux review receipt: {error}")))
}

pub fn project_cmux_review_receipt(
    receipt: &CmuxReviewReceipt,
) -> Result<CmuxReviewEnvelope, CmuxReviewError> {
    validate_receipt(receipt)?;

    let requirements = CmuxReviewRequirementSummary {
        satisfied: receipt
            .brief
            .requirements
            .iter()
            .filter(|requirement| requirement.status == CmuxReviewRequirementStatus::Satisfied)
            .count(),
        missing: receipt
            .brief
            .requirements
            .iter()
            .filter(|requirement| requirement.status == CmuxReviewRequirementStatus::Missing)
            .count(),
        uncertain: receipt
            .brief
            .requirements
            .iter()
            .filter(|requirement| requirement.status == CmuxReviewRequirementStatus::Uncertain)
            .count(),
    };

    let mut attention = Vec::new();
    let mut quiet = Vec::new();
    let mut frontier = Vec::new();
    let mut frontier_keys = BTreeSet::new();

    for finding in &receipt.findings {
        if matches!(
            finding.disposition,
            CmuxReviewDisposition::Refuted | CmuxReviewDisposition::Suppressed
        ) {
            quiet.push(CmuxReviewQuietItem {
                id: finding.id.clone(),
                title: finding.title.clone(),
                disposition: finding.disposition,
            });
            continue;
        }

        attention.push(CmuxReviewAttentionItem {
            id: finding.id.clone(),
            title: finding.title.clone(),
            severity: finding.severity,
            disposition: finding.disposition,
            paths: finding.paths.clone(),
            claims: finding.claims.clone(),
            challenge: finding.challenge.disposition,
            verification: finding.verification.result,
            repair: finding.repair.clone(),
        });

        for claim in &finding.claims {
            if claim.kind == ClaimKind::Unknown {
                push_frontier(
                    &mut frontier,
                    &mut frontier_keys,
                    &finding.id,
                    CmuxReviewFrontierKind::UnknownClaim,
                    claim.message.clone(),
                );
            }
        }
        if finding.challenge.disposition == CmuxReviewChallengeDisposition::Uncertain {
            push_frontier(
                &mut frontier,
                &mut frontier_keys,
                &finding.id,
                CmuxReviewFrontierKind::ChallengeUncertain,
                "Adversarial challenge left competing interpretations unresolved.".to_string(),
            );
        }
        match finding.verification.result {
            CmuxReviewVerificationResult::Blocked => push_frontier(
                &mut frontier,
                &mut frontier_keys,
                &finding.id,
                CmuxReviewFrontierKind::VerificationBlocked,
                "Verification was blocked before a decisive discriminator could run.".to_string(),
            ),
            CmuxReviewVerificationResult::HumanJudgment => push_frontier(
                &mut frontier,
                &mut frontier_keys,
                &finding.id,
                CmuxReviewFrontierKind::HumanJudgment,
                "Verification requires human judgment.".to_string(),
            ),
            _ => {}
        }
        match finding.disposition {
            CmuxReviewDisposition::HumanRequired => push_frontier(
                &mut frontier,
                &mut frontier_keys,
                &finding.id,
                CmuxReviewFrontierKind::HumanRequired,
                "Finding disposition requires human review.".to_string(),
            ),
            CmuxReviewDisposition::Unresolved => push_frontier(
                &mut frontier,
                &mut frontier_keys,
                &finding.id,
                CmuxReviewFrontierKind::Unresolved,
                "Finding remains unresolved.".to_string(),
            ),
            _ => {}
        }
    }

    Ok(CmuxReviewEnvelope {
        schema_version: CMUX_REVIEW_ENVELOPE_SCHEMA_VERSION,
        policy_version: receipt.policy_version.clone(),
        source: receipt.source.clone(),
        created_at: receipt.created_at.clone(),
        intent: receipt.brief.intent.clone(),
        requirements,
        attention,
        frontier,
        quiet,
        coverage_gaps: receipt.brief.coverage_gaps.clone(),
    })
}

fn validate_receipt(receipt: &CmuxReviewReceipt) -> Result<(), CmuxReviewError> {
    if receipt.schema_version != CMUX_REVIEW_RECEIPT_SCHEMA_VERSION {
        return Err(CmuxReviewError::new(format!(
            "unsupported cmux review receipt schema {}; expected {CMUX_REVIEW_RECEIPT_SCHEMA_VERSION}",
            receipt.schema_version
        )));
    }

    validate_nonempty(&receipt.policy_version, "policy_version")?;
    validate_nonempty(&receipt.repository_root, "repository_root")?;
    validate_nonempty(&receipt.created_at, "created_at")?;
    validate_source(&receipt.source, "source")?;

    if receipt.summary.hypotheses_investigated < receipt.findings.len() {
        return Err(CmuxReviewError::new(
            "summary.hypotheses_investigated cannot be smaller than the retained finding count",
        ));
    }

    let refuted = receipt
        .findings
        .iter()
        .filter(|finding| finding.disposition == CmuxReviewDisposition::Refuted)
        .count();
    if receipt.summary.refuted != refuted {
        return Err(CmuxReviewError::new(format!(
            "summary.refuted={} does not match {} refuted findings",
            receipt.summary.refuted, refuted
        )));
    }

    let suppressed = receipt
        .findings
        .iter()
        .filter(|finding| finding.disposition == CmuxReviewDisposition::Suppressed)
        .count();
    if receipt.summary.suppressed != suppressed {
        return Err(CmuxReviewError::new(format!(
            "summary.suppressed={} does not match {} suppressed findings",
            receipt.summary.suppressed, suppressed
        )));
    }

    let human_judgment = receipt
        .findings
        .iter()
        .filter(|finding| finding.disposition == CmuxReviewDisposition::HumanRequired)
        .count();
    if receipt.summary.human_judgment != human_judgment {
        return Err(CmuxReviewError::new(format!(
            "summary.human_judgment={} does not match {} human-required findings",
            receipt.summary.human_judgment, human_judgment
        )));
    }

    let verified = receipt
        .findings
        .iter()
        .filter(|finding| {
            matches!(
                finding.verification.result,
                CmuxReviewVerificationResult::Reproduced
                    | CmuxReviewVerificationResult::SupportedStatic
            )
        })
        .count();
    if receipt.summary.verified != verified {
        return Err(CmuxReviewError::new(format!(
            "summary.verified={} does not match {} evidence-supported findings",
            receipt.summary.verified, verified
        )));
    }

    let mut finding_ids = BTreeSet::new();
    for finding in &receipt.findings {
        validate_finding(finding, &receipt.source)?;
        if !finding_ids.insert(finding.id.as_str()) {
            return Err(CmuxReviewError::new(format!(
                "duplicate finding id {}",
                finding.id
            )));
        }
    }

    Ok(())
}

fn validate_finding(
    finding: &CmuxReviewFinding,
    source: &CmuxReviewSource,
) -> Result<(), CmuxReviewError> {
    validate_nonempty(&finding.id, "finding.id")?;
    validate_nonempty(&finding.title, "finding.title")?;
    validate_nonempty(&finding.failure_mode, "finding.failure_mode")?;
    if finding.claims.is_empty() {
        return Err(CmuxReviewError::new(format!(
            "finding {} must retain at least one provenance-bearing claim",
            finding.id
        )));
    }

    for claim in &finding.claims {
        validate_nonempty(&claim.message, "finding claim message")?;
        for evidence in &claim.evidence {
            validate_nonempty(&evidence.summary, "finding claim evidence summary")?;
        }
    }
    for evidence in &finding.challenge.evidence {
        validate_nonempty(&evidence.summary, "challenge evidence summary")?;
    }
    for evidence in &finding.verification.evidence {
        validate_nonempty(&evidence.summary, "verification evidence summary")?;
    }

    if finding.disposition == CmuxReviewDisposition::Refuted
        && finding.challenge.disposition != CmuxReviewChallengeDisposition::Refuted
    {
        return Err(CmuxReviewError::new(format!(
            "finding {} is disposed refuted but the challenger did not refute it",
            finding.id
        )));
    }

    if finding.challenge.disposition == CmuxReviewChallengeDisposition::Refuted
        && finding.disposition != CmuxReviewDisposition::Refuted
        && finding.disposition != CmuxReviewDisposition::Suppressed
    {
        return Err(CmuxReviewError::new(format!(
            "finding {} survived as {:?} after the challenger refuted it",
            finding.id, finding.disposition
        )));
    }

    if matches!(
        finding.verification.result,
        CmuxReviewVerificationResult::Reproduced | CmuxReviewVerificationResult::SupportedStatic
    ) && !finding
        .claims
        .iter()
        .any(|claim| matches!(claim.kind, ClaimKind::Proven | ClaimKind::Derived))
    {
        return Err(CmuxReviewError::new(format!(
            "finding {} reports evidence-supported verification without a proven or derived claim",
            finding.id
        )));
    }

    if finding.disposition == CmuxReviewDisposition::Repaired {
        let repair = finding.repair.as_ref().ok_or_else(|| {
            CmuxReviewError::new(format!(
                "finding {} is disposed repaired without a repair receipt",
                finding.id
            ))
        })?;
        if !repair.attempted || repair.result != CmuxReviewRepairResult::Fixed {
            return Err(CmuxReviewError::new(format!(
                "finding {} is disposed repaired without an attempted fixed repair",
                finding.id
            )));
        }
        let after_source = repair.after_source.as_ref().ok_or_else(|| {
            CmuxReviewError::new(format!(
                "finding {} is disposed repaired without an exact resulting source",
                finding.id
            ))
        })?;
        validate_source(after_source, "repair.after_source")?;
        if after_source == source {
            return Err(CmuxReviewError::new(format!(
                "finding {} is disposed repaired but the resulting source is unchanged",
                finding.id
            )));
        }
        let post_verification = repair.verification.as_ref().ok_or_else(|| {
            CmuxReviewError::new(format!(
                "finding {} is disposed repaired without post-repair verification",
                finding.id
            ))
        })?;
        if post_verification.result != CmuxReviewPostRepairVerificationResult::Passed
            || post_verification.evidence.is_empty()
        {
            return Err(CmuxReviewError::new(format!(
                "finding {} is disposed repaired without passed, evidence-bearing post-repair verification",
                finding.id
            )));
        }
        for evidence in &post_verification.evidence {
            validate_nonempty(&evidence.summary, "post-repair verification evidence summary")?;
        }
        if !matches!(
            finding.verification.result,
            CmuxReviewVerificationResult::Reproduced | CmuxReviewVerificationResult::SupportedStatic
        ) {
            return Err(CmuxReviewError::new(format!(
                "finding {} is disposed repaired without pre-repair evidence support",
                finding.id
            )));
        }
    }

    if finding.disposition == CmuxReviewDisposition::HumanRequired
        && !finding
            .claims
            .iter()
            .any(|claim| claim.kind == ClaimKind::Unknown)
        && finding.challenge.disposition != CmuxReviewChallengeDisposition::Uncertain
        && !matches!(
            finding.verification.result,
            CmuxReviewVerificationResult::Blocked | CmuxReviewVerificationResult::HumanJudgment
        )
    {
        return Err(CmuxReviewError::new(format!(
            "finding {} requires a human but retains no explicit uncertainty or blocked/human verification state",
            finding.id
        )));
    }

    Ok(())
}

fn validate_source(source: &CmuxReviewSource, field: &str) -> Result<(), CmuxReviewError> {
    validate_git_object_id(&source.base_sha, &format!("{field}.base_sha"))?;
    validate_git_object_id(&source.head_sha, &format!("{field}.head_sha"))?;
    validate_sha256(&source.diff_sha256, &format!("{field}.diff_sha256"))?;
    if let Some(ruleset) = &source.ruleset_sha256 {
        validate_sha256(ruleset, &format!("{field}.ruleset_sha256"))?;
    }
    Ok(())
}

fn validate_nonempty(value: &str, field: &str) -> Result<(), CmuxReviewError> {
    if value.is_empty() || value.trim() != value || value.contains('\0') {
        return Err(CmuxReviewError::new(format!(
            "{field} must be a non-empty canonical string"
        )));
    }
    Ok(())
}

fn validate_git_object_id(value: &str, field: &str) -> Result<(), CmuxReviewError> {
    if !matches!(value.len(), 40 | 64) || !is_lower_hex(value) {
        return Err(CmuxReviewError::new(format!(
            "{field} must be an exact 40- or 64-character lowercase Git object id"
        )));
    }
    Ok(())
}

fn validate_sha256(value: &str, field: &str) -> Result<(), CmuxReviewError> {
    if value.len() != 64 || !is_lower_hex(value) {
        return Err(CmuxReviewError::new(format!(
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

fn push_frontier(
    frontier: &mut Vec<CmuxReviewFrontierItem>,
    keys: &mut BTreeSet<(String, CmuxReviewFrontierKind, String)>,
    finding_id: &str,
    kind: CmuxReviewFrontierKind,
    message: String,
) {
    let key = (finding_id.to_string(), kind, message.clone());
    if keys.insert(key) {
        frontier.push(CmuxReviewFrontierItem {
            finding_id: finding_id.to_string(),
            kind,
            message,
        });
    }
}
