use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};

const QUARRY_MAIN: &str = "26f3ab7e4dc223b91524b94595592eee5cb7ed1a";
const OBSERVED_AT: &str = "2026-08-22T16:40:14Z";

#[allow(clippy::too_many_arguments)]
fn work(
    id: &str,
    kind: &str,
    activity: &str,
    title: &str,
    url: &str,
    head_ref: &str,
    head_sha: &str,
    updated_at: &str,
    draft: bool,
    changed_paths: &[&str],
) -> Value {
    json!({
        "id": id,
        "kind": kind,
        "activity": activity,
        "title": title,
        "url": url,
        "head_ref": head_ref,
        "head_sha": head_sha,
        "updated_at": updated_at,
        "draft": draft,
        "changed_paths": changed_paths,
    })
}

fn quarry_work() -> Vec<Value> {
    vec![
        work(
            "quarry-pr-604",
            "delivery_candidate",
            "confirmed_active",
            "Pilot agent-native outcome ownership on current main",
            "https://github.com/Coreys-Quarry/quarry/pull/604",
            "kiln/530-agent-native-current-v2",
            "63eece80df17a97a8544c4d716feca4fad1970ea",
            "2026-08-22T16:25:52Z",
            false,
            &["AGENTS.md", "docs/agent-native-operating-mode.md"],
        ),
        work(
            "quarry-pr-608",
            "delivery_candidate",
            "confirmed_active",
            "perf: add minimal exact research IR envelope",
            "https://github.com/Coreys-Quarry/quarry/pull/608",
            "perf/564-minimal-research-ir",
            "515c60f694664f3b691bfd7f920e4740d75226d1",
            "2026-08-22T16:26:25Z",
            false,
            &["src/quarry/research_ir.py", "tests/test_research_ir.py"],
        ),
        work(
            "quarry-prep-532",
            "preparation_branch",
            "preparation",
            "Issue 532 private continuation work-ahead",
            "https://github.com/Coreys-Quarry/quarry/issues/532",
            "wip/532-agent-continuation-context",
            "5e353896a3fa383866b0398125ce8409fd0ddb2b",
            "2026-08-16T10:02:56Z",
            true,
            &[
                "src/quarry/agent_continuation_context.py",
                "tests/test_agent_continuation_context.py",
                "tests/test_agent_continuation_context_direct_object.py",
                "tests/test_agent_continuation_context_visibility.py",
            ],
        ),
        work(
            "quarry-branch-391",
            "branch_observation_ambiguous",
            "unresolved",
            "Issue 391 branch observation; current activity unresolved",
            "https://github.com/Coreys-Quarry/quarry/issues/391",
            "codex/391-control-dirty-consumers-current",
            "bac4be013432f1386756028bfcc4bf1c2c6aa637",
            "2026-08-16T05:39:09Z",
            true,
            &["tests/test_control_dirty_receipt_consumers.py"],
        ),
        work(
            "quarry-branch-471",
            "branch_observation_ambiguous",
            "unresolved",
            "Issue 471 branch observation; current activity unresolved",
            "https://github.com/Coreys-Quarry/quarry/issues/471",
            "codex/471-target-decision-id-current-v3",
            "66a5ae7b6dbf6005377821cb2920e8638c2494f0",
            "2026-08-16T05:36:43Z",
            true,
            &["tests/test_exact_research_target_decision_identity_types.py"],
        ),
    ]
}

fn unique_inventory(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "cultist-quarry-activity-replay-{name}-{}-{nanos}.json",
        std::process::id()
    ))
}

fn run_case(name: &str, paths: &[&str]) -> Value {
    let inventory_path = unique_inventory(name);
    let inventory = json!({
        "schema_version": 1,
        "source": "Quarry #596 public Phase-0 replay observed 2026-08-22T16:40:14Z",
        "observed_at": OBSERVED_AT,
        "current": {
            "id": format!("chat-{name}"),
            "kind": "chat_lane",
            "activity": "confirmed_active",
            "title": name,
            "url": "https://github.com/Coreys-Quarry/quarry/issues/596",
            "head_ref": "main",
            "head_sha": QUARRY_MAIN,
            "updated_at": OBSERVED_AT,
            "draft": false,
            "changed_paths": paths,
        },
        "active_work": quarry_work(),
        "coordination_edges": [],
    });
    fs::write(
        &inventory_path,
        serde_json::to_vec_pretty(&inventory).unwrap(),
    )
    .unwrap();

    let root = std::env::current_dir().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_cargo-cultist"))
        .args(["preflight", "--inventory"])
        .arg(&inventory_path)
        .args(["--format", "json"])
        .arg(root)
        .output()
        .unwrap();
    let _ = fs::remove_file(&inventory_path);
    assert!(
        output.status.success(),
        "{name} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

fn finding_kinds(report: &Value) -> Vec<&str> {
    report["findings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|finding| finding["kind"].as_str().unwrap())
        .collect()
}

#[test]
fn quarry_phase0_activity_replay_preserves_decision_boundary() {
    for (name, paths) in [
        ("confirmed-policy", &["AGENTS.md"][..]),
        ("confirmed-ir", &["src/quarry/research_ir.py"][..]),
        (
            "preparation-532",
            &["src/quarry/agent_continuation_context.py"][..],
        ),
    ] {
        let report = run_case(name, paths);
        assert!(
            finding_kinds(&report).contains(&"preflight-inventory-path-overlap"),
            "{name}: {report}"
        );
    }

    for (name, path) in [
        (
            "unresolved-391",
            "tests/test_control_dirty_receipt_consumers.py",
        ),
        (
            "unresolved-471",
            "tests/test_exact_research_target_decision_identity_types.py",
        ),
    ] {
        let report = run_case(name, &[path]);
        let kinds = finding_kinds(&report);
        assert!(
            kinds.contains(&"preflight-inventory-path-overlap-activity-unknown"),
            "{name}: {report}"
        );
        assert!(!kinds.contains(&"preflight-inventory-path-overlap"));
        let finding = report["findings"]
            .as_array()
            .unwrap()
            .iter()
            .find(|finding| finding["kind"] == "preflight-inventory-path-overlap-activity-unknown")
            .unwrap();
        assert!(finding["claims"].as_array().unwrap().iter().any(|claim| {
            claim["kind"] == "unknown"
                && claim["message"]
                    .as_str()
                    .unwrap()
                    .contains("currently active or owned")
        }));
        assert!(
            finding["question"]
                .as_str()
                .unwrap()
                .contains("Refresh or resolve current activity")
        );
    }

    let quiet = run_case("quiet-control", &["docs/cultist-290-quiet-control.md"]);
    assert_eq!(quiet["findings"].as_array().unwrap().len(), 0);
}
