use std::error::Error;
use std::path::Path;

use super::applicability::{
    APPLICABILITY_SCHEMA_VERSION, ApplicabilityQuery, ApplicabilityStatus, EvaluationContext,
    EvidenceApplicability, EvidenceRequirements, evaluate_query,
};
use crate::finding::{AnalysisReport, Claim, ClaimKind, Evidence, Finding};

use super::{
    INVENTORY_SCHEMA_VERSION, MAX_SHA_BYTES, ValidatedInventory, analyze_inventory,
    read_bounded_inventory, validate_bounded_text, validate_id, validate_inventory,
};

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ProviderCurrentWorkContext {
    pub repository: String,
    pub work_id: String,
    pub head_sha: Option<String>,
}

pub fn build_active_inventory_analysis_report_with_provider_current(
    root: &Path,
    inventory_path: &Path,
    scope: Option<&Path>,
    required_repository: &str,
    provider_current: &ProviderCurrentWorkContext,
) -> Result<AnalysisReport, Box<dyn Error>> {
    validate_provider_current(provider_current)?;

    let bytes = read_bounded_inventory(inventory_path)?;
    let inventory = validate_inventory(serde_json::from_slice(&bytes)?)?;
    let applicability =
        evaluate_provider_current(&inventory, required_repository, provider_current)?;

    match applicability.status {
        ApplicabilityStatus::Applies => {
            let mut analysis = analyze_inventory(root, &inventory, scope);
            analysis.claims.push(Claim::new(
                ClaimKind::Derived,
                format!(
                    "The supplied provider-current context matches frozen work `{}` at head `{}` in repository `{required_repository}`.",
                    inventory.current.id, inventory.current.head_sha
                ),
            ));
            Ok(analysis)
        }
        ApplicabilityStatus::Invalid | ApplicabilityStatus::Unknown => Ok(gated_analysis(
            root,
            &inventory,
            required_repository,
            provider_current,
            &applicability,
        )),
    }
}

fn validate_provider_current(
    provider_current: &ProviderCurrentWorkContext,
) -> Result<(), Box<dyn Error>> {
    validate_id(&provider_current.work_id)?;
    if let Some(head_sha) = &provider_current.head_sha {
        validate_bounded_text(head_sha, "provider-current head sha", MAX_SHA_BYTES, true)?;
    }
    Ok(())
}

fn evaluate_provider_current(
    inventory: &ValidatedInventory,
    required_repository: &str,
    provider_current: &ProviderCurrentWorkContext,
) -> Result<EvidenceApplicability, Box<dyn Error>> {
    Ok(evaluate_query(&ApplicabilityQuery {
        schema_version: APPLICABILITY_SCHEMA_VERSION,
        requirements: EvidenceRequirements {
            repository: Some(required_repository.to_string()),
            revision: Some(inventory.current.head_sha.clone()),
            work: Some(inventory.current.id.clone()),
            scope: None,
        },
        context: EvaluationContext {
            repository: Some(provider_current.repository.clone()),
            revision: provider_current.head_sha.clone(),
            work: Some(provider_current.work_id.clone()),
            path: None,
        },
    })?)
}

fn gated_analysis(
    root: &Path,
    inventory: &ValidatedInventory,
    required_repository: &str,
    provider_current: &ProviderCurrentWorkContext,
    applicability: &EvidenceApplicability,
) -> AnalysisReport {
    let mut analysis = AnalysisReport::new(
        "preflight-active-inventory",
        root.to_string_lossy().into_owned(),
    );

    analysis.claims.push(Claim::new(
        ClaimKind::Observed,
        format!(
            "Admitted active-work inventory schema v{INVENTORY_SCHEMA_VERSION} from `{}` observed at `{}`.",
            inventory.source, inventory.observed_at
        ),
    ));
    analysis.claims.push(Claim::new(
        ClaimKind::Observed,
        format!(
            "Frozen current work `{}` is `{}` at head `{}` (updated `{}`, draft={}).",
            inventory.current.id,
            inventory.current.title,
            inventory.current.head_sha,
            inventory.current.updated_at,
            inventory.current.draft
        ),
    ));

    let actual_head = provider_current
        .head_sha
        .as_deref()
        .unwrap_or("<unavailable>");
    let evidence = Evidence::new(format!(
        "Frozen provider work coordinate: repository=`{required_repository}` work=`{}` head=`{}`; supplied provider-current coordinate: repository=`{}` work=`{}` head=`{actual_head}`.",
        inventory.current.id,
        inventory.current.head_sha,
        provider_current.repository,
        provider_current.work_id
    ));

    let finding = match applicability.status {
        ApplicabilityStatus::Invalid => Finding::new(
            "preflight-inventory-current-work-applicability-invalid",
            "Frozen active-work current coordinate moved",
        )
        .with_claim(
            Claim::new(
                ClaimKind::Derived,
                "Shared applicability evaluation is INVALID for the frozen current-work coordinate.",
            )
            .with_evidence(evidence),
        )
        .with_question(
            "Refresh the active-work inventory before treating its path or coordination facts as current routing evidence.",
        ),
        ApplicabilityStatus::Unknown => Finding::new(
            "preflight-inventory-current-work-applicability-unknown",
            "Current active-work applicability unresolved",
        )
        .with_claim(
            Claim::new(
                ClaimKind::Unknown,
                "The supplied provider-current context does not establish whether the frozen current-work coordinate still applies.",
            )
            .with_evidence(evidence),
        )
        .with_question(
            "Resolve the provider-current work coordinate before treating frozen path or coordination facts as current routing evidence.",
        ),
        ApplicabilityStatus::Applies => unreachable!("applicable context is analyzed normally"),
    };

    analysis.findings.push(finding);
    analysis.claims.push(Claim::new(
        ClaimKind::Derived,
        "Current-routing path overlap and explicit-coordination findings were withheld by the provider-current applicability gate.",
    ));
    analysis
}
