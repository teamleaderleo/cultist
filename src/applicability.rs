use std::error::Error;
use std::fmt;

use serde::{Deserialize, Serialize};

pub const APPLICABILITY_SCHEMA_VERSION: u32 = 1;
pub const MAX_APPLICABILITY_QUERY_BYTES: usize = 1024 * 1024;
const MAX_COORDINATE_BYTES: usize = 1024;
const MAX_PATH_BYTES: usize = 4096;

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ApplicabilityQuery {
    pub schema_version: u32,
    pub requirements: EvidenceRequirements,
    pub context: EvaluationContext,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize, Default)]
#[serde(deny_unknown_fields)]
pub struct EvidenceRequirements {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repository: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revision: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub work: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<PathScope>,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PathScope {
    pub mode: PathScopeMode,
    pub path: String,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PathScopeMode {
    Exact,
    Prefix,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize, Default)]
#[serde(deny_unknown_fields)]
pub struct EvaluationContext {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repository: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revision: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub work: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EvidenceApplicability {
    pub schema_version: u32,
    pub status: ApplicabilityStatus,
    pub dimensions: Vec<DimensionEvaluation>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ApplicabilityStatus {
    Applies,
    Invalid,
    Unknown,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ApplicabilityDimension {
    Repository,
    Revision,
    Work,
    Scope,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DimensionStatus {
    Matched,
    Mismatched,
    Missing,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DimensionEvaluation {
    pub dimension: ApplicabilityDimension,
    pub status: DimensionStatus,
    pub required: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actual: Option<String>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ApplicabilityError {
    message: String,
}

impl ApplicabilityError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for ApplicabilityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for ApplicabilityError {}

pub fn evaluate_exact_coordinate(required: &str, actual: Option<&str>) -> ApplicabilityStatus {
    match actual {
        Some(actual) if actual == required => ApplicabilityStatus::Applies,
        Some(_) => ApplicabilityStatus::Invalid,
        None => ApplicabilityStatus::Unknown,
    }
}

pub fn evaluate_query(
    query: &ApplicabilityQuery,
) -> Result<EvidenceApplicability, ApplicabilityError> {
    if query.schema_version != APPLICABILITY_SCHEMA_VERSION {
        return Err(ApplicabilityError::new(format!(
            "unsupported applicability schema {}; expected {APPLICABILITY_SCHEMA_VERSION}",
            query.schema_version
        )));
    }

    validate_requirements(&query.requirements)?;
    validate_context(&query.context)?;

    let mut dimensions = Vec::new();

    if let Some(required) = &query.requirements.repository {
        dimensions.push(evaluate_exact_dimension(
            ApplicabilityDimension::Repository,
            required,
            query.context.repository.as_deref(),
        ));
    }
    if let Some(required) = &query.requirements.revision {
        dimensions.push(evaluate_exact_dimension(
            ApplicabilityDimension::Revision,
            required,
            query.context.revision.as_deref(),
        ));
    }
    if let Some(required) = &query.requirements.work {
        dimensions.push(evaluate_exact_dimension(
            ApplicabilityDimension::Work,
            required,
            query.context.work.as_deref(),
        ));
    }
    if let Some(required) = &query.requirements.scope {
        dimensions.push(evaluate_scope_dimension(
            required,
            query.context.path.as_deref(),
        ));
    }

    let status = if dimensions
        .iter()
        .any(|dimension| dimension.status == DimensionStatus::Mismatched)
    {
        ApplicabilityStatus::Invalid
    } else if dimensions
        .iter()
        .any(|dimension| dimension.status == DimensionStatus::Missing)
    {
        ApplicabilityStatus::Unknown
    } else {
        ApplicabilityStatus::Applies
    };

    Ok(EvidenceApplicability {
        schema_version: APPLICABILITY_SCHEMA_VERSION,
        status,
        dimensions,
    })
}

fn evaluate_exact_dimension(
    dimension: ApplicabilityDimension,
    required: &str,
    actual: Option<&str>,
) -> DimensionEvaluation {
    let status = match evaluate_exact_coordinate(required, actual) {
        ApplicabilityStatus::Applies => DimensionStatus::Matched,
        ApplicabilityStatus::Invalid => DimensionStatus::Mismatched,
        ApplicabilityStatus::Unknown => DimensionStatus::Missing,
    };
    DimensionEvaluation {
        dimension,
        status,
        required: required.to_string(),
        actual: actual.map(str::to_string),
    }
}

fn evaluate_scope_dimension(required: &PathScope, actual: Option<&str>) -> DimensionEvaluation {
    let required_display = format!("{}:{}", required.mode.as_str(), required.path);
    match actual {
        Some(actual) if required.matches(actual) => DimensionEvaluation {
            dimension: ApplicabilityDimension::Scope,
            status: DimensionStatus::Matched,
            required: required_display,
            actual: Some(actual.to_string()),
        },
        Some(actual) => DimensionEvaluation {
            dimension: ApplicabilityDimension::Scope,
            status: DimensionStatus::Mismatched,
            required: required_display,
            actual: Some(actual.to_string()),
        },
        None => DimensionEvaluation {
            dimension: ApplicabilityDimension::Scope,
            status: DimensionStatus::Missing,
            required: required_display,
            actual: None,
        },
    }
}

impl PathScopeMode {
    fn as_str(self) -> &'static str {
        match self {
            Self::Exact => "exact",
            Self::Prefix => "prefix",
        }
    }
}

impl PathScope {
    fn matches(&self, actual: &str) -> bool {
        match self.mode {
            PathScopeMode::Exact => actual == self.path,
            PathScopeMode::Prefix => {
                actual == self.path
                    || actual
                        .strip_prefix(self.path.as_str())
                        .is_some_and(|suffix| suffix.starts_with('/'))
            }
        }
    }
}

fn validate_requirements(requirements: &EvidenceRequirements) -> Result<(), ApplicabilityError> {
    if requirements.repository.is_none()
        && requirements.revision.is_none()
        && requirements.work.is_none()
        && requirements.scope.is_none()
    {
        return Err(ApplicabilityError::new(
            "at least one explicit applicability requirement is required",
        ));
    }

    validate_coordinate(
        requirements.repository.as_deref(),
        "requirements.repository",
    )?;
    validate_coordinate(requirements.revision.as_deref(), "requirements.revision")?;
    validate_coordinate(requirements.work.as_deref(), "requirements.work")?;
    if let Some(scope) = &requirements.scope {
        validate_repo_path(&scope.path, "requirements.scope.path")?;
    }
    Ok(())
}

fn validate_context(context: &EvaluationContext) -> Result<(), ApplicabilityError> {
    validate_coordinate(context.repository.as_deref(), "context.repository")?;
    validate_coordinate(context.revision.as_deref(), "context.revision")?;
    validate_coordinate(context.work.as_deref(), "context.work")?;
    if let Some(path) = &context.path {
        validate_repo_path(path, "context.path")?;
    }
    Ok(())
}

fn validate_coordinate(value: Option<&str>, field: &str) -> Result<(), ApplicabilityError> {
    let Some(value) = value else {
        return Ok(());
    };
    if value.is_empty() || value.trim() != value {
        return Err(ApplicabilityError::new(format!(
            "{field} must be a non-empty canonical coordinate"
        )));
    }
    if value.len() > MAX_COORDINATE_BYTES || value.contains('\0') {
        return Err(ApplicabilityError::new(format!(
            "{field} exceeds the admitted coordinate boundary"
        )));
    }
    Ok(())
}

fn validate_repo_path(path: &str, field: &str) -> Result<(), ApplicabilityError> {
    if path.is_empty()
        || path.len() > MAX_PATH_BYTES
        || path.starts_with('/')
        || path.ends_with('/')
        || path.contains('\\')
        || path.contains('\0')
        || path
            .split('/')
            .any(|component| component.is_empty() || component == "." || component == "..")
    {
        return Err(ApplicabilityError::new(format!(
            "{field} must be a canonical repository-relative path"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn query(requirements: EvidenceRequirements, context: EvaluationContext) -> ApplicabilityQuery {
        ApplicabilityQuery {
            schema_version: APPLICABILITY_SCHEMA_VERSION,
            requirements,
            context,
        }
    }

    #[test]
    fn exact_provider_coordinate_applies() {
        let evaluation = evaluate_query(&query(
            EvidenceRequirements {
                repository: Some("teamleaderleo/preflight".to_string()),
                revision: Some("a2e14c4265e3568d8f943906a53e3b0e16dca141".to_string()),
                work: Some("#748".to_string()),
                scope: None,
            },
            EvaluationContext {
                repository: Some("teamleaderleo/preflight".to_string()),
                revision: Some("a2e14c4265e3568d8f943906a53e3b0e16dca141".to_string()),
                work: Some("#748".to_string()),
                path: None,
            },
        ))
        .unwrap();

        assert_eq!(evaluation.status, ApplicabilityStatus::Applies);
        assert!(
            evaluation
                .dimensions
                .iter()
                .all(|dimension| dimension.status == DimensionStatus::Matched)
        );
    }

    #[test]
    fn moved_provider_head_invalidates_exact_head_evidence() {
        let evaluation = evaluate_query(&query(
            EvidenceRequirements {
                revision: Some("old-head".to_string()),
                work: Some("#748".to_string()),
                ..EvidenceRequirements::default()
            },
            EvaluationContext {
                revision: Some("new-head".to_string()),
                work: Some("#748".to_string()),
                ..EvaluationContext::default()
            },
        ))
        .unwrap();

        assert_eq!(evaluation.status, ApplicabilityStatus::Invalid);
        assert_eq!(
            evaluation.dimensions[0].dimension,
            ApplicabilityDimension::Revision
        );
        assert_eq!(evaluation.dimensions[0].status, DimensionStatus::Mismatched);
    }

    #[test]
    fn missing_required_head_remains_unknown() {
        let evaluation = evaluate_query(&query(
            EvidenceRequirements {
                revision: Some("exact-head".to_string()),
                work: Some("#748".to_string()),
                ..EvidenceRequirements::default()
            },
            EvaluationContext {
                work: Some("#748".to_string()),
                ..EvaluationContext::default()
            },
        ))
        .unwrap();

        assert_eq!(evaluation.status, ApplicabilityStatus::Unknown);
        assert_eq!(evaluation.dimensions[0].status, DimensionStatus::Missing);
        assert_eq!(evaluation.dimensions[1].status, DimensionStatus::Matched);
    }

    #[test]
    fn known_mismatch_takes_precedence_over_missing_dimension() {
        let evaluation = evaluate_query(&query(
            EvidenceRequirements {
                repository: Some("owner/repo".to_string()),
                revision: Some("head-a".to_string()),
                ..EvidenceRequirements::default()
            },
            EvaluationContext {
                repository: Some("other/repo".to_string()),
                ..EvaluationContext::default()
            },
        ))
        .unwrap();

        assert_eq!(evaluation.status, ApplicabilityStatus::Invalid);
        assert_eq!(evaluation.dimensions[0].status, DimensionStatus::Mismatched);
        assert_eq!(evaluation.dimensions[1].status, DimensionStatus::Missing);
    }

    #[test]
    fn exact_and_prefix_scope_have_distinct_semantics() {
        let exact = PathScope {
            mode: PathScopeMode::Exact,
            path: "src/history.rs".to_string(),
        };
        let prefix = PathScope {
            mode: PathScopeMode::Prefix,
            path: "src/history.rs".to_string(),
        };

        let exact_child = evaluate_query(&query(
            EvidenceRequirements {
                scope: Some(exact),
                ..EvidenceRequirements::default()
            },
            EvaluationContext {
                path: Some("src/history.rs/tests.rs".to_string()),
                ..EvaluationContext::default()
            },
        ))
        .unwrap();
        let prefix_child = evaluate_query(&query(
            EvidenceRequirements {
                scope: Some(prefix),
                ..EvidenceRequirements::default()
            },
            EvaluationContext {
                path: Some("src/history.rs/tests.rs".to_string()),
                ..EvaluationContext::default()
            },
        ))
        .unwrap();

        assert_eq!(exact_child.status, ApplicabilityStatus::Invalid);
        assert_eq!(prefix_child.status, ApplicabilityStatus::Applies);
    }

    #[test]
    fn decision_scope_refuses_unrelated_target() {
        let evaluation = evaluate_query(&query(
            EvidenceRequirements {
                scope: Some(PathScope {
                    mode: PathScopeMode::Exact,
                    path: "src/history.rs".to_string(),
                }),
                ..EvidenceRequirements::default()
            },
            EvaluationContext {
                path: Some("src/main.rs".to_string()),
                ..EvaluationContext::default()
            },
        ))
        .unwrap();

        assert_eq!(evaluation.status, ApplicabilityStatus::Invalid);
    }

    #[test]
    fn missing_delta_base_coordinate_is_unknown() {
        let evaluation = evaluate_query(&query(
            EvidenceRequirements {
                revision: Some("envelope:E41".to_string()),
                ..EvidenceRequirements::default()
            },
            EvaluationContext::default(),
        ))
        .unwrap();

        assert_eq!(evaluation.status, ApplicabilityStatus::Unknown);
    }

    #[test]
    fn dimensions_are_evaluated_in_stable_order() {
        let evaluation = evaluate_query(&query(
            EvidenceRequirements {
                repository: Some("owner/repo".to_string()),
                revision: Some("head".to_string()),
                work: Some("#1".to_string()),
                scope: Some(PathScope {
                    mode: PathScopeMode::Exact,
                    path: "src/lib.rs".to_string(),
                }),
            },
            EvaluationContext::default(),
        ))
        .unwrap();

        assert_eq!(
            evaluation
                .dimensions
                .iter()
                .map(|dimension| dimension.dimension)
                .collect::<Vec<_>>(),
            vec![
                ApplicabilityDimension::Repository,
                ApplicabilityDimension::Revision,
                ApplicabilityDimension::Work,
                ApplicabilityDimension::Scope,
            ]
        );
    }

    #[test]
    fn rejects_noncanonical_paths() {
        for path in [
            "/src/lib.rs",
            "src/../lib.rs",
            "src\\lib.rs",
            "src//lib.rs",
            "./src/lib.rs",
        ] {
            let error = evaluate_query(&query(
                EvidenceRequirements {
                    scope: Some(PathScope {
                        mode: PathScopeMode::Exact,
                        path: path.to_string(),
                    }),
                    ..EvidenceRequirements::default()
                },
                EvaluationContext::default(),
            ))
            .unwrap_err();
            assert!(
                error
                    .to_string()
                    .contains("canonical repository-relative path")
            );
        }
    }

    #[test]
    fn rejects_empty_requirements_instead_of_treating_them_as_global() {
        let error = evaluate_query(&query(
            EvidenceRequirements::default(),
            EvaluationContext::default(),
        ))
        .unwrap_err();
        assert!(
            error
                .to_string()
                .contains("at least one explicit applicability requirement")
        );
    }

    #[test]
    fn rejects_unknown_schema_and_unknown_json_fields() {
        let mut unsupported = query(
            EvidenceRequirements {
                work: Some("#1".to_string()),
                ..EvidenceRequirements::default()
            },
            EvaluationContext::default(),
        );
        unsupported.schema_version = 2;
        assert!(evaluate_query(&unsupported).is_err());

        let json = r#"{
            "schema_version": 1,
            "requirements": {"future_semantics": true},
            "context": {}
        }"#;
        let error = serde_json::from_str::<ApplicabilityQuery>(json).unwrap_err();
        assert!(error.to_string().contains("unknown field"));
    }

    #[test]
    fn query_and_evaluation_round_trip_as_machine_objects() {
        let input = query(
            EvidenceRequirements {
                repository: Some("owner/repo".to_string()),
                scope: Some(PathScope {
                    mode: PathScopeMode::Prefix,
                    path: "src".to_string(),
                }),
                ..EvidenceRequirements::default()
            },
            EvaluationContext {
                repository: Some("owner/repo".to_string()),
                path: Some("src/lib.rs".to_string()),
                ..EvaluationContext::default()
            },
        );
        let query_json = serde_json::to_string(&input).unwrap();
        let decoded_query: ApplicabilityQuery = serde_json::from_str(&query_json).unwrap();
        assert_eq!(decoded_query, input);

        let evaluation = evaluate_query(&input).unwrap();
        let evaluation_json = serde_json::to_string(&evaluation).unwrap();
        let decoded_evaluation: EvidenceApplicability =
            serde_json::from_str(&evaluation_json).unwrap();
        assert_eq!(decoded_evaluation, evaluation);
    }
}
