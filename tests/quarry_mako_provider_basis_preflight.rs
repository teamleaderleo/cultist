use std::fs;
use std::process::Command;

use serde_json::Value;

#[test]
fn quarry_mako_provider_basis_is_path_quiet() {
    let root = std::env::temp_dir().join(format!(
        "cultist-quarry-mako-provider-basis-root-{}",
        std::process::id()
    ));
    let inventory_path = std::env::temp_dir().join(format!(
        "cultist-quarry-mako-provider-basis-inventory-{}.json",
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
  "source": "github:mako-provider-basis-preflight-2026-08-23",
  "observed_at": "2026-08-23T19:00:00Z",
  "current": {
    "id": "mako/provider-basis-ab",
    "kind": "planned_work",
    "title": "Mako measure operation-owned provider basis",
    "url": "https://github.com/teamleaderleo/quarry/issues/563",
    "head_ref": "main",
    "head_sha": "f2a4e65441e6e8886373f6d1fddaf915c22cc175",
    "updated_at": "2026-08-23T19:00:00Z",
    "draft": false,
    "activity": "preparation",
    "changed_paths": [
      ".github/workflows/research-563-provider-basis-ab.yml",
      "scripts/research_563_provider_basis_ab.py"
    ]
  },
  "active_work": [
    {
      "id": "pull/862",
      "kind": "pull_request",
      "title": "research: add frozen #633 daily source-attempt core",
      "url": "https://github.com/teamleaderleo/quarry/pull/862",
      "head_ref": "research/633-daily-source-attempt-core",
      "head_sha": "81f5c3c0de44567753c4df2d4a36a75423322ae1",
      "updated_at": "2026-08-23T19:00:00Z",
      "draft": false,
      "activity": "confirmed_active",
      "changed_paths": [
        "src/quarry/data/btc_prospective_source_attempt.py",
        "tests/test_btc_prospective_source_attempt.py"
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
    let overlaps = findings
        .iter()
        .filter(|finding| finding["kind"].as_str() == Some("preflight-inventory-path-overlap"))
        .collect::<Vec<_>>();
    assert!(
        overlaps.is_empty(),
        "provider-basis experiment unexpectedly overlaps active work: {stdout}"
    );

    let claims = report["claims"].as_array().expect("claims array");
    assert!(claims.iter().any(|claim| {
        claim["kind"].as_str() == Some("unknown")
            && claim["message"]
                .as_str()
                .is_some_and(|message| message.contains("semantically independent"))
    }));

    fs::remove_file(&inventory_path).ok();
    fs::remove_dir_all(&root).ok();
}
