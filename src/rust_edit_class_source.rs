use std::error::Error;
use std::fmt;
use std::path::{Component, Path, PathBuf};
use std::process::Command;

use serde::{Deserialize, Serialize};

use crate::applicability::{
    APPLICABILITY_SCHEMA_VERSION, ApplicabilityQuery, ApplicabilityStatus, EvaluationContext,
    EvidenceRequirements, PathScope, PathScopeMode, evaluate_query,
};
use crate::discriminator_observation::{
    DISCRIMINATOR_OBSERVATION_SCHEMA_VERSION, DiscriminatorObservation,
    DiscriminatorObservationBatch, DiscriminatorValueState, ObservationApplicability,
    ObservationApplicabilityStatus, validate_discriminator_observation_batch,
};
use crate::durable_obligation::DiscriminatorKey;
use crate::evidence_planner::{EvidenceProbe, ProbeCost, ProbeEffect};
use crate::observation_probe_bridge::ObservationProbeBridge;

#[allow(dead_code)]
#[path = "../examples/rust_syntax_cohort.rs"]
mod rust_syntax_cohort;

use rust_syntax_cohort::{RustEditClass, classify_rust_edit};

pub const RUST_EDIT_CLASS_DISCRIMINATOR_ID: &str = "edit_class";
pub const RUST_EDIT_CLASS_PROBE_KIND: &str = "rust_edit_class";
const MAX_REPOSITORY_BYTES: usize = 256;
const MAX_PATH_BYTES: usize = 2048;
const SHA_BYTES: usize = 40;

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RustEditClassSubject {
    pub repository: String,
    pub revision: String,
    pub path: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RustEditClassSourceResult {
    pub subject: RustEditClassSubject,
    pub current_head: String,
    pub bridge: ObservationProbeBridge,
    pub probe: EvidenceProbe,
    pub observation: DiscriminatorObservation,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct RustEditClassSourceError {
    message: String,
}

impl RustEditClassSourceError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for RustEditClassSourceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for RustEditClassSourceError {}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum FocusAdmission {
    Focused,
    NotSingleParent,
    AnchorUnchanged,
}

pub fn collect_rust_edit_class_source(
    root: &Path,
    current_repository: &str,
    subject: &RustEditClassSubject,
) -> Result<RustEditClassSourceResult, RustEditClassSourceError> {
    validate_subject(subject)?;
    validate_repository(current_repository, "current repository")?;
    let root = root.canonicalize().map_err(|error| {
        RustEditClassSourceError::new(format!("cannot canonicalize repository root: {error}"))
    })?;
    let current_head = git_head(&root)?;
    let path = PathBuf::from(&subject.path);
    let subject_ref = subject_ref(subject);
    let requirements = exact_requirements(subject);
    let context = EvaluationContext {
        repository: Some(current_repository.to_string()),
        revision: Some(current_head.clone()),
        work: None,
        path: Some(subject.path.clone()),
    };
    let applicability = evaluate_query(&ApplicabilityQuery {
        schema_version: APPLICABILITY_SCHEMA_VERSION,
        requirements: requirements.clone(),
        context,
    })
    .map_err(|error| {
        RustEditClassSourceError::new(format!("edit-class applicability failed: {error}"))
    })?;

    let value_state = match focus_admission(&root, &subject.revision, &path)? {
        FocusAdmission::Focused => match classify_rust_edit(&root, &subject.revision, &path) {
            RustEditClass::SyntaxChanged => DiscriminatorValueState::Known {
                value_ref: "syntax_changed".to_string(),
            },
            RustEditClass::CommentsOrWhitespaceOnly => DiscriminatorValueState::Known {
                value_ref: "comments_or_docs_only".to_string(),
            },
            RustEditClass::Unclassified => DiscriminatorValueState::Unknown {
                reason_ref: format!("rust-edit-class:unclassified:{subject_ref}"),
            },
        },
        FocusAdmission::NotSingleParent => DiscriminatorValueState::Unknown {
            reason_ref: format!("rust-edit-class:not-single-parent:{subject_ref}"),
        },
        FocusAdmission::AnchorUnchanged => DiscriminatorValueState::Unknown {
            reason_ref: format!("rust-edit-class:anchor-unchanged:{subject_ref}"),
        },
    };
    let applicability_status = match applicability.status {
        ApplicabilityStatus::Applies => ObservationApplicabilityStatus::Applies,
        ApplicabilityStatus::Unknown => ObservationApplicabilityStatus::Unknown,
        ApplicabilityStatus::Invalid => ObservationApplicabilityStatus::Invalid,
    };
    let applicability_ref = format!(
        "rust-edit-class:applicability:{subject_ref}:current={current_repository}@{current_head}"
    );
    let source_receipt = format!("rust-syntax-cohort:{subject_ref}");
    let observation = DiscriminatorObservation {
        observation_id: format!("rust-edit-class:{subject_ref}"),
        discriminator_id: RUST_EDIT_CLASS_DISCRIMINATOR_ID.to_string(),
        subject_ref: subject_ref.clone(),
        source_receipt,
        value_state,
        applicability: ObservationApplicability {
            status: applicability_status,
            receipt_ref: applicability_ref,
        },
    };
    validate_discriminator_observation_batch(&DiscriminatorObservationBatch {
        schema_version: DISCRIMINATOR_OBSERVATION_SCHEMA_VERSION,
        observations: vec![observation.clone()],
    })
    .map_err(|error| {
        RustEditClassSourceError::new(format!("produced observation failed validation: {error}"))
    })?;

    let probe_discriminator = DiscriminatorKey {
        kind: RUST_EDIT_CLASS_PROBE_KIND.to_string(),
        target: subject_ref.clone(),
    };
    let suffix = &subject.revision[..12];
    let bridge = ObservationProbeBridge {
        bridge_id: format!("rust-edit-class-{suffix}"),
        observation_discriminator_id: RUST_EDIT_CLASS_DISCRIMINATOR_ID.to_string(),
        observation_subject_ref: subject_ref.clone(),
        probe_discriminator: probe_discriminator.clone(),
        clearing_requirements: requirements.clone(),
        source_receipt: format!("rust-edit-class-adapter:{subject_ref}"),
    };
    let probe = EvidenceProbe {
        id: format!("rust-edit-class-{suffix}"),
        produces: probe_discriminator,
        requirements,
        effect: ProbeEffect::ReadOnly,
        cost: ProbeCost {
            git_subprocesses: 5,
            rust_files_parsed: 2,
            ..ProbeCost::default()
        },
    };

    Ok(RustEditClassSourceResult {
        subject: subject.clone(),
        current_head,
        bridge,
        probe,
        observation,
    })
}

pub fn subject_ref(subject: &RustEditClassSubject) -> String {
    format!(
        "{}@{}:{}",
        subject.repository, subject.revision, subject.path
    )
}

fn exact_requirements(subject: &RustEditClassSubject) -> EvidenceRequirements {
    EvidenceRequirements {
        repository: Some(subject.repository.clone()),
        revision: Some(subject.revision.clone()),
        work: None,
        scope: Some(PathScope {
            mode: PathScopeMode::Exact,
            path: subject.path.clone(),
        }),
    }
}

fn git_head(root: &Path) -> Result<String, RustEditClassSourceError> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["rev-parse", "HEAD"])
        .output()
        .map_err(|error| {
            RustEditClassSourceError::new(format!("cannot execute git rev-parse: {error}"))
        })?;
    if !output.status.success() {
        return Err(RustEditClassSourceError::new(format!(
            "git rev-parse HEAD failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )));
    }
    let head = String::from_utf8(output.stdout)
        .map_err(|error| RustEditClassSourceError::new(format!("invalid git head UTF-8: {error}")))?
        .trim()
        .to_string();
    validate_revision(&head, "current git head")?;
    Ok(head)
}

fn focus_admission(
    root: &Path,
    revision: &str,
    path: &Path,
) -> Result<FocusAdmission, RustEditClassSourceError> {
    let parents = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["rev-list", "--parents", "-n", "1", revision])
        .output()
        .map_err(|error| {
            RustEditClassSourceError::new(format!("cannot inspect edit-class parents: {error}"))
        })?;
    if !parents.status.success() {
        return Err(RustEditClassSourceError::new(format!(
            "git rev-list failed for {revision}: {}",
            String::from_utf8_lossy(&parents.stderr)
        )));
    }
    let parents = String::from_utf8(parents.stdout).map_err(|error| {
        RustEditClassSourceError::new(format!("invalid parent-list UTF-8: {error}"))
    })?;
    if parents.split_whitespace().count() != 2 {
        return Ok(FocusAdmission::NotSingleParent);
    }

    let changed = Command::new("git")
        .arg("-C")
        .arg(root)
        .args([
            "diff-tree",
            "--no-commit-id",
            "--name-only",
            "-r",
            revision,
            "--",
        ])
        .arg(path)
        .output()
        .map_err(|error| {
            RustEditClassSourceError::new(format!("cannot inspect edit-class path change: {error}"))
        })?;
    if !changed.status.success() {
        return Err(RustEditClassSourceError::new(format!(
            "git diff-tree failed for {revision}: {}",
            String::from_utf8_lossy(&changed.stderr)
        )));
    }
    if String::from_utf8(changed.stdout)
        .map_err(|error| {
            RustEditClassSourceError::new(format!("invalid changed-path UTF-8: {error}"))
        })?
        .lines()
        .all(|line| line.trim().is_empty())
    {
        return Ok(FocusAdmission::AnchorUnchanged);
    }
    Ok(FocusAdmission::Focused)
}

fn validate_subject(subject: &RustEditClassSubject) -> Result<(), RustEditClassSourceError> {
    validate_repository(&subject.repository, "subject repository")?;
    validate_revision(&subject.revision, "subject revision")?;
    if subject.path.is_empty()
        || subject.path.trim() != subject.path
        || subject.path.len() > MAX_PATH_BYTES
        || subject.path.contains(['\n', '\r', '\0', '\\'])
    {
        return Err(RustEditClassSourceError::new(
            "path must be bounded canonical repository-relative text",
        ));
    }
    let path = Path::new(&subject.path);
    if path.is_absolute()
        || path.extension().and_then(|extension| extension.to_str()) != Some("rs")
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(RustEditClassSourceError::new(
            "path must be a normalized repository-relative Rust source path",
        ));
    }
    let reference = subject_ref(subject);
    if reference.len() > 480 {
        return Err(RustEditClassSourceError::new(
            "edit-class subject reference exceeds the admitted boundary",
        ));
    }
    Ok(())
}

fn validate_repository(value: &str, label: &str) -> Result<(), RustEditClassSourceError> {
    if value.is_empty()
        || value.trim() != value
        || value.len() > MAX_REPOSITORY_BYTES
        || value.contains(['\n', '\r', '\0'])
    {
        return Err(RustEditClassSourceError::new(format!(
            "{label} must be bounded canonical single-line text"
        )));
    }
    Ok(())
}

fn validate_revision(value: &str, label: &str) -> Result<(), RustEditClassSourceError> {
    if value.len() != SHA_BYTES
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(RustEditClassSourceError::new(format!(
            "{label} must be an exact lowercase 40-hex Git commit"
        )));
    }
    Ok(())
}
