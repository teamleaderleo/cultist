use std::fs;
use std::path::PathBuf;
use std::process::Command;

use serde_json::Value;

#[test]
fn quarry_mako_563_preflight_has_no_direct_path_collision() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let inventory_path = std::env::temp_dir().join(format!(
        "cultist-quarry-mako-inventory-{}.json",
        std::process::id()
    ));

    let inventory = r#"{
  "schema_version": 1,
  "source": "github:manual-bounded-mako-2026-08-22T18:40:50Z",
  "observed_at": "2026-08-22T18:40:50Z",
  "current": {
    "id": "mako/quarry-563-post-merge-review",
    "kind": "planned_review",
    "title": "Mako post-merge adversarial review of Quarry #637",
    "url": "https://github.com/Coreys-Quarry/quarry/pull/637",
    "head_ref": "main",
    "head_sha": "c570b460869933c31b2b9f81b88f688aced3eb56",
    "updated_at": "2026-08-22T18:40:50Z",
    "draft": false,
    "changed_paths": ["tests/test_research_supergraph_mako_adversarial.py"]
  },
  "active_work": [
    {
      "id": "pull/671",
      "kind": "pull_request",
      "title": "feat: retain durable exact research abstain decisions",
      "url": "https://github.com/Coreys-Quarry/quarry/pull/671",
      "head_ref": "kiln/633-durable-abstain-final",
      "head_sha": "4ef102243e6026618e410d1f781809817fe8238d",
      "updated_at": "2026-08-22T18:40:43Z",
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
      "updated_at": "2026-08-22T18:38:55Z",
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
      "head_sha": "9fbc5f81e111cb04a6c742cf2239162db412a607",
      "updated_at": "2026-08-22T18:39:31Z",
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
      "id": "pull/691",
      "kind": "pull_request",
      "title": "feat: add transparent corporate event study baseline",
      "url": "https://github.com/Coreys-Quarry/quarry/pull/691",
      "head_ref": "research/659-corporate-event-study-v1",
      "head_sha": "6a4afd0780eb30043098aa2a885f37c0f586b742",
      "updated_at": "2026-08-22T18:37:01Z",
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
      "head_sha": "7cbf7c03e4f53490c830c6702d56017bd008c73e",
      "updated_at": "2026-08-22T18:40:50Z",
      "draft": false,
      "changed_paths": [
        "docs/stock-selection-research.md",
        "src/quarry/stock_selection_research.py",
        "tests/test_stock_selection_research.py"
      ]
    },
    {
      "id": "pull/693",
      "kind": "pull_request",
      "title": "[research] #658 Yahoo defense replenishment event-study carrier",
      "url": "https://github.com/Coreys-Quarry/quarry/pull/693",
      "head_ref": "research/658-defense-replenishment-yahoo-v2",
      "head_sha": "5b508375598240ad06842cb917e08c5ee5f505a4",
      "updated_at": "2026-08-22T18:39:06Z",
      "draft": false,
      "changed_paths": ["tests/test_research_658_yahoo_carrier.py"]
    },
    {
      "id": "pull/694",
      "kind": "pull_request",
      "title": "research: measure scorecard bound-parent admission",
      "url": "https://github.com/Coreys-Quarry/quarry/pull/694",
      "head_ref": "research/563-scorecard-bound-parent-admission",
      "head_sha": "1d9ffb1796fa77b7999e31e507f425fbe700804b",
      "updated_at": "2026-08-22T18:40:48Z",
      "draft": true,
      "changed_paths": [
        ".github/workflows/research-563-scorecard-bound-parent-admission.yml",
        "scripts/research_563_scorecard_bound_parent_admission.py"
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
    println!("Mako Cultist preflight report: {stdout}");
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
}
