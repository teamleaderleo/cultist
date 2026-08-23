use std::error::Error;
use std::fs::File;
use std::io::Read;
use std::path::Path;

use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::active_changes::build_active_inventory_analysis_report;
use crate::applicability::{
    APPLICABILITY_SCHEMA_VERSION, ApplicabilityQuery, ApplicabilityStatus, DimensionStatus,
    EvaluationContext, EvidenceApplicability, EvidenceRequirements, evaluate_query,
};
use crate::finding::{AnalysisReport, Claim, ClaimKind, Evidence, Finding};
use crate::provider_snapshot_applicability::{
    ProviderSnapshotApplicability, ProviderSnapshotIdentity, evaluate_provider_snapshot,
};

const CONSUMPTION_CONTEXT_SCHEMA_VERSION: u32 = 1;
const MAX_CONTEXT_BYTES: usize = 1024 * 1024;
const MAX_INVENTORY_BYTES: usize = 1024 * 1024;
const SHA256_PREFIX: &str = "sha256:";
const SHA256_HEX_BYTES: usize = 64;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ActiveInventoryConsumptionContext {
    schema_version: u32,
    inventory_sha256: String,
    #[serde(default)]
    current_work: Option<CurrentWorkApplicabilityInput>,
    #[serde(default)]
    provider_snapshot: Option<ProviderSnapshotApplicabilityInput>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CurrentWorkApplicabilityInput {
    #[serde(default)]
    current: Option<WorkCoordinate>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct WorkCoordinate {
    id: String,
    head_sha: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProviderSnapshotApplicabilityInput {
    required: ProviderSnapshotIdentity,
    #[serde(default)]
    current: Option<ProviderSnapshotIdentity>,
}

#[derive(Debug, Deserialize)]
struct InventoryBinding {
    current: InventoryWorkBinding,
}

#[derive(Debug, Deserialize)]
struct InventoryWorkBinding {
    id: String,
    head_sha: String,
}

struct ConsumptionEvaluation {
    work: Option<EvidenceApplicability>,
    provider_snapshot: Option<ProviderSnapshotApplicability>,
    status: ApplicabilityStatus,
}

pub fn build_active_inventory_analysis_report_with_context(
    root: &Path,
    inventory_path: &Path,
    context_path: Option<&Path>,
    scope: Option<&Path>,
) -> Result<AnalysisReport, Box<dyn Error>> {
    let Some(context_path) = context_path else {
        return build_active_inventory_analysis_report(root, inventory_path, scope);
    };

    let inventory_bytes =
        read_bounded(inventory_path, MAX_INVENTORY_BYTES, "active-work inventory")?;
    let context_bytes = read_bounded(
        context_path,
        MAX_CONTEXT_BYTES,
        "active-work consumption context",
    )?;
    let context: ActiveInventoryConsumptionContext = serde_json::from_slice(&context_bytes)?;
    validate_context(&context, &inventory_bytes)?;

    let binding: InventoryBinding = serde_json::from_slice(&inventory_bytes)?;
    let evaluation = evaluate_context(&binding, &context)?;
    let mut report = build_active_inventory_analysis_report(root, inventory_path, scope)?;
    apply_context(&mut report, &context, &evaluation);
    Ok(report)
}

fn read_bounded(path: &Path, limit: usize, label: &str) -> Result<Vec<u8>, Box<dyn Error>> {
    let metadata = std::fs::metadata(path)?;
    if metadata.len() > limit as u64 {
        return Err(format!("{label} exceeds the {limit}-byte limit").into());
    }

    let mut bytes = Vec::new();
    File::open(path)?
        .take((limit + 1) as u64)
        .read_to_end(&mut bytes)?;
    if bytes.len() > limit {
        return Err(format!("{label} exceeds the {limit}-byte limit").into());
    }
    Ok(bytes)
}

fn validate_context(
    context: &ActiveInventoryConsumptionContext,
    inventory_bytes: &[u8],
) -> Result<(), Box<dyn Error>> {
    if context.schema_version != CONSUMPTION_CONTEXT_SCHEMA_VERSION {
        return Err(format!(
            "unsupported active-work consumption context schema {}; expected {CONSUMPTION_CONTEXT_SCHEMA_VERSION}",
            context.schema_version
        )
        .into());
    }
    if context.current_work.is_none() && context.provider_snapshot.is_none() {
        return Err("active-work consumption context must require current_work and/or provider_snapshot applicability".into());
    }
    validate_sha256_identity(&context.inventory_sha256, "inventory_sha256")?;

    let actual = sha256_identity(inventory_bytes);
    if context.inventory_sha256 != actual {
        return Err(format!(
            "active-work consumption context is bound to inventory `{}`, but supplied inventory is `{actual}`",
            context.inventory_sha256
        )
        .into());
    }
    Ok(())
}

fn validate_sha256_identity(value: &str, label: &str) -> Result<(), Box<dyn Error>> {
    let Some(digest) = value.strip_prefix(SHA256_PREFIX) else {
        return Err(format!("{label} must use `sha256:<digest>`").into());
    };
    if digest.len() != SHA256_HEX_BYTES
        || !digest
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
    {
        return Err(format!(
            "{label} sha256 digest must contain exactly 64 lowercase hexadecimal characters"
        )
        .into());
    }
    Ok(())
}

fn sha256_identity(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    format!("sha256:{digest:x}")
}

fn evaluate_context(
    binding: &InventoryBinding,
    context: &ActiveInventoryConsumptionContext,
) -> Result<ConsumptionEvaluation, Box<dyn Error>> {
    let work = context
        .current_work
        .as_ref()
        .map(|input| evaluate_current_work(&binding.current, input))
        .transpose()?;
    let provider_snapshot = context
        .provider_snapshot
        .as_ref()
        .map(|input| evaluate_provider_snapshot(&input.required, input.current.as_ref()));

    let status = combine_statuses(
        work.as_ref().map(|value| value.status),
        provider_snapshot.as_ref().map(|value| value.status),
    );
    Ok(ConsumptionEvaluation {
        work,
        provider_snapshot,
        status,
    })
}

fn evaluate_current_work(
    required: &InventoryWorkBinding,
    input: &CurrentWorkApplicabilityInput,
) -> Result<EvidenceApplicability, Box<dyn Error>> {
    let query = ApplicabilityQuery {
        schema_version: APPLICABILITY_SCHEMA_VERSION,
        requirements: EvidenceRequirements {
            revision: Some(required.head_sha.clone()),
            work: Some(required.id.clone()),
            ..EvidenceRequirements::default()
        },
        context: EvaluationContext {
            revision: input
                .current
                .as_ref()
                .map(|current| current.head_sha.clone()),
            work: input.current.as_ref().map(|current| current.id.clone()),
            ..EvaluationContext::default()
        },
    };
    Ok(evaluate_query(&query)?)
}

fn combine_statuses(
    work: Option<ApplicabilityStatus>,
    provider_snapshot: Option<ApplicabilityStatus>,
) -> ApplicabilityStatus {
    let statuses = [work, provider_snapshot];
    if statuses
        .iter()
        .flatten()
        .any(|status| *status == ApplicabilityStatus::Invalid)
    {
        ApplicabilityStatus::Invalid
    } else if statuses
        .iter()
        .flatten()
        .any(|status| *status == ApplicabilityStatus::Unknown)
    {
        ApplicabilityStatus::Unknown
    } else {
        ApplicabilityStatus::Applies
    }
}

fn apply_context(
    report: &mut AnalysisReport,
    context: &ActiveInventoryConsumptionContext,
    evaluation: &ConsumptionEvaluation,
) {
    report.claims.push(
        Claim::new(
            ClaimKind::Observed,
            format!(
                "Admitted active-work consumption context schema v{CONSUMPTION_CONTEXT_SCHEMA_VERSION} bound to inventory `{}`.",
                context.inventory_sha256
            ),
        )
        .with_evidence(Evidence::new(
            "Consumption context was supplied explicitly; inventory mode did not infer provider-current coordinates from checkout HEAD, repository revision, branch age, or observed_at.",
        )),
    );

    if let Some(work) = &evaluation.work {
        report.claims.push(work_applicability_claim(work));
    }
    if let Some(provider_snapshot) = &evaluation.provider_snapshot {
        report
            .claims
            .push(provider_snapshot_applicability_claim(provider_snapshot));
    }

    if evaluation.status == ApplicabilityStatus::Applies {
        report.claims.push(Claim::new(
            ClaimKind::Derived,
            "Every applicability axis required by the supplied consumption context APPLIES; current-routing interpretation may use the admitted inventory facts.",
        ));
        return;
    }

    let gate_claim = routing_gate_claim(evaluation.status);
    report.claims.push(gate_claim.clone());
    report.findings.push(
        Finding::new(
            "preflight-inventory-applicability-gated",
            "Active-work current applicability not established",
        )
        .with_claim(gate_claim.clone())
        .with_question(
            "Refresh the explicit provider-current context before using this inventory for current routing.",
        ),
    );

    for finding in &mut report.findings {
        match finding.kind.as_str() {
            "preflight-inventory-path-overlap"
            | "preflight-inventory-path-overlap-activity-unknown" => {
                finding.kind = "preflight-inventory-path-overlap-applicability-gated".to_string();
                finding.title =
                    "Path overlap in supplied inventory; current applicability gated".to_string();
                finding.claims.push(gate_claim.clone());
                finding.question = Some(
                    "Refresh the explicit provider-current context before treating this supplied path overlap as a current collision."
                        .to_string(),
                );
            }
            "preflight-explicit-coordination" => {
                finding.kind = "preflight-explicit-coordination-applicability-gated".to_string();
                finding.title =
                    "Explicit coordination in supplied inventory; current applicability gated"
                        .to_string();
                finding.claims.push(gate_claim.clone());
                finding.question = Some(
                    "Refresh the explicit provider-current context before treating this supplied coordination relation as current routing evidence."
                        .to_string(),
                );
            }
            _ => {}
        }
    }
}

fn work_applicability_claim(evaluation: &EvidenceApplicability) -> Claim {
    let mut claim = Claim::new(
        claim_kind_for_status(evaluation.status),
        format!(
            "Provider-current work applicability is {}.",
            status_name(evaluation.status)
        ),
    );
    for dimension in &evaluation.dimensions {
        claim = claim.with_evidence(Evidence::new(format!(
            "work applicability {:?}: required `{}`, actual {}, status {}.",
            dimension.dimension,
            dimension.required,
            dimension
                .actual
                .as_deref()
                .map(|actual| format!("`{actual}`"))
                .unwrap_or_else(|| "UNAVAILABLE".to_string()),
            dimension_status_name(dimension.status)
        )));
    }
    claim
}

fn provider_snapshot_applicability_claim(evaluation: &ProviderSnapshotApplicability) -> Claim {
    Claim::new(
        claim_kind_for_status(evaluation.status),
        format!(
            "Provider active-work population applicability is {}.",
            status_name(evaluation.status)
        ),
    )
    .with_evidence(Evidence::new(format!(
        "required provider snapshot `{}`, actual {}.",
        evaluation.required.as_str(),
        evaluation
            .actual
            .as_ref()
            .map(|actual| format!("`{}`", actual.as_str()))
            .unwrap_or_else(|| "UNAVAILABLE".to_string())
    )))
}

fn routing_gate_claim(status: ApplicabilityStatus) -> Claim {
    Claim::new(
        claim_kind_for_status(status),
        format!(
            "Current-routing interpretation is gated because explicit active-work consumption applicability is {}.",
            status_name(status)
        ),
    )
}

fn claim_kind_for_status(status: ApplicabilityStatus) -> ClaimKind {
    match status {
        ApplicabilityStatus::Unknown => ClaimKind::Unknown,
        ApplicabilityStatus::Applies | ApplicabilityStatus::Invalid => ClaimKind::Derived,
    }
}

fn status_name(status: ApplicabilityStatus) -> &'static str {
    match status {
        ApplicabilityStatus::Applies => "APPLIES",
        ApplicabilityStatus::Invalid => "INVALID",
        ApplicabilityStatus::Unknown => "UNKNOWN",
    }
}

fn dimension_status_name(status: DimensionStatus) -> &'static str {
    match status {
        DimensionStatus::Matched => "matched",
        DimensionStatus::Mismatched => "mismatched",
        DimensionStatus::Missing => "missing",
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    use serde_json::{Value, json};

    use super::*;

    const HEAD_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const HEAD_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    const SNAPSHOT_A: &str =
        "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const SNAPSHOT_B: &str =
        "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

    fn root(name: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "cultist-active-inventory-context-{name}-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&root).unwrap();
        root
    }

    fn inventory(root: &Path, activity: &str, with_edge: bool) -> PathBuf {
        let mut document = json!({
            "schema_version": 1,
            "source": "test:active-inventory-context",
            "observed_at": "2026-08-23T00:00:00Z",
            "current": {
                "id": "pull/10",
                "kind": "pull_request",
                "title": "current",
                "url": "https://example.invalid/pull/10",
                "head_ref": "feature/current",
                "head_sha": HEAD_A,
                "updated_at": "2026-08-23T00:00:00Z",
                "draft": false,
                "activity": "confirmed_active",
                "changed_paths": ["src/lib.rs"]
            },
            "active_work": [{
                "id": "pull/20",
                "kind": "pull_request",
                "title": "other",
                "url": "https://example.invalid/pull/20",
                "head_ref": "feature/other",
                "head_sha": HEAD_B,
                "updated_at": "2026-08-23T00:00:01Z",
                "draft": false,
                "activity": activity,
                "changed_paths": ["src/lib.rs"]
            }],
            "coordination_edges": []
        });
        if with_edge {
            document["coordination_edges"] = json!([{
                "kind": "depends_on",
                "from": "pull/10",
                "to": "pull/20",
                "source": "fixture"
            }]);
        }
        let path = root.join("inventory.json");
        fs::write(&path, serde_json::to_vec_pretty(&document).unwrap()).unwrap();
        path
    }

    fn write_context(root: &Path, inventory: &Path, mut value: Value) -> PathBuf {
        let bytes = fs::read(inventory).unwrap();
        value["inventory_sha256"] = json!(sha256_identity(&bytes));
        let path = root.join("context.json");
        fs::write(&path, serde_json::to_vec_pretty(&value).unwrap()).unwrap();
        path
    }

    fn context(current_work: Value, provider_snapshot: Value) -> Value {
        json!({
            "schema_version": 1,
            "inventory_sha256": "placeholder",
            "current_work": current_work,
            "provider_snapshot": provider_snapshot
        })
    }

    fn has_strong_overlap(report: &AnalysisReport) -> bool {
        report
            .findings
            .iter()
            .any(|finding| finding.kind == "preflight-inventory-path-overlap")
    }

    fn has_gated_overlap(report: &AnalysisReport) -> bool {
        report
            .findings
            .iter()
            .any(|finding| finding.kind == "preflight-inventory-path-overlap-applicability-gated")
    }

    #[test]
    fn legacy_inventory_without_context_preserves_strong_overlap() {
        let root = root("legacy");
        let inventory = inventory(&root, "confirmed_active", false);
        let report =
            build_active_inventory_analysis_report_with_context(&root, &inventory, None, None)
                .unwrap();
        assert!(has_strong_overlap(&report));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn exact_current_work_context_applies_and_preserves_strong_overlap() {
        let root = root("work-applies");
        let inventory = inventory(&root, "confirmed_active", false);
        let context = write_context(
            &root,
            &inventory,
            context(
                json!({"current": {"id": "pull/10", "head_sha": HEAD_A}}),
                Value::Null,
            ),
        );
        let report = build_active_inventory_analysis_report_with_context(
            &root,
            &inventory,
            Some(&context),
            None,
        )
        .unwrap();
        assert!(has_strong_overlap(&report));
        assert!(report.claims.iter().any(|claim| {
            claim
                .message
                .contains("Provider-current work applicability is APPLIES")
        }));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn moved_current_work_head_gates_strong_overlap() {
        let root = root("work-invalid");
        let inventory = inventory(&root, "confirmed_active", false);
        let context = write_context(
            &root,
            &inventory,
            context(
                json!({"current": {"id": "pull/10", "head_sha": HEAD_B}}),
                Value::Null,
            ),
        );
        let report = build_active_inventory_analysis_report_with_context(
            &root,
            &inventory,
            Some(&context),
            None,
        )
        .unwrap();
        assert!(!has_strong_overlap(&report));
        assert!(has_gated_overlap(&report));
        assert!(report.claims.iter().any(|claim| {
            claim
                .message
                .contains("Provider-current work applicability is INVALID")
        }));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn unavailable_current_work_gates_as_unknown() {
        let root = root("work-unknown");
        let inventory = inventory(&root, "confirmed_active", false);
        let context = write_context(
            &root,
            &inventory,
            context(json!({"current": null}), Value::Null),
        );
        let report = build_active_inventory_analysis_report_with_context(
            &root,
            &inventory,
            Some(&context),
            None,
        )
        .unwrap();
        assert!(!has_strong_overlap(&report));
        assert!(report.claims.iter().any(|claim| {
            claim
                .message
                .contains("Provider-current work applicability is UNKNOWN")
        }));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn exact_provider_snapshot_applies_and_preserves_strong_overlap() {
        let root = root("snapshot-applies");
        let inventory = inventory(&root, "confirmed_active", false);
        let context = write_context(
            &root,
            &inventory,
            context(
                Value::Null,
                json!({"required": SNAPSHOT_A, "current": SNAPSHOT_A}),
            ),
        );
        let report = build_active_inventory_analysis_report_with_context(
            &root,
            &inventory,
            Some(&context),
            None,
        )
        .unwrap();
        assert!(has_strong_overlap(&report));
        assert!(report.claims.iter().any(|claim| {
            claim
                .message
                .contains("Provider active-work population applicability is APPLIES")
        }));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn moved_provider_snapshot_gates_strong_overlap() {
        let root = root("snapshot-invalid");
        let inventory = inventory(&root, "confirmed_active", false);
        let context = write_context(
            &root,
            &inventory,
            context(
                Value::Null,
                json!({"required": SNAPSHOT_A, "current": SNAPSHOT_B}),
            ),
        );
        let report = build_active_inventory_analysis_report_with_context(
            &root,
            &inventory,
            Some(&context),
            None,
        )
        .unwrap();
        assert!(!has_strong_overlap(&report));
        assert!(has_gated_overlap(&report));
        assert!(report.claims.iter().any(|claim| {
            claim
                .message
                .contains("Provider active-work population applicability is INVALID")
        }));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn unavailable_provider_snapshot_gates_as_unknown() {
        let root = root("snapshot-unknown");
        let inventory = inventory(&root, "confirmed_active", false);
        let context = write_context(
            &root,
            &inventory,
            context(
                Value::Null,
                json!({"required": SNAPSHOT_A, "current": null}),
            ),
        );
        let report = build_active_inventory_analysis_report_with_context(
            &root,
            &inventory,
            Some(&context),
            None,
        )
        .unwrap();
        assert!(!has_strong_overlap(&report));
        assert!(report.claims.iter().any(|claim| {
            claim
                .message
                .contains("Provider active-work population applicability is UNKNOWN")
        }));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn invalid_precedes_unknown_when_required_axes_disagree() {
        let root = root("precedence");
        let inventory = inventory(&root, "confirmed_active", false);
        let context = write_context(
            &root,
            &inventory,
            context(
                json!({"current": null}),
                json!({"required": SNAPSHOT_A, "current": SNAPSHOT_B}),
            ),
        );
        let report = build_active_inventory_analysis_report_with_context(
            &root,
            &inventory,
            Some(&context),
            None,
        )
        .unwrap();
        assert!(report.claims.iter().any(|claim| {
            claim
                .message
                .contains("Current-routing interpretation is gated because explicit active-work consumption applicability is INVALID")
        }));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn inventory_binding_mismatch_fails_closed() {
        let root = root("binding");
        let inventory = inventory(&root, "confirmed_active", false);
        let context_path = root.join("context.json");
        fs::write(
            &context_path,
            serde_json::to_vec_pretty(&json!({
                "schema_version": 1,
                "inventory_sha256": SNAPSHOT_A,
                "current_work": {"current": {"id": "pull/10", "head_sha": HEAD_A}}
            }))
            .unwrap(),
        )
        .unwrap();
        let error = build_active_inventory_analysis_report_with_context(
            &root,
            &inventory,
            Some(&context_path),
            None,
        )
        .unwrap_err();
        assert!(error.to_string().contains("bound to inventory"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn unresolved_activity_semantics_survive_applicable_context() {
        let root = root("activity");
        let inventory = inventory(&root, "unresolved", false);
        let context = write_context(
            &root,
            &inventory,
            context(
                json!({"current": {"id": "pull/10", "head_sha": HEAD_A}}),
                Value::Null,
            ),
        );
        let report = build_active_inventory_analysis_report_with_context(
            &root,
            &inventory,
            Some(&context),
            None,
        )
        .unwrap();
        assert!(report.findings.iter().any(|finding| {
            finding.kind == "preflight-inventory-path-overlap-activity-unknown"
        }));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn explicit_coordination_is_downgraded_when_applicability_is_gated() {
        let root = root("coordination");
        let inventory = inventory(&root, "confirmed_active", true);
        let context = write_context(
            &root,
            &inventory,
            context(
                Value::Null,
                json!({"required": SNAPSHOT_A, "current": SNAPSHOT_B}),
            ),
        );
        let report = build_active_inventory_analysis_report_with_context(
            &root,
            &inventory,
            Some(&context),
            None,
        )
        .unwrap();
        assert!(report.findings.iter().any(|finding| {
            finding.kind == "preflight-explicit-coordination-applicability-gated"
        }));
        assert!(
            !report
                .findings
                .iter()
                .any(|finding| finding.kind == "preflight-explicit-coordination")
        );
        fs::remove_dir_all(root).unwrap();
    }
}
