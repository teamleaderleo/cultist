use std::fs;
use std::process::Command;

use serde_json::Value;

#[test]
fn quarry_mako_diagnostic_source_reuse_ab_is_path_quiet_and_semantically_unknown() {
    let root = std::env::temp_dir().join(format!(
        "cultist-quarry-mako-source-reuse-root-{}",
        std::process::id()
    ));
    let inventory_path = std::env::temp_dir().join(format!(
        "cultist-quarry-mako-source-reuse-inventory-{}.json",
        std::process::id()
    ));
    fs::create_dir_all(&root).expect("create analysis root");
    fs::write(root.join(".gitignore"), "\n").expect("write seed file");
    for args in [
        vec!["init", "-q"],
        vec!["config", "user.email", "mako@example.invalid"],
        vec!["config", "user.name", "Mako"],
        vec!["add", ".gitignore"],
        vec!["commit", "-q", "-m", "seed"],
    ] {
        let status = Command::new("git")
            .arg("-C")
            .arg(&root)
            .args(args)
            .status()
            .expect("run git setup command");
        assert!(status.success(), "git setup command failed");
    }

    let inventory = r#"{
  "schema_version": 1,
  "source": "github:bounded-performance-family-mako-2026-08-22T20:13:00Z",
  "observed_at": "2026-08-22T20:13:00Z",
  "current": {
    "id": "mako/quarry-diagnostic-source-reuse-ab",
    "kind": "planned_profiling",
    "title": "Mako isolate public diagnostic source replay versus owned source reuse",
    "url": "https://github.com/Coreys-Quarry/quarry/issues/563",
    "head_ref": "main",
    "head_sha": "e322e729b9ed6e252da015caef30351e3cbfc72e",
    "updated_at": "2026-08-22T20:13:00Z",
    "draft": true,
    "activity": "preparation",
    "changed_paths": [
      ".github/workflows/research-563-diagnostic-source-reuse-cost.yml",
      "scripts/research_563_diagnostic_source_reuse_cost.py"
    ]
  },
  "active_work": [
    {
      "id": "pull/719",
      "kind": "pull_request",
      "title": "research: measure immutable timing materialization current",
      "url": "https://github.com/Coreys-Quarry/quarry/pull/719",
      "head_ref": "research/563-immutable-timing-materialization-current",
      "head_sha": "74e9255ab2f7c44c155fee8413fa91386e2992c5",
      "updated_at": "2026-08-22T19:28:32Z",
      "draft": true,
      "activity": "confirmed_active",
      "changed_paths": [
        ".github/workflows/research-563-immutable-timing-materialization-current.yml",
        "scripts/research_563_immutable_timing_materialization.py"
      ]
    },
    {
      "id": "pull/715",
      "kind": "pull_request",
      "title": "research: measure immutable timing materialization",
      "url": "https://github.com/Coreys-Quarry/quarry/pull/715",
      "head_ref": "research/563-immutable-timing-materialization",
      "head_sha": "284bea7c52b466d0acbfa1ef8c997cb7285a0e57",
      "updated_at": "2026-08-22T19:23:32Z",
      "draft": true,
      "activity": "confirmed_active",
      "changed_paths": [
        ".github/workflows/research-563-immutable-timing-materialization.yml",
        "scripts/research_563_immutable_timing_materialization.py"
      ]
    },
    {
      "id": "pull/737",
      "kind": "pull_request",
      "title": "fix: fail closed on unproven research fanout isolation",
      "url": "https://github.com/Coreys-Quarry/quarry/pull/737",
      "head_ref": "mako/637-fanout-isolation-repair-20260823",
      "head_sha": "267ddd480095625712ca81135f721f97f020f24a",
      "updated_at": "2026-08-22T19:54:47Z",
      "draft": false,
      "activity": "confirmed_active",
      "changed_paths": [
        "src/quarry/research_supergraph.py",
        "tests/test_research_supergraph.py"
      ]
    }
  ],
  "coordination_edges": []
}"#;
    fs::write(&inventory_path, inventory).expect("write bounded inventory");

    let output = Command::new(env!("CARGO_BIN_EXE_cargo-cultist"))
        .args([
            "preflight",
            "--inventory",
            inventory_path.to_str().expect("inventory path is utf-8"),
            "--format",
            "json",
            root.to_str().expect("root path is utf-8"),
        ])
        .output()
        .expect("run current Cultist preflight binary");
    assert!(
        output.status.success(),
        "preflight failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("preflight output is utf-8");
    let report: Value = serde_json::from_str(&stdout).expect("preflight output is json");
    let findings = report["findings"].as_array().expect("findings array");
    assert!(
        findings.is_empty(),
        "unexpected supplied-work finding: {stdout}"
    );

    let claims = report["claims"].as_array().expect("claims array");
    assert!(claims.iter().any(|claim| {
        claim["kind"].as_str() == Some("observed")
            && claim["message"]
                .as_str()
                .is_some_and(|message| message.contains("Examined 3 supplied work candidate(s)"))
    }));
    assert!(claims.iter().any(|claim| {
        claim["kind"].as_str() == Some("observed")
            && claim["message"]
                .as_str()
                .is_some_and(|message| message.contains("no direct path overlap"))
    }));
    assert!(claims.iter().any(|claim| {
        claim["kind"].as_str() == Some("unknown")
            && claim["message"]
                .as_str()
                .is_some_and(|message| message.contains("semantically independent"))
    }));

    fs::remove_file(&inventory_path).ok();
    fs::remove_dir_all(&root).ok();
}
