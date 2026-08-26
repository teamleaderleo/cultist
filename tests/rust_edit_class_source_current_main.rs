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

use applicability::{ApplicabilityStatus, EvaluationContext};
use discriminator_observation::{DiscriminatorValueState, ObservationApplicabilityStatus};
use evidence_planner::{EvidencePlanStatus, ProbeSelectionPolicy};
use observation_frontier::{
    OBSERVATION_FRONTIER_SCHEMA_VERSION, ObservationFrontierRequest, ObservationFrontierStatus,
    ObservationRequirement, evaluate_observation_frontiers,
};
use observation_probe_bridge::{
    OBSERVATION_PROBE_BRIDGE_SCHEMA_VERSION, ObservationProbePlanRequest,
    ObservationProbePlanStatus, plan_observation_probe,
};
use rust_edit_class_source::{
    RustEditClassSourceResult, RustEditClassSubject, collect_rust_edit_class_source,
};

const CURRENT_REPOSITORY: &str = "owner/repo";

struct TempRepo(PathBuf);

impl TempRepo {
    fn new(default_branch: &str) -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "cultist-rust-edit-class-current-main-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(root.join("src")).unwrap();
        let output = Command::new("git")
            .arg("-C")
            .arg(&root)
            .arg("-c")
            .arg(format!("init.defaultBranch={default_branch}"))
            .args(["init", "-q"])
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        git(&root, &["config", "user.email", "cultist@example.invalid"]);
        git(&root, &["config", "user.name", "Cultist Test"]);
        Self(root)
    }

    fn write(&self, source: &str) {
        fs::write(self.0.join("src/lib.rs"), source).unwrap();
    }

    fn commit(&self, message: &str) -> String {
        git(&self.0, &["add", "src/lib.rs"]);
        git(&self.0, &["commit", "-q", "-m", message]);
        git_text(&self.0, &["rev-parse", "HEAD"])
    }

    fn commit_unrelated(&self) -> String {
        fs::write(self.0.join("README.md"), "unrelated\n").unwrap();
        git(&self.0, &["add", "README.md"]);
        git(&self.0, &["commit", "-q", "-m", "unrelated"]);
        git_text(&self.0, &["rev-parse", "HEAD"])
    }
}

