#[allow(dead_code)]
#[path = "../src/compact_ir.rs"]
mod compact_ir;
#[allow(dead_code)]
#[path = "../src/finding.rs"]
mod finding;
#[allow(dead_code)]
#[path = "../src/report_fingerprint.rs"]
mod report_fingerprint;

use finding::{AnalysisReport, Claim, ClaimKind, REPORT_SCHEMA_VERSION};
use report_fingerprint::{ReportFingerprint, fingerprint_report};

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
struct AmbientContext<'a> {
    repository: Option<&'a str>,
    revision: Option<&'a str>,
    task: Option<&'a str>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
struct ContextRequirements {
    repository: bool,
    revision: bool,
    task: bool,
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct BoundEnvelopeIdentity {
    report: ReportFingerprint,
    repository: Option<String>,
    revision: Option<String>,
    task: Option<String>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum BindError {
    NoRequirements,
    MissingRepository,
    MissingRevision,
    MissingTask,
}

fn bind_context(
    report: &AnalysisReport,
    context: AmbientContext<'_>,
    requirements: ContextRequirements,
) -> Result<BoundEnvelopeIdentity, BindError> {
    if !requirements.repository && !requirements.revision && !requirements.task {
        return Err(BindError::NoRequirements);
    }

    let repository = required_value(
        context.repository,
        requirements.repository,
        BindError::MissingRepository,
    )?;
    let revision = required_value(
        context.revision,
        requirements.revision,
        BindError::MissingRevision,
    )?;
    let task = required_value(context.task, requirements.task, BindError::MissingTask)?;

    Ok(BoundEnvelopeIdentity {
        report: fingerprint_report(report).expect("research fixture report should fingerprint"),
        repository,
        revision,
        task,
    })
}

fn required_value(
    value: Option<&str>,
    required: bool,
    missing: BindError,
) -> Result<Option<String>, BindError> {
    if required {
        return value.map(|value| Some(value.to_string())).ok_or(missing);
    }
    Ok(None)
}

fn report_without_revision_semantics() -> AnalysisReport {
    AnalysisReport {
        schema_version: REPORT_SCHEMA_VERSION,
        analysis: "context-binding-research".to_string(),
        repository: "teamleaderleo/cultist".to_string(),
        claims: vec![Claim::new(
            ClaimKind::Observed,
            "the selected local convention is `tests`",
        )],
        findings: Vec::new(),
    }
}

#[test]
fn report_fingerprint_does_not_change_when_only_external_revision_moves() {
    let report = report_without_revision_semantics();
    let before = AmbientContext {
        repository: Some("teamleaderleo/cultist"),
        revision: Some("aaaaaaaa"),
        task: Some("T1"),
    };
    let after = AmbientContext {
        revision: Some("bbbbbbbb"),
        ..before
    };

    assert_ne!(before, after);
    assert_eq!(
        fingerprint_report(&report).unwrap(),
        fingerprint_report(&report).unwrap()
    );
}

#[test]
fn requiring_revision_makes_the_bound_identity_change_when_head_moves() {
    let report = report_without_revision_semantics();
    let requirements = ContextRequirements {
        repository: true,
        revision: true,
        task: false,
    };
    let before = bind_context(
        &report,
        AmbientContext {
            repository: Some("teamleaderleo/cultist"),
            revision: Some("aaaaaaaa"),
            task: Some("T1"),
        },
        requirements,
    )
    .unwrap();
    let after = bind_context(
        &report,
        AmbientContext {
            repository: Some("teamleaderleo/cultist"),
            revision: Some("bbbbbbbb"),
            task: Some("T1"),
        },
        requirements,
    )
    .unwrap();

    assert_eq!(before.report, after.report);
    assert_ne!(before, after);
    assert_ne!(before.revision, after.revision);
}

#[test]
fn missing_required_revision_fails_closed_instead_of_guessing_from_ambient_state() {
    let report = report_without_revision_semantics();
    let result = bind_context(
        &report,
        AmbientContext {
            repository: Some("teamleaderleo/cultist"),
            revision: None,
            task: Some("T1"),
        },
        ContextRequirements {
            repository: true,
            revision: true,
            task: false,
        },
    );

    assert_eq!(result, Err(BindError::MissingRevision));
}

#[test]
fn empty_requirements_fail_closed_instead_of_meaning_context_free_global() {
    let report = report_without_revision_semantics();
    let result = bind_context(
        &report,
        AmbientContext {
            repository: Some("teamleaderleo/cultist"),
            revision: Some("aaaaaaaa"),
            task: Some("T1"),
        },
        ContextRequirements {
            repository: false,
            revision: false,
            task: false,
        },
    );

    assert_eq!(result, Err(BindError::NoRequirements));
}

#[test]
fn unrequired_context_dimensions_do_not_pollute_the_bound_identity() {
    let report = report_without_revision_semantics();
    let requirements = ContextRequirements {
        repository: true,
        revision: false,
        task: false,
    };
    let first = bind_context(
        &report,
        AmbientContext {
            repository: Some("teamleaderleo/cultist"),
            revision: Some("aaaaaaaa"),
            task: Some("T1"),
        },
        requirements,
    )
    .unwrap();
    let second = bind_context(
        &report,
        AmbientContext {
            repository: Some("teamleaderleo/cultist"),
            revision: Some("bbbbbbbb"),
            task: Some("T2"),
        },
        requirements,
    )
    .unwrap();

    assert_eq!(first, second);
    assert_eq!(first.revision, None);
    assert_eq!(first.task, None);
}

#[test]
fn report_snapshot_identity_and_context_identity_remain_separate_values() {
    let report = report_without_revision_semantics();
    let bound = bind_context(
        &report,
        AmbientContext {
            repository: Some("teamleaderleo/cultist"),
            revision: Some("aaaaaaaa"),
            task: Some("T1"),
        },
        ContextRequirements {
            repository: true,
            revision: true,
            task: true,
        },
    )
    .unwrap();

    assert_eq!(bound.report, fingerprint_report(&report).unwrap());
    assert_eq!(bound.repository.as_deref(), Some("teamleaderleo/cultist"));
    assert_eq!(bound.revision.as_deref(), Some("aaaaaaaa"));
    assert_eq!(bound.task.as_deref(), Some("T1"));
}
