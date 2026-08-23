use std::error::Error;
use std::path::Path;

use super::{
    build_active_inventory_analysis_report,
    build_active_inventory_analysis_report_from_bound_bytes, read_bounded_inventory,
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

fn apply_provider_snapshot_applicability(
    mut analysis: AnalysisReport,
    required: &ProviderSnapshotIdentity,
    current_provider_snapshot: Option<&ProviderSnapshotIdentity>,
) -> AnalysisReport {
    let applicability = evaluate_provider_snapshot(required, current_provider_snapshot);

    if applicability.status == ApplicabilityStatus::Applies {
        analysis.claims.push(Claim::new(
            ClaimKind::Derived,
            format!(
                "Provider snapshot `{}` matches the independently supplied current snapshot.",
                required.as_str()
            ),
        ));
        return analysis;
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
                        ClaimKind::Derived,
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

    analysis
}

pub(crate) fn build_active_inventory_analysis_report_with_provider_snapshot(
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

    let analysis = build_active_inventory_analysis_report_from_bound_bytes(root, &bytes, scope)?;
    Ok(apply_provider_snapshot_applicability(
        analysis,
        &required,
        current_provider_snapshot,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identity(byte: char) -> ProviderSnapshotIdentity {
        ProviderSnapshotIdentity::parse(format!("sha256:{}", byte.to_string().repeat(64))).unwrap()
    }

    fn report_with(findings: &[(&str, &str)]) -> AnalysisReport {
        let mut report = AnalysisReport::new("test-provider-applicability", "/repo");
        report.findings.extend(
            findings
                .iter()
                .map(|(kind, title)| Finding::new(*kind, *title)),
        );
        report
    }

    fn has_kind(report: &AnalysisReport, kind: &str) -> bool {
        report.findings.iter().any(|finding| finding.kind == kind)
    }

    #[test]
    fn work_invalid_survives_population_unknown_and_routing_is_withheld() {
        let required = identity('a');
        let report = report_with(&[
            (
                "preflight-inventory-current-work-applicability-invalid",
                "Current work applicability changed",
            ),
            ("preflight-inventory-path-overlap", "Active-change path overlap"),
        ]);

        let result = apply_provider_snapshot_applicability(report, &required, None);

        assert!(has_kind(
            &result,
            "preflight-inventory-current-work-applicability-invalid"
        ));
        assert!(has_kind(
            &result,
            "preflight-inventory-provider-snapshot-unknown"
        ));
        assert!(!has_kind(&result, "preflight-inventory-path-overlap"));
    }

    #[test]
    fn work_unknown_survives_population_invalid_and_routing_is_withheld() {
        let required = identity('a');
        let current = identity('b');
        let report = report_with(&[
            (
                "preflight-inventory-current-work-applicability-unknown",
                "Current work applicability unknown",
            ),
            ("preflight-inventory-path-overlap", "Active-change path overlap"),
        ]);

        let result = apply_provider_snapshot_applicability(report, &required, Some(&current));

        assert!(has_kind(
            &result,
            "preflight-inventory-current-work-applicability-unknown"
        ));
        assert!(has_kind(
            &result,
            "preflight-inventory-provider-snapshot-invalid"
        ));
        assert!(!has_kind(&result, "preflight-inventory-path-overlap"));
    }

    #[test]
    fn population_applies_preserves_upstream_routing_when_work_gate_applies() {
        let required = identity('a');
        let report = report_with(&[(
            "preflight-inventory-path-overlap",
            "Active-change path overlap",
        )]);

        let result =
            apply_provider_snapshot_applicability(report, &required, Some(&required));

        assert!(has_kind(&result, "preflight-inventory-path-overlap"));
        assert!(!has_kind(
            &result,
            "preflight-inventory-provider-snapshot-invalid"
        ));
        assert!(!has_kind(
            &result,
            "preflight-inventory-provider-snapshot-unknown"
        ));
    }
}