impl Drop for TempRepo {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn git(root: &Path, args: &[&str]) {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn git_text(root: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap().trim().to_string()
}

fn subject(revision: &str) -> RustEditClassSubject {
    RustEditClassSubject {
        repository: CURRENT_REPOSITORY.to_string(),
        revision: revision.to_string(),
        path: "src/lib.rs".to_string(),
    }
}

fn collect(repo: &TempRepo, subject: &RustEditClassSubject) -> RustEditClassSourceResult {
    collect_rust_edit_class_source(&repo.0, CURRENT_REPOSITORY, subject).unwrap()
}

fn seeded(default_branch: &str) -> (TempRepo, String, String, String) {
    let repo = TempRepo::new(default_branch);
    repo.write("fn answer() -> usize { 41 }\n");
    let root = repo.commit("root");
    repo.write("fn answer() -> usize { 42 }\n");
    let syntax = repo.commit("syntax");
    repo.write("// comment\nfn answer() -> usize { 42 }\n");
    let comments = repo.commit("comments");
    (repo, root, syntax, comments)
}

fn assert_focused_source_values_and_focus_admission(default_branch: &str) {
    let (repo, root, syntax, comments) = seeded(default_branch);
    let initial_branch = git_text(&repo.0, &["symbolic-ref", "--short", "HEAD"]);
    assert_eq!(initial_branch, default_branch);

    git(&repo.0, &["checkout", "-q", &syntax]);
    let syntax_result = collect(&repo, &subject(&syntax));
    assert_eq!(
        syntax_result.observation.value_state,
        DiscriminatorValueState::Known {
            value_ref: "syntax_changed".to_string()
        }
    );
    assert_eq!(
        syntax_result.observation.applicability.status,
        ObservationApplicabilityStatus::Applies
    );

    git(&repo.0, &["checkout", "-q", &comments]);
    let comments_result = collect(&repo, &subject(&comments));
    assert_eq!(
        comments_result.observation.value_state,
        DiscriminatorValueState::Known {
            value_ref: "comments_or_docs_only".to_string()
        }
    );

    git(&repo.0, &["checkout", "-q", &root]);
    let root_result = collect(&repo, &subject(&root));
    assert!(matches!(
        root_result.observation.value_state,
        DiscriminatorValueState::Unknown { .. }
    ));

    git(&repo.0, &["checkout", "-q", &initial_branch]);
    let unrelated = repo.commit_unrelated();
    let unrelated_result = collect(&repo, &subject(&unrelated));
    assert!(matches!(
        &unrelated_result.observation.value_state,
        DiscriminatorValueState::Unknown { reason_ref } if reason_ref.contains("anchor-unchanged")
    ));
}

#[test]
fn focused_source_values_and_focus_admission_survive_current_main() {
    for default_branch in ["main", "master"] {
        assert_focused_source_values_and_focus_admission(default_branch);
    }
}

#[test]
fn source_bridge_uses_v2_consumption_applicability_and_closes_missing_frontier() {
    let (repo, _root, _syntax, comments) = seeded("main");
    let source = collect(&repo, &subject(&comments));
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
        frontier_requirements: source.bridge.clearing_requirements.clone(),
        bridges: vec![source.bridge.clone()],
        context: EvaluationContext {
            repository: Some(CURRENT_REPOSITORY.to_string()),
            revision: Some(comments.clone()),
            work: None,
            path: Some("src/lib.rs".to_string()),
        },
        probes: vec![source.probe.clone()],
        allow_effectful: false,
        policy: ProbeSelectionPolicy::Conservative,
    })
    .unwrap();

    assert_eq!(plan.applicability_status, ApplicabilityStatus::Applies);
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
        observations: discriminator_observation::DiscriminatorObservationBatch {
            schema_version: discriminator_observation::DISCRIMINATOR_OBSERVATION_SCHEMA_VERSION,
            observations: vec![source.observation],
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
fn repository_and_head_movement_are_independent_applicability_axes() {
    let (repo, _root, syntax, comments) = seeded("main");

    let wrong_repository =
        collect_rust_edit_class_source(&repo.0, "owner/other", &subject(&comments)).unwrap();
    assert_eq!(
        wrong_repository.observation.value_state,
        DiscriminatorValueState::Known {
            value_ref: "comments_or_docs_only".to_string()
        }
    );
    assert_eq!(
        wrong_repository.observation.applicability.status,
        ObservationApplicabilityStatus::Invalid
    );
    assert!(
        wrong_repository
            .observation
            .applicability
            .receipt_ref
            .contains("current=owner/other@")
    );

    let moved_head =
        collect_rust_edit_class_source(&repo.0, CURRENT_REPOSITORY, &subject(&syntax)).unwrap();
    assert_eq!(
        moved_head.observation.applicability.status,
        ObservationApplicabilityStatus::Invalid
    );
    assert_eq!(git_text(&repo.0, &["rev-parse", "HEAD"]), comments);
}

#[test]
fn bad_source_and_subject_coordinates_fail_closed() {
    let (repo, _root, _syntax, comments) = seeded("main");

    assert!(collect_rust_edit_class_source(&repo.0, " owner/repo", &subject(&comments)).is_err());

    let mut bad_path = subject(&comments);
    bad_path.path = "../src/lib.rs".to_string();
    assert!(collect_rust_edit_class_source(&repo.0, CURRENT_REPOSITORY, &bad_path).is_err());

    let mut short_revision = subject(&comments);
    short_revision.revision = "deadbeef".to_string();
    assert!(collect_rust_edit_class_source(&repo.0, CURRENT_REPOSITORY, &short_revision).is_err());
}
