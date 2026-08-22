#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

#[path = "../src/applicability.rs"]
mod applicability;
#[path = "../src/discriminator_observation.rs"]
mod discriminator_observation;
#[path = "../src/durable_obligation.rs"]
mod durable_obligation;
#[path = "../src/evidence_planner.rs"]
mod evidence_planner;
#[path = "../src/justification.rs"]
mod justification;
#[path = "../src/observation_frontier.rs"]
mod observation_frontier;
#[path = "../src/observation_probe_bridge.rs"]
mod observation_probe_bridge;
#[path = "../src/rust_edit_class_source.rs"]
mod rust_edit_class_source;

use applicability::EvaluationContext;
use discriminator_observation::{
    DISCRIMINATOR_OBSERVATION_SCHEMA_VERSION, DiscriminatorObservationBatch,
    DiscriminatorValueState, ObservationApplicabilityStatus, parse_discriminator_observation_batch,
};
use evidence_planner::{EvidencePlanStatus, ProbeEffect, ProbeSelectionPolicy};
use observation_frontier::{
    OBSERVATION_FRONTIER_SCHEMA_VERSION, ObservationFrontierRequest, ObservationFrontierStatus,
    ObservationRequirement, evaluate_observation_frontiers,
};
use observation_probe_bridge::{
    OBSERVATION_PROBE_BRIDGE_SCHEMA_VERSION, ObservationProbePlanRequest,
    ObservationProbePlanStatus, plan_observation_probe,
};
use rust_edit_class_source::{RustEditClassSubject, collect_rust_edit_class_source, subject_ref};

struct TempRepo {
    root: PathBuf,
}

impl TempRepo {
    fn new(label: &str) -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "cultist-rust-edit-class-{label}-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(root.join("src")).unwrap();
        run_git(&root, &["init", "-q"]);
        run_git(&root, &["config", "user.email", "cultist@example.invalid"]);
        run_git(&root, &["config", "user.name", "Cultist Test"]);
        Self { root }
    }

    fn write(&self, source: &str) {
        fs::write(self.root.join("src/lib.rs"), source).unwrap();
    }

    fn commit(&self, message: &str) -> String {
        run_git(&self.root, &["add", "src/lib.rs"]);
        run_git(&self.root, &["commit", "-q", "-m", message]);
        git_output(&self.root, &["rev-parse", "HEAD"])
    }

    fn commit_other_path(&self, message: &str) -> String {
        fs::write(self.root.join("README.md"), "unrelated\n").unwrap();
        run_git(&self.root, &["add", "README.md"]);
        run_git(&self.root, &["commit", "-q", "-m", message]);
        git_output(&self.root, &["rev-parse", "HEAD"])
    }
}

