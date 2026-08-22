use std::fs;
use std::process::Command;

use serde_json::Value;

#[test]
fn quarry_mako_563_preflight_has_no_direct_path_collision() {
    let root =
        std::env::temp_dir().join(format!("cultist-quarry-mako-root-{}", std::process::id()));
    let inventory_path = std::env::temp_dir().join(format!(
        "cultist-quarry-mako-inventory-{}.json",
        std::process::id()
    ));
    fs::create_dir_all(&root).expect("create empty Quarry analysis root");

    let inventory = r#"{
  "schema_version": 1,
  "source": "github:manual-bounded-mako-2026-08-22T18:36:50Z",
  "observed_at": "2026-08-22T18:36:50Z",
  "current": {
    "id": "mako/quarry-563-post-merge-review",
    "kind": "planned_review",
    "title": "Mako post-merge adversarial review of Quarry #637",
    "url": "https://github.com/Coreys-Quarry/quarry/pull/637",
    "head_ref": "main",
    "head_sha": "2b072b6ab6f01060e67c11b4306a3338e164700b",
    "updated_at": "2026-08-22T18:36:50Z",
    "draft": false,
    "changed_paths": [
      "src/quarry/research_supergraph.py",
      "tests/test_research_supergraph_mako_adversarial.py"
    ]
  },
  "active_work": [
    {
      "id": "pull/671",
      "kind": "pull_request",
      "title": "feat: retain durable exact research abstain decisions",
      "url": "https://github.com/Coreys-Quarry/quarry/pull/671",
      "head_ref": "kiln/633-durable-abstain-final",
      "head_sha": "0a95dcd52e282bbeabfbcd9662c44f472ca029f5",
      "updated_at": "2026-08-22T18:32:39Z",
      "draft": false,
      "changed_paths": [
        "src/quarry/_exact_research_engine.py",
        "src/quarry/_exact_research_execution.py",
        "src/quarry/exact_research_contract.py",
        "src/quarry/exact_research_result_artifact.py",
        "tests/test_exact_research_abstain.py",
        "tests/test_exact_research_result_artifact_abstain.py"
      ]
    },
    {
      "id": "pull/682",
      "kind": "pull_request",
      "title": "research: retain #656 memory read-through data blocker",
      "url": "https://github.com/Coreys-Quarry/quarry/pull/682",
      "head_ref": "research/656-mu-memory-readthrough-v1-current",
      "head_sha": "5bd7087e5719dd8123ffaf0a72affbb5cf895cc0",
      "updated_at": "2026-08-22T18:32:56Z",
      "draft": false,
      "changed_paths": [
        "research/experiments/semiconductor-memory-readthrough-v1-data-blocker.json",
        "research/programs/semiconductor-memory-readthrough-v1.json",
        "tests/test_semiconductor_memory_readthrough_program.py"
      ]
    },
    {
      "id": "pull/684",
      "kind": "pull_request",
      "title": "Research carrier: #661 daily volatility compression pilot",
      "url": "https://github.com/Coreys-Quarry/quarry/pull/684",
      "head_ref": "research/661-daily-compression-pilot-r2",
      "head_sha": "519640d38f5de8341d99ab798ce138e6f172b33b",
      "updated_at": "2026-08-22T18:27:31Z",
      "draft": false,
      "changed_paths": ["tests/test_research_661_carrier.py"]
    },
    {
      "id": "pull/685",
      "kind": "pull_request",
      "title": "WIP carrier: harden #633 frozen prospective BTC momentum ledger",
      "url": "https://github.com/Coreys-Quarry/quarry/pull/685",
      "head_ref": "wip/633-prospective-hardening-current",
      "head_sha": "a4adef30f72b8f5ac2891c7481509dbeb8075bbe",
      "updated_at": "2026-08-22T18:34:05Z",
      "draft": false,
      "changed_paths": [
        "src/quarry/btc_momentum_prospective.py",
        "src/quarry/btc_momentum_prospective_campaign.py",
        "tests/test_btc_momentum_prospective.py",
        "tests/test_btc_momentum_prospective_campaign.py"
      ]
    },
    {
      "id": "pull/686",
      "kind": "pull_request",
      "title": "research: add #663 deterministic news-cycle novelty baseline",
      "url": "https://github.com/Coreys-Quarry/quarry/pull/686",
      "head_ref": "codex/issue-663-news-cycle-novelty-baseline",
      "head_sha": "b2ae98130994b7a2e30ee4169f33313cb674655d",
      "updated_at": "2026-08-22T18:29:00Z",
      "draft": false,
      "changed_paths": [
        "research/news_cycle/issue_663_inventory.json",
        "src/quarry/news_cycle.py",
        "tests/test_news_cycle.py"
      ]
    },
    {
      "id": "pull/689",
      "kind": "pull_request",
      "title": "research: admit frozen #645 BTC hourly source",
      "url": "https://github.com/Coreys-Quarry/quarry/pull/689",
      "head_ref": "agent/645-hourly-btc-campaign-20260823",
      "head_sha": "a91e0133ee228f826c1b4a54ba6a05ee7e9dac6e",
      "updated_at": "2026-08-22T18:32:17Z",
      "draft": false,
      "changed_paths": [
        ".github/workflows/hourly-btc-source.yml",
        "configs/research/hourly_btc_baselines_v1.json",
        "src/quarry/hourly_source.py",
        "tests/test_hourly_source.py"
      ]
    },
    {
      "id": "pull/690",
      "kind": "pull_request",
      "title": "[dogfood] Echo one-shot Cultist preflight carrier",
      "url": "https://github.com/Coreys-Quarry/quarry/pull/690",
      "head_ref": "echo/cultist-preflight-20260823",
      "head_sha": "edc0f895d14b2f2f19459187eb59935e9f5b590e",
      "updated_at": "2026-08-22T18:34:40Z",
      "draft": true,
      "changed_paths": [".github/workflows/echo-cultist-preflight.yml"]
    },
    {
      "id": "pull/691",
      "kind": "pull_request",
      "title": "feat: add transparent corporate event study baseline",
      "url": "https://github.com/Coreys-Quarry/quarry/pull/691",
      "head_ref": "research/659-corporate-event-study-v1",
      "head_sha": "6a4afd0780eb30043098aa2a885f37c0f586b742",
      "updated_at": "2026-08-22T18:36:48Z",
      "draft": false,
      "changed_paths": [
        "research/programs/corporate-event-study-659-v1.json",
        "research/results/corporate-event-study-659-v1-data-blocked.json",
        "src/quarry/company_event_study.py",
        "tests/test_company_event_study.py"
      ]
    },
    {
      "id": "pull/692",
      "kind": "pull_request",
      "title": "research: freeze #665 stock-selection v1 admission gate",
      "url": "https://github.com/Coreys-Quarry/quarry/pull/692",
      "head_ref": "research/665-stock-selection-v1",
      "head_sha": "a4aa9f7c5c6f49b83c084d3628f6ebe6a05af2c9",
      "updated_at": "2026-08-22T18:34:28Z",
      "draft": false,
      "changed_paths": [
        "docs/stock-selection-research.md",
        "src/quarry/stock_selection_research.py",
        "tests/test_stock_selection_research.py"
      ]
    }
  ],
  "coordination_edges": []
}"#;
    fs::write(&inventory_path, inventory).expect("write bounded active-work inventory");

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
    let active_collisions = findings
        .iter()
        .filter(|finding| {
            finding["kind"]
                .as_str()
                .is_some_and(|kind| kind.starts_with("preflight-inventory"))
        })
        .collect::<Vec<_>>();
    assert!(
        active_collisions.is_empty(),
        "unexpected direct active-work collision: {stdout}"
    );

    fs::remove_file(&inventory_path).ok();
    fs::remove_dir_all(&root).ok();
}
