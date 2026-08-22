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
  "source": "github:manual-bounded-mako-2026-08-22T19:36:20Z",
  "observed_at": "2026-08-22T19:36:20Z",
  "current": {
    "id": "mako/quarry-637-fanout-repair",
    "kind": "planned_repair",
    "title": "Mako repair of merged Quarry fanout isolation defect",
    "url": "https://github.com/Coreys-Quarry/quarry/pull/637",
    "head_ref": "main",
    "head_sha": "ff941a6704fa89e987adf170b2b7a30604b0438c",
    "updated_at": "2026-08-22T19:36:20Z",
    "draft": false,
    "activity": "preparation",
    "changed_paths": [
      "src/quarry/research_supergraph.py",
      "tests/test_research_supergraph.py"
    ]
  },
  "active_work": [
    {
      "id": "pull/724",
      "kind": "pull_request",
      "title": "research: profile post-parent regime attribution",
      "url": "https://github.com/Coreys-Quarry/quarry/pull/724",
      "head_ref": "research/563-post-parent-attribution-profile",
      "head_sha": "bcc4b158fc393fb4b5f81a2a2aed555d9a1f25a5",
      "updated_at": "2026-08-22T19:34:45Z",
      "draft": true,
      "activity": "confirmed_active",
      "changed_paths": [
        ".github/workflows/research-563-post-parent-attribution-profile.yml",
        "scripts/research_563_post_parent_attribution_profile.py"
      ]
    },
    {
      "id": "pull/722",
      "kind": "pull_request",
      "title": "Research carrier: #687 BTC downside-semivol entry cap",
      "url": "https://github.com/Coreys-Quarry/quarry/pull/722",
      "head_ref": "research/687-downside-semivol-entry-cap",
      "head_sha": "bb94cc703d0de003d97e29d9966fc1f97960fcb6",
      "updated_at": "2026-08-22T19:31:32Z",
      "draft": false,
      "activity": "confirmed_active",
      "changed_paths": [
        "tests/_research_687_exact_support.py",
        "tests/test_research_687_carrier.py"
      ]
    },
    {
      "id": "pull/721",
      "kind": "pull_request",
      "title": "research: add narrow IBKR options event evidence lane (#660)",
      "url": "https://github.com/Coreys-Quarry/quarry/pull/721",
      "head_ref": "research/660-options-event-volatility-v1",
      "head_sha": "55f365a4c4397db91f77862166c23ac77f3a07fb",
      "updated_at": "2026-08-22T19:32:09Z",
      "draft": false,
      "activity": "confirmed_active",
      "changed_paths": [
        ".github/workflows/research-660-options-event-volatility.yml",
        "docs/options-event-volatility-660.md",
        "research/results/options-event-volatility-660-v1-data-blocked.json",
        "src/quarry/options_event_volatility.py",
        "tests/test_options_event_volatility.py",
        "tests/test_options_event_volatility_result.py"
      ]
    },
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
      "head_sha": "bcabe7df323d9907362e32edecc3a781faf91b26",
      "updated_at": "2026-08-22T19:36:13Z",
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
      "head_sha": "fb0edce0dc78cef9b841789f8e372b846f6575d9",
      "updated_at": "2026-08-22T19:35:28Z",
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
      "updated_at": "2026-08-22T19:31:54Z",
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
      "head_sha": "f6a451ccfba1b14f3537fa9f6d62fa5060d636eb",
      "updated_at": "2026-08-22T19:36:11Z",
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
      "id": "pull/690",
      "kind": "pull_request",
      "title": "[dogfood] Echo one-shot Cultist preflight carrier",
      "url": "https://github.com/Coreys-Quarry/quarry/pull/690",
      "head_ref": "echo/cultist-preflight-20260823",
      "head_sha": "8103002a50f165c4600915209dd41439a011e664",
      "updated_at": "2026-08-22T19:34:06Z",
      "draft": true,
      "activity": "confirmed_active",
      "changed_paths": [
        ".github/workflows/echo-cultist-preflight.yml"
      ]
    },
    {
      "id": "pull/689",
      "kind": "pull_request",
      "title": "research: admit frozen #645 BTC hourly source",
      "url": "https://github.com/Coreys-Quarry/quarry/pull/689",
      "head_ref": "agent/645-hourly-btc-campaign-20260823",
      "head_sha": "b175f99aea9cec6477f0b6a69a4e76655a7b2087",
      "updated_at": "2026-08-22T19:34:23Z",
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
    assert!(
        findings.is_empty(),
        "unexpected supplied-work finding: {stdout}"
    );

    let claims = report["claims"].as_array().expect("claims array");
    assert!(claims.iter().any(|claim| {
        claim["kind"].as_str() == Some("observed")
            && claim["message"]
                .as_str()
                .is_some_and(|message| message.contains("Examined 12 supplied work candidate(s)"))
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