impl Drop for TempRepo {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn run_git(root: &Path, args: &[&str]) {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn git_output(root: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap().trim().to_string()
}

fn subject(revision: &str) -> RustEditClassSubject {
    RustEditClassSubject {
        repository: "owner/repo".to_string(),
        revision: revision.to_string(),
        path: "src/lib.rs".to_string(),
    }
}

fn seeded_repo() -> (TempRepo, String, String, String) {
    let repo = TempRepo::new("cohorts");
    repo.write("fn answer() -> usize { 41 }\n");
    let root = repo.commit("root");
    repo.write("fn answer() -> usize { 42 }\n");
    let syntax = repo.commit("syntax");
    repo.write("// retained comment\nfn answer() -> usize { 42 }\n");
    let comments = repo.commit("comments");
    (repo, root, syntax, comments)
}

#[test]
fn comment_only_edit_emits_current_known_observation_and_exact_bridge() {
    let (repo, _root, _syntax, comments) = seeded_repo();
    let result = collect_rust_edit_class_source(&repo.root, &subject(&comments)).unwrap();

    assert_eq!(result.current_head, comments);
    assert_eq!(result.observation.discriminator_id, "edit_class");
    assert_eq!(
        result.observation.value_state,
        DiscriminatorValueState::Known {
            value_ref: "comments_or_docs_only".to_string()
        }
    );
    assert_eq!(
        result.observation.applicability.status,
        ObservationApplicabilityStatus::Applies
    );
    assert_eq!(
        result.bridge.observation_subject_ref,
        subject_ref(&subject(&comments))
    );
    assert_eq!(result.probe.effect, ProbeEffect::ReadOnly);
}

#[test]
fn syntax_commit_is_classified_separately_from_comment_only_commit() {
    let (repo, _root, syntax, _comments) = seeded_repo();
    run_git(&repo.root, &["checkout", "-q", &syntax]);
    let result = collect_rust_edit_class_source(&repo.root, &subject(&syntax)).unwrap();
    assert_eq!(
        result.observation.value_state,
        DiscriminatorValueState::Known {
            value_ref: "syntax_changed".to_string()
        }
    );
    assert_eq!(
        result.observation.applicability.status,
        ObservationApplicabilityStatus::Applies
    );
}

#[test]
fn root_commit_is_unknown_instead_of_guessed() {
    let (repo, root, _syntax, _comments) = seeded_repo();
    run_git(&repo.root, &["checkout", "-q", &root]);
    let result = collect_rust_edit_class_source(&repo.root, &subject(&root)).unwrap();
    assert!(matches!(
        result.observation.value_state,
        DiscriminatorValueState::Unknown { .. }
    ));
    assert_eq!(
        result.observation.applicability.status,
        ObservationApplicabilityStatus::Applies
    );
}

#[test]
fn unrelated_repository_commit_is_unknown_instead_of_comment_only() {
    let (repo, _root, _syntax, _comments) = seeded_repo();
    let unrelated = repo.commit_other_path("unrelated");
    let result = collect_rust_edit_class_source(&repo.root, &subject(&unrelated)).unwrap();
    assert!(matches!(
        &result.observation.value_state,
        DiscriminatorValueState::Unknown { reason_ref }
            if reason_ref.contains("anchor-unchanged")
    ));
    assert_eq!(
        result.observation.applicability.status,
        ObservationApplicabilityStatus::Applies
    );
}

#[test]
fn known_old_classification_stays_invalid_after_head_moves() {
    let (repo, _root, syntax, comments) = seeded_repo();
    assert_eq!(git_output(&repo.root, &["rev-parse", "HEAD"]), comments);
    let result = collect_rust_edit_class_source(&repo.root, &subject(&syntax)).unwrap();
    assert_eq!(
        result.observation.value_state,
        DiscriminatorValueState::Known {
            value_ref: "syntax_changed".to_string()
        }
    );
    assert_eq!(
        result.observation.applicability.status,
        ObservationApplicabilityStatus::Invalid
    );
}

#[test]
fn selected_probe_then_produced_observation_closes_missing_frontier() {
    let (repo, _root, _syntax, comments) = seeded_repo();
    let source = collect_rust_edit_class_source(&repo.root, &subject(&comments)).unwrap();
    let required_subject = source.observation.subject_ref.clone();
    let missing = observation_frontier::ObservationFrontierReceipt {
        discriminator_id: "edit_class".to_string(),
        subject_ref: required_subject.clone(),
        status: ObservationFrontierStatus::Missing,
        current: Vec::new(),
        unknown: Vec::new(),
        invalid: Vec::new(),
        other_subject: Vec::new(),
    };

    let plan = plan_observation_probe(&ObservationProbePlanRequest {
        schema_version: OBSERVATION_PROBE_BRIDGE_SCHEMA_VERSION,
        frontier: missing,
        bridges: vec![source.bridge.clone()],
        context: EvaluationContext {
            repository: Some("owner/repo".to_string()),
            revision: Some(comments.clone()),
            work: None,
            path: Some("src/lib.rs".to_string()),
        },
        probes: vec![source.probe.clone()],
        allow_effectful: false,
        policy: ProbeSelectionPolicy::Conservative,
    })
    .unwrap();
    assert_eq!(plan.status, ObservationProbePlanStatus::Planned);
    assert_eq!(
        plan.evidence_plan.unwrap().status,
        EvidencePlanStatus::Selected
    );

    let evaluation = evaluate_observation_frontiers(&ObservationFrontierRequest {
        schema_version: OBSERVATION_FRONTIER_SCHEMA_VERSION,
        requirements: vec![ObservationRequirement {
            discriminator_id: "edit_class".to_string(),
            subject_ref: required_subject,
        }],
        observations: DiscriminatorObservationBatch {
            schema_version: DISCRIMINATOR_OBSERVATION_SCHEMA_VERSION,
            observations: vec![source.observation.clone()],
        },
    })
    .unwrap();
    assert_eq!(
        evaluation.frontiers[0].status,
        ObservationFrontierStatus::Current
    );
    assert_eq!(
        evaluation.frontiers[0].current[0].value_ref,
        "comments_or_docs_only"
    );
}

#[test]
fn produced_observation_round_trips_through_v2_validation() {
    let (repo, _root, _syntax, comments) = seeded_repo();
    let result = collect_rust_edit_class_source(&repo.root, &subject(&comments)).unwrap();
    let batch = DiscriminatorObservationBatch {
        schema_version: DISCRIMINATOR_OBSERVATION_SCHEMA_VERSION,
        observations: vec![result.observation],
    };
    let encoded = serde_json::to_vec(&batch).unwrap();
    let decoded = parse_discriminator_observation_batch(&encoded).unwrap();
    assert_eq!(decoded, batch);
}

#[test]
fn path_and_revision_admission_fail_closed() {
    let (repo, _root, _syntax, comments) = seeded_repo();
    let mut bad_path = subject(&comments);
    bad_path.path = "../src/lib.rs".to_string();
    assert!(collect_rust_edit_class_source(&repo.root, &bad_path).is_err());

    let mut short_revision = subject(&comments);
    short_revision.revision = "deadbeef".to_string();
    assert!(collect_rust_edit_class_source(&repo.root, &short_revision).is_err());
}
