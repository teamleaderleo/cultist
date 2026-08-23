use std::env;
use std::error::Error;
use std::path::Path;

const REQUIRED_PROVIDER_REPOSITORY_ENV: &str = "CULTIST_REQUIRED_PROVIDER_REPOSITORY";
const CURRENT_PROVIDER_REPOSITORY_ENV: &str = "CULTIST_CURRENT_PROVIDER_REPOSITORY";
const CURRENT_PROVIDER_WORK_ENV: &str = "CULTIST_CURRENT_PROVIDER_WORK";
const CURRENT_PROVIDER_HEAD_ENV: &str = "CULTIST_CURRENT_PROVIDER_HEAD";

mod inventory {
    include!("active_changes/inventory.rs");

    #[allow(dead_code)]
    mod applicability {
        include!("applicability.rs");
    }

    mod provider_current {
        include!("active_changes/provider_current.rs");
    }

    pub(crate) use provider_current::{
        ProviderCurrentWorkContext, build_active_inventory_analysis_report_with_provider_current,
        build_active_inventory_analysis_report_with_provider_current_from_bound_bytes,
    };
}

#[path = "active_changes/provider_snapshot.rs"]
mod provider_snapshot;
pub(crate) use provider_snapshot::build_active_inventory_analysis_report_with_provider_snapshot;

#[allow(unused_imports)]
pub(crate) use inventory::{
    ProviderCurrentWorkContext, build_active_inventory_analysis_report_with_provider_current,
};

fn read_bounded_inventory(path: &Path) -> Result<Vec<u8>, Box<dyn Error>> {
    inventory::read_bounded_inventory(path)
}

fn build_active_inventory_analysis_report_from_bound_bytes(
    root: &Path,
    bytes: &[u8],
    scope: Option<&Path>,
) -> Result<crate::finding::AnalysisReport, Box<dyn Error>> {
    let provider_current = provider_current_from_environment()?;
    match provider_current {
        Some((required_repository, context)) => {
            inventory::build_active_inventory_analysis_report_with_provider_current_from_bound_bytes(
                root,
                bytes,
                scope,
                &required_repository,
                &context,
            )
        }
        None => {
            inventory::build_active_inventory_analysis_report_from_bound_bytes(root, bytes, scope)
        }
    }
}

pub fn build_active_inventory_analysis_report(
    root: &Path,
    inventory_path: &Path,
    scope: Option<&Path>,
) -> Result<crate::finding::AnalysisReport, Box<dyn Error>> {
    let bytes = read_bounded_inventory(inventory_path)?;
    let raw: serde_json::Value = serde_json::from_slice(&bytes)?;
    if raw.get("provider_snapshot_identity").is_some() {
        return Err(
            "provider-snapshot-bound inventory requires an explicit current provider snapshot context"
                .into(),
        );
    }
    build_active_inventory_analysis_report_from_bound_bytes(root, &bytes, scope)
}

fn provider_current_from_environment()
-> Result<Option<(String, ProviderCurrentWorkContext)>, Box<dyn Error>> {
    let required_repository = environment_value(REQUIRED_PROVIDER_REPOSITORY_ENV)?;
    let current_repository = environment_value(CURRENT_PROVIDER_REPOSITORY_ENV)?;
    let work_id = environment_value(CURRENT_PROVIDER_WORK_ENV)?;
    let head_sha = environment_value(CURRENT_PROVIDER_HEAD_ENV)?;
    provider_current_from_values(required_repository, current_repository, work_id, head_sha)
}

fn environment_value(name: &str) -> Result<Option<String>, Box<dyn Error>> {
    match env::var(name) {
        Ok(value) => Ok(Some(value)),
        Err(env::VarError::NotPresent) => Ok(None),
        Err(env::VarError::NotUnicode(_)) => {
            Err(format!("`{name}` must contain valid Unicode text").into())
        }
    }
}

fn provider_current_from_values(
    required_repository: Option<String>,
    current_repository: Option<String>,
    work_id: Option<String>,
    head_sha: Option<String>,
) -> Result<Option<(String, ProviderCurrentWorkContext)>, Box<dyn Error>> {
    let any = required_repository.is_some()
        || current_repository.is_some()
        || work_id.is_some()
        || head_sha.is_some();
    if !any {
        return Ok(None);
    }

    let required_repository = required_repository.ok_or_else(|| {
        format!("provider-current gating requires `{REQUIRED_PROVIDER_REPOSITORY_ENV}`")
    })?;
    let current_repository = current_repository.ok_or_else(|| {
        format!("provider-current gating requires `{CURRENT_PROVIDER_REPOSITORY_ENV}`")
    })?;
    let work_id = work_id
        .ok_or_else(|| format!("provider-current gating requires `{CURRENT_PROVIDER_WORK_ENV}`"))?;

    Ok(Some((
        required_repository,
        ProviderCurrentWorkContext {
            repository: current_repository,
            work_id,
            head_sha,
        },
    )))
}

#[cfg(test)]
mod provider_current_environment_tests {
    use super::*;

    #[test]
    fn absent_provider_current_environment_preserves_legacy_path() {
        assert_eq!(
            provider_current_from_values(None, None, None, None).unwrap(),
            None
        );
    }

    #[test]
    fn work_without_head_represents_unknown_current_head() {
        assert_eq!(
            provider_current_from_values(
                Some("owner/repo".to_string()),
                Some("owner/repo".to_string()),
                Some("#10".to_string()),
                None,
            )
            .unwrap(),
            Some((
                "owner/repo".to_string(),
                ProviderCurrentWorkContext {
                    repository: "owner/repo".to_string(),
                    work_id: "#10".to_string(),
                    head_sha: None,
                },
            ))
        );
    }

    #[test]
    fn full_provider_current_binding_is_preserved() {
        assert_eq!(
            provider_current_from_values(
                Some("owner/repo".to_string()),
                Some("owner/repo".to_string()),
                Some("#10".to_string()),
                Some("abc".to_string()),
            )
            .unwrap(),
            Some((
                "owner/repo".to_string(),
                ProviderCurrentWorkContext {
                    repository: "owner/repo".to_string(),
                    work_id: "#10".to_string(),
                    head_sha: Some("abc".to_string()),
                },
            ))
        );
    }

    #[test]
    fn partial_provider_current_binding_fails_closed() {
        assert!(
            provider_current_from_values(
                Some("owner/repo".to_string()),
                None,
                Some("#10".to_string()),
                Some("abc".to_string()),
            )
            .is_err()
        );
        assert!(
            provider_current_from_values(
                Some("owner/repo".to_string()),
                Some("owner/repo".to_string()),
                None,
                Some("abc".to_string()),
            )
            .is_err()
        );
    }
}
