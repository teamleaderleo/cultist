use std::error::Error;
use std::path::Path;

use crate::active_changes::{
    build_active_inventory_analysis_report, build_active_inventory_analysis_report_from_bound_bytes,
    read_bounded_inventory,
};
use crate::applicability::ApplicabilityStatus;
use crate::finding::{AnalysisReport, Claim, ClaimKind, Evidence, Finding};
use crate::provider_snapshot_applicability::{
    ProviderSnapshotIdentity, evaluate_provider_snapshot,
};

fn required_provider_snapshot(
    bytes: &[u8],
) -> Result<Option<ProviderSnapshotIdentity>, Box<dyn Error>> {
    let value: serde_json::Value = serde_json::from_slice(bytes)?;
    match value.get("provider_snapshot_identity") {
        None => Ok(None),
        Some(serde_json::Value::String(identity)) => {
            Ok(Some(ProviderSnapshotIdentity::parse(identity.clone())?))
        }
        Some(_) => Err(
            "`provider_snapshot_identity` must be a canonical `sha256:<64-lowercase-hex>` string"
                .into(),
        ),
    }
}

fn is_work_applicability_finding(finding: &Finding) -> bool {
    matches!(
        finding.kind.as_str(),
        "preflight-inventory-current-work-applicability-invalid"
            | "preflight-inventory-current-work-applicability-unknown"
    )
}

pub fn build_active_inventory_analysis_report_with_provider_snapshot(
    root: &Path,
    inventory_path: &Path,
    scope: Option<&Path>,
    current_provider_snapshot: Option<&ProviderSnapshotIdentity>,
) -> Result<AnalysisReport, Box<dyn Error>> {
    let bytes = read_bounded_inventory(inventory_path)?;
    let required = required_provider_snapshot(&bytes)?;

    let Some(required) = required else {
        if current_provider_snapshot.is_some() {
            return Err(
                "current provider snapshot was supplied, but the inventory does not bind `provider_snapshot_identity`"
                    .into(),
            );
        }
        return build_active_inventory_analysis_report(root, inventory_path, scope);
    };

    let mut analysis =
        build_active_inventory_analysis_report_from_bound_bytes(root, &bytes, scope)?;
    let applicability = evaluate_provider_snapshot(&required, current_provider_snapshot);

    if applicability.status == ApplicabilityStatus::Applies {
        analysis.claims.push(Claim::new(
            ClaimKind::Derived,
            format!(
                "Provider snapshot `{}` matches the independently supplied current snapshot.",
                required.as_str()
            ),
        ));
        return Ok(analysis);
    }

    let before = analysis.findings.len();
    analysis.findings.retain(is_work_applicability_finding);
    let withheld = before - analysis.findings.len();
    analysis.claims.push(Claim::new(
        ClaimKind::Observed,
        format!(
            "The frozen inventory binds provider snapshot `{}`.",
            required.as_str()
        ),
    ));

    match applicability.status {
        ApplicabilityStatus::Invalid => {
            let actual = applicability
                .actual
                .as_ref()
                .expect("INVALID provider snapshot applicability has an actual identity");
            analysis.claims.push(Claim::new(
                ClaimKind::Derived,
                format!(
                    "Withheld {withheld} frozen routing finding(s) because current provider snapshot `{}` differs from required snapshot `{}`.",
                    actual.as_str(),
                    required.as_str()
                ),
            ));
            analysis.findings.push(
                Finding::new(
                    "preflight-inventory-provider-snapshot-invalid",
                    "Provider active-work snapshot changed",
                )
                .with_claim(
                    Claim::new(
                        ClaimKind::Observed,
                        "The independently supplied provider population identity differs from the frozen inventory requirement.",
                    )
                    .with_evidence(Evidence::new(format!(
                        "Required provider snapshot: `{}`.",
                        required.as_str()
                    )))
                    .with_evidence(Evidence::new(format!(
                        "Current provider snapshot: `{}`.",
                        actual.as_str()
                    ))),
                )
                .with_question(
                    "Refresh the active-work inventory from the current provider population before routing on its collision evidence.",
                ),
            );
        }
        ApplicabilityStatus::Unknown => {
            analysis.claims.push(Claim::new(
                ClaimKind::Unknown,
                format!(
                    "Withheld {withheld} frozen routing finding(s) because the current provider snapshot identity is unavailable."
                ),
            ));
            analysis.findings.push(
                Finding::new(
                    "preflight-inventory-provider-snapshot-unknown",
                    "Provider active-work snapshot currentness unknown",
                )
                .with_claim(
                    Claim::new(
                        ClaimKind::Unknown,
                        "The current provider population identity was not supplied, so the frozen inventory cannot be treated as current routing evidence.",
                    )
                    .with_evidence(Evidence::new(format!(
                        "Required provider snapshot: `{}`.",
                        required.as_str()
                    ))),
                )
                .with_question(
                    "Supply an independently derived current provider snapshot or refresh the active-work inventory before routing on collision evidence.",
                ),
            );
        }
        ApplicabilityStatus::Applies => unreachable!(),
    }

    Ok(analysis)
}
