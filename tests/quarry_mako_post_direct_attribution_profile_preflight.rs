use std::fs;
use std::process::Command;

use serde_json::Value;

#[test]
fn quarry_mako_post_direct_attribution_profile_is_path_quiet() {
    let root = std::env::temp_dir().join(format!(
        "cultist-quarry-mako-attribution-profile-root-{}",
        std::process::id()
    ));
    let inventory_path = std::env::temp_dir().join(format!(
        "cultist-quarry-mako-attribution-profile-inventory-{}.json",
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
  "source": "github:bounded-performance-family-mako-2026-08-22T20:48:00Z",
  "observed_at": "2026-08-22T20:48:00Z",
  "current": {
    "id": "mako/post-direct-attribution-profile",
    "kind": "planned_work",
    "title": "Mako profile current attribution residual after direct timing",
    "url": "https://github.com/Coreys-Quarry/quarry/issues/563",
    "head_ref": "main",
    "head_sha": "4da48af927d0ca389dea8a1d6a12085e25303259",
    "updated_at": "2026-08-22T20:48:00Z",
    "draft": false,
    "activity": "preparation",
    "changed_paths": [
      ".github/workflows/research-563-post-direct-attribution-profile.yml",
      "scripts/research_563_post_direct_attribution_profile.py",
      "src/quarry/_exact_regime_attribution_receipts.py"
    ]
  },
  "active_work": [
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
    let overlaps = findings
        .iter()
        .filter(|finding| finding["kind"].as_str() == Some("preflight-inventory-path-overlap"))
        .collect::<Vec<_>>();
    assert!(
        overlaps.is_empty(),
        "attribution profile unexpectedly overlaps active work: {stdout}"
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
