use std::fs;
use std::process::Command;

use serde_json::Value;

#[test]
fn quarry_mako_fanout_repair_preflight_is_path_quiet_and_semantically_unknown() {
    let root = std::env::temp_dir().join(format!(
        "cultist-quarry-mako-fanout-root-{}",
        std::process::id()
    ));
    let inventory_path = std::env::temp_dir().join(format!(
        "cultist-quarry-mako-fanout-inventory-{}.json",
        std::process::id()
    ));
    fs::create_dir_all(&root).expect("create empty Quarry analysis root");

    let inventory = r#"{
  "schema_version": 1,
  "source": "github:manual-bounded-mako-2026-08-22T19:29:20Z",
  "observed_at": "2026-08-22T19:29:20Z",
  "current": {
    "id": "mako/quarry-637-fanout-repair",
    "kind": "planned_repair",
    "title": "Mako repair of merged Quarry fanout isolation defect",
    "url": "https://github.com/Coreys-Quarry/quarry/pull/637",
    "head_ref": "main",
    "head_sha": "ff941a6704fa89e987adf170b2b7a30604b0438c",
    "updated_at": "2026-08-22T19:29:20Z",
    "draft": false,
    "activity": "preparation",
    "changed_paths": [
      "src/quarry/research_supergraph.py",
      "tests/test_research_supergraph.py"
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
      "id": "pull/718",
      "kind": "pull_request",
      "title": "feat: persist exact admission and risk execution on one coupled head",
      "url": "https://github.com/Coreys-Quarry/quarry/pull/718",
      "head_ref": "sable/648-single-head-cutover-risk-execution-20260823",
      "head_sha": "e8a0f5d6e58d48612e5e80e6e963921cf915920d",
      "updated_at": "2026-08-22T19:29:11Z",
      "draft": false,
      "activity": "confirmed_active",
      "changed_paths": [
        "src/quarry/exact_coupled_authority_v2.py",
        "tests/test_exact_coupled_authority_v2.py"
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
      "id": "pull/709",
      "kind": "pull_request",
      "title": "research: run #656 SEC + Yahoo memory read-through v2",
      "url": "https://github.com/Coreys-Quarry/quarry/pull/709",
      "head_ref": "research/656-yahoo-sec-v2",
      "head_sha": "151b169fd4104a6086483a34e6d8fe3fe4adeb0f",
      "updated_at": "2026-08-22T19:28:18Z",
      "draft": false,
      "activity": "confirmed_active",
      "changed_paths": [
        ".github/workflows/research-656-yahoo-sec-v2.yml",
        "research/results/semiconductor-memory-readthrough-yahoo-sec-v2-reject.json",
        "research/results/semiconductor-memory-readthrough-yahoo-sec-v2-replay.json",
        "research/sources/semiconductor-memory-sec-events-v2.json",
        "src/quarry/public_equity_history.py",
        "tests/test_public_equity_history.py",
        "tests/test_research_656_yahoo_sec_live.py",
        "tests/test_semiconductor_memory_readthrough_v2_result.py"
      ]
    },
    {
      "id": "pull/692",
      "kind": "pull_request",
      "title": "research: freeze #665 stock-selection v1 admission gate",
      "url": "https://github.com/Coreys-Quarry/quarry/pull/692",
      "head_ref": "research/665-stock-selection-v1",
      "head_sha": "000be92e92aff2bea9c0b63542b27f2f0d269844",
      "updated_at": "2026-08-22T18:48:05Z",
      "draft": false,
      "activity": "confirmed_active",
      "changed_paths": [
        "docs/stock-selection-research.md",
        "src/quarry/stock_selection_research.py",
        "tests/test_stock_selection_research.py"
      ]
    },
    {
      "id": "pull/691",
      "kind": "pull_request",
      "title": "feat: add corporate event study and SEC filing baseline",
      "url": "https://github.com/Coreys-Quarry/quarry/pull/691",
      "head_ref": "research/659-corporate-event-study-v1",
      "head_sha": "30946d26b4185b8564706f073637cf05ffb7aa3b",
      "updated_at": "2026-08-22T19:08:06Z",
      "draft": false,
      "activity": "confirmed_active",
      "changed_paths": [
        "research/programs/corporate-event-study-659-v1.json",
        "research/results/corporate-event-study-659-v1-data-blocked.json",
        "research/results/corporate-event-study-659-v1-sec-source-result.json",
        "src/quarry/company_event_study.py",
        "src/quarry/sec_edgar_filing.py",
        "tests/test_company_event_study.py",
        "tests/test_sec_edgar_filing.py"
      ]
    },
    {
      "id": "pull/689",
      "kind": "pull_request",
      "title": "research: admit frozen #645 BTC hourly source",
      "url": "https://github.com/Coreys-Quarry/quarry/pull/689",
      "head_ref": "agent/645-hourly-btc-campaign-20260823",
      "head_sha": "a91e0133ee228f826c1b4a54ba6a05ee7e9dac6e",
      "updated_at": "2026-08-22T18:49:56Z",
      "draft": false,
      "activity": "confirmed_active",
      "changed_paths": [
        ".github/workflows/hourly-btc-source.yml",
        "configs/research/hourly_btc_baselines_v1.json",
        "src/quarry/hourly_source.py",
        "tests/test_hourly_source.py"
      ]
    },
    {
      "id": "pull/686",
      "kind": "pull_request",
      "title": "research: add #663 deterministic news-cycle novelty baseline",
      "url": "https://github.com/Coreys-Quarry/quarry/pull/686",
      "head_ref": "codex/issue-663-news-cycle-novelty-baseline",
      "head_sha": "9fbc5f81e111cb04a6c742cf2239162db412a607",
      "updated_at": "2026-08-22T18:42:59Z",
      "draft": false,
      "activity": "confirmed_active",
      "changed_paths": [
        "research/news_cycle/issue_663_inventory.json",
        "src/quarry/news_cycle.py",
        "tests/test_news_cycle.py"
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
    assert!(findings.is_empty(), "unexpected supplied-work finding: {stdout}");

    let claims = report["claims"].as_array().expect("claims array");
    assert!(claims.iter().any(|claim| {
        claim["kind"].as_str() == Some("observed")
            && claim["message"]
                .as_str()
                .is_some_and(|message| message.contains("Examined 8 supplied work candidate(s)"))
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
