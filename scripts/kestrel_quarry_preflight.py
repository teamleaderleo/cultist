#!/usr/bin/env python3
"""Disposable hosted Cultist consumer for Kestrel's frozen Quarry provider packet."""

from __future__ import annotations

import hashlib
import json
import subprocess
import tempfile
from datetime import datetime, timezone
from pathlib import Path

CANONICAL_REPOSITORY = "teamleaderleo/quarry"
QUARRY_VISIBILITY = "private"
FROZEN_QUARRY_MAIN = "1a65f8fe795f615e1f2f2587c1dd2ef341cac08a"
CULTIST_MAIN_UNDER_TEST = "97843bcdbe356b410d7e7dafd06ff64929117c41"
PROVIDER_PACKET_SOURCE = (
    "connected GitHub provider; canonical private Quarry; exhaustive open PR membership, "
    "drafts included; exact provider heads and changed paths refreshed immediately before carrier update"
)
FAILED_LIVE_PRODUCER_RUNS = [
    {"run": 32637491515, "job": 97189324252, "failure": "Cultist Actions token could not resolve historical Quarry alias"},
    {"run": 32637593037, "job": 97189565845, "failure": "anonymous hosted runner could not read private canonical Quarry"},
]

# Frozen provider facts. These are intentionally supplied from the connected GitHub provider
# rather than fetched by the Cultist-hosted runner: Quarry is private and the Cultist Actions
# credential has no cross-repository Quarry authority.
FROZEN_WORK = {
    807: {
        "head": "e2aee80bb4c85ce0966eb2e53aaf1ef07a6140a8",
        "paths": [
            ".github/workflows/hourly-btc-momentum.yml",
            "src/quarry/hourly_baseline_signals.py",
            "src/quarry/hourly_momentum_campaign.py",
            "src/quarry/hourly_research_fill.py",
            "src/quarry/hourly_retained_source.py",
            "tests/test_hourly_baseline_signals.py",
            "tests/test_hourly_momentum_campaign.py",
            "tests/test_hourly_research_fill.py",
            "tests/test_hourly_retained_source.py",
        ],
    },
    805: {
        "head": "5403d087bb92c0742a1dfeddedee35fa8321760b",
        "paths": [
            ".github/workflows/research-677-frontier-event-study-v4-null.yml",
            "research/results/frontier-tech-event-study-677-v4-null.json",
            "research/sources/frontier-tech-events-677-v4-null.json",
            "src/quarry/frontier_expectation_null.py",
            "tests/test_frontier_expectation_null.py",
            "tests/test_research_677_frontier_event_study_v4_null_live.py",
        ],
    },
    803: {
        "head": "c76425a5c045c2a8fb08c20cbbeb113fc9778b4f",
        "paths": ["src/quarry/range_volume_response_atlas.py", "tests/test_range_volume_response_atlas.py"],
    },
    800: {
        "head": "257880fd1cc8e7ed4239706734a43fb2fb302dc5",
        "paths": [
            ".github/workflows/research-550-yahoo-actions.yml",
            "research/results/yahoo-corporate-actions-descriptive-v1.json",
            "src/quarry/public_equity_actions.py",
            "tests/test_public_equity_actions.py",
            "tests/test_research_550_yahoo_corporate_actions_live.py",
            "tests/test_yahoo_corporate_actions_descriptive_result.py",
        ],
    },
    781: {
        "head": "8b5102b6577a4f57eea0cb391fcc1a546981eb5b",
        "paths": [
            "src/quarry/data/btc_quote_attempt.py",
            "src/quarry/data/coinbase_l2_stream.py",
            "tests/test_btc_quote_attempt_live_message_bound.py",
            "tests/test_coinbase_l2_stream.py",
        ],
    },
    776: {
        "head": "da7122f6a35bef83dcd6112bbebcc9ee3210efa8",
        "paths": [
            ".github/workflows/research-755-auction-demand-complete-era-v2.yml",
            "research/declarations/755-auction-demand-complete-era-v2.json",
            "research/results/755-auction-demand-complete-era-v2.json",
            "scripts/research_755_verify_complete_era_v2.py",
            "tests/test_research_755_auction_demand_complete_era_v2.py",
        ],
    },
    774: {
        "head": "3383c9472223a50e3572a95045ef98c04ae9ea73",
        "paths": ["tests/_research_725_source.py", "tests/test_research_725_expansion_onset.py"],
    },
    762: {
        "head": "77bd9231428f241c5b6d79393f2748274c8bc9ed",
        "paths": [
            "research/experiments/biotech-pdufa-prospective-validation-v1-readiness.json",
            "research/programs/biotech-pdufa-prospective-validation-v1.json",
            "tests/test_biotech_pdufa_prospective_program.py",
        ],
    },
    757: {
        "head": "aeac1c74006448f247203b04b3a47178a0229064",
        "paths": [
            ".github/workflows/alpaca-us-equity-daily-source.yml",
            "configs/research/alpaca_qqq_2026_08_17_21_v1.json",
            "configs/research/alpaca_spy_2026_08_17_21_v1.json",
            "src/quarry/_alpaca_us_equity_daily_common_v1.py",
            "src/quarry/_alpaca_us_equity_daily_normalize_v1.py",
            "src/quarry/_alpaca_us_equity_daily_provider_v1.py",
            "src/quarry/_alpaca_us_equity_daily_receipt_v1.py",
            "src/quarry/alpaca_us_equity_daily_source.py",
            "tests/test_alpaca_us_equity_daily_source.py",
            "tests/test_alpaca_us_equity_daily_transport.py",
        ],
    },
    750: {
        "head": "4beafd2fe3a7e414934e687e40dbdfba35493adf",
        "paths": [
            "research/scoreboard/research-reconvergence-late-v1.json",
            "research/scoreboard/research-reconvergence-v1.json",
            "research/scoreboard/research-reconvergence-wave3-v1.json",
            "src/quarry/research_reconvergence.py",
            "tests/test_research_reconvergence.py",
        ],
    },
    747: {
        "head": "9c7f5f1a89ffafebda31d9dbec7e9a10649eaba4",
        "paths": [
            ".github/workflows/research-731-atomic-nuclear-milestones-v1.yml",
            "research/results/atomic-hard-nuclear-milestones-731-v1.json",
            "research/sources/nuclear-hard-milestones-731-v1.json",
            "src/quarry/nuclear_milestone_event_study.py",
            "tests/test_nuclear_milestone_event_study.py",
            "tests/test_research_731_nuclear_milestones_live.py",
        ],
    },
    742: {
        "head": "0b01f8165131d40e30f234c59206594cca6fa609",
        "paths": [
            ".github/workflows/research-674-operative-trade-policy-v1.yml",
            "research/results/operative-tariff-change-repricing-v1.json",
            "research/sources/trade-policy-operative-events-v1.json",
            "research/sources/us-equity-session-calendar-2025-02-2026-05.json",
            "src/quarry/trade_policy_event_study.py",
            "tests/test_research_674_trade_policy_live.py",
            "tests/test_trade_policy_event_study.py",
        ],
    },
    737: {
        "head": "267ddd480095625712ca81135f721f97f020f24a",
        "paths": ["src/quarry/research_supergraph.py", "tests/test_research_supergraph.py"],
    },
    733: {
        "head": "fc3c9c8c5589072f06c8c48f4b34f7ddeabac3cb",
        "paths": [
            "docs/reflexive-crowding-research-676.md",
            "research/results/reflexive-crowding-676-v1-data-blocked.json",
            "src/quarry/reflexive_crowding_research.py",
            "tests/test_reflexive_crowding_research.py",
        ],
    },
    729: {
        "head": "f9616200a573f4773b408f8b922d052494ddf6d2",
        "paths": [
            ".github/workflows/research-678-aiv-liquidation-v2.yml",
            ".github/workflows/research-678-asps-staging-v2.yml",
            ".github/workflows/research-678-betr-replication-v2.yml",
            ".github/workflows/research-678-chartexchange-fallback.yml",
            ".github/workflows/research-678-gtx-liquidity-probe.yml",
            ".github/workflows/research-678-liquidation-value-ledger-v2.yml",
            ".github/workflows/research-678-path-attribution-v2.yml",
            ".github/workflows/research-678-ppsi-cash-stub-v2.yml",
            ".github/workflows/research-678-rent-v2-live.yml",
            ".github/workflows/research-678-scor-derisking-v2.yml",
            ".github/workflows/research-678-smallcap-special-situations-v2-archive-probe.yml",
            ".github/workflows/research-678-smallcap-special-situations-v2-outcomes.yml",
            ".github/workflows/research-678-smallcap-special-situations-v2-replication.yml",
            ".github/workflows/research-678-smallcap-special-situations-v2.yml",
            ".github/workflows/research-678-smallcap-special-situations.yml",
            ".github/workflows/research-678-stooq-archive-probe.yml",
            "docs/smallcap-special-situations.md",
            "research/programs/smallcap-special-situations-678-v1-source-census.json",
            "research/programs/smallcap-special-situations-678-v1.json",
            "research/programs/smallcap-special-situations-678-v2-2024-replication.json",
            "research/programs/smallcap-special-situations-678-v2-asps-staging.json",
            "research/programs/smallcap-special-situations-678-v2-betr-replication.json",
            "research/programs/smallcap-special-situations-678-v2-chartexchange-fallback.json",
            "research/programs/smallcap-special-situations-678-v2-discovery.json",
            "research/programs/smallcap-special-situations-678-v2-liquidation-aiv-replication.json",
            "research/programs/smallcap-special-situations-678-v2-liquidation-value-realization.json",
            "research/programs/smallcap-special-situations-678-v2-outcome-pilot.json",
            "research/programs/smallcap-special-situations-678-v2-path-attribution.json",
            "research/programs/smallcap-special-situations-678-v2-ppsi-cash-stub.json",
            "research/programs/smallcap-special-situations-678-v2-scor-de-risking.json",
            "research/results/smallcap-special-situations-678-v1-data-blocked.json",
            "research/results/smallcap-special-situations-678-v1-gtx-admission-blocked.json",
            "research/results/smallcap-special-situations-678-v1-gtx-liquidity.json",
            "research/results/smallcap-special-situations-678-v1-provider-recall-2025-01.json",
            "research/results/smallcap-special-situations-678-v1-provider-recall.json",
            "research/results/smallcap-special-situations-678-v1-scale-prefilter-round1.json",
            "research/results/smallcap-special-situations-678-v1-scale-prefilter-round2.json",
            "research/results/smallcap-special-situations-678-v2-2024-replication.json",
            "research/results/smallcap-special-situations-678-v2-aiv-liquidation.json",
            "research/results/smallcap-special-situations-678-v2-asps-path-attribution.json",
            "research/results/smallcap-special-situations-678-v2-asps-staged.json",
            "research/results/smallcap-special-situations-678-v2-betr-replication.json",
            "research/results/smallcap-special-situations-678-v2-chartexchange-dead-ticker-recovery.json",
            "research/results/smallcap-special-situations-678-v2-five-clock-exploratory.json",
            "research/results/smallcap-special-situations-678-v2-interest-ledger.json",
            "research/results/smallcap-special-situations-678-v2-liquidation-source-readiness.json",
            "research/results/smallcap-special-situations-678-v2-outcome-pilot.json",
            "research/results/smallcap-special-situations-678-v2-ppsi-cash-stub.json",
            "research/results/smallcap-special-situations-678-v2-rent-preoutcome.json",
            "research/results/smallcap-special-situations-678-v2-risk-removal-ledger.json",
            "research/results/smallcap-special-situations-678-v2-scor-derisking.json",
            "src/quarry/smallcap_special_situations.py",
            "src/quarry/smallcap_special_situations_aiv_v2.py",
            "src/quarry/smallcap_special_situations_archive_probe_v2.py",
            "src/quarry/smallcap_special_situations_asps_v2.py",
            "src/quarry/smallcap_special_situations_betr_replication_v2.py",
            "src/quarry/smallcap_special_situations_chartexchange_v2.py",
            "src/quarry/smallcap_special_situations_liquidation_v2.py",
            "src/quarry/smallcap_special_situations_outcomes_v2.py",
            "src/quarry/smallcap_special_situations_outcomes_v2_hardened.py",
            "src/quarry/smallcap_special_situations_path_attribution_v2.py",
            "src/quarry/smallcap_special_situations_ppsi_v2.py",
            "src/quarry/smallcap_special_situations_prefilter.py",
            "src/quarry/smallcap_special_situations_prefilter_round2.py",
            "src/quarry/smallcap_special_situations_replication_v2.py",
            "src/quarry/smallcap_special_situations_scor_derisking_v2.py",
            "src/quarry/smallcap_special_situations_sec.py",
            "src/quarry/smallcap_special_situations_sec_access.py",
            "src/quarry/smallcap_special_situations_sources.py",
            "src/quarry/smallcap_special_situations_stooq.py",
            "src/quarry/smallcap_special_situations_v2.py",
            "tests/test_research_678_rent_v2_live.py",
            "tests/test_smallcap_special_situations.py",
            "tests/test_smallcap_special_situations_aiv_v2.py",
            "tests/test_smallcap_special_situations_archive_probe_v2.py",
            "tests/test_smallcap_special_situations_asps_v2.py",
            "tests/test_smallcap_special_situations_betr_replication_v2.py",
            "tests/test_smallcap_special_situations_chartexchange_v2.py",
            "tests/test_smallcap_special_situations_liquidation_v2.py",
            "tests/test_smallcap_special_situations_liquidation_v2_tamper.py",
            "tests/test_smallcap_special_situations_outcomes_v2.py",
            "tests/test_smallcap_special_situations_outcomes_v2_hardened.py",
            "tests/test_smallcap_special_situations_path_attribution_v2.py",
            "tests/test_smallcap_special_situations_ppsi_v2.py",
            "tests/test_smallcap_special_situations_prefilter.py",
            "tests/test_smallcap_special_situations_prefilter_round2.py",
            "tests/test_smallcap_special_situations_replication_v2.py",
            "tests/test_smallcap_special_situations_scor_derisking_v2.py",
            "tests/test_smallcap_special_situations_sec.py",
            "tests/test_smallcap_special_situations_sec_access.py",
            "tests/test_smallcap_special_situations_sources.py",
            "tests/test_smallcap_special_situations_stooq.py",
            "tests/test_smallcap_special_situations_v2.py",
        ],
    },
    727: {
        "head": "625ed88ff001ebfb847dea9816e669d197747dbc",
        "paths": [
            "research/experiments/geopolitical-physical-energy-cross-asset-v2-result.json",
            "research/experiments/opec-plus-oil-shock-v1-data-blocker.json",
            "research/experiments/physical-supply-vs-risk-only-v3b-result.json",
            "research/programs/geopolitical-energy-transmission-v2-v3.json",
            "research/programs/opec-plus-oil-shock-v1.json",
            "tests/test_geopolitical_energy_transmission.py",
            "tests/test_opec_plus_oil_shock_program.py",
        ],
    },
    721: {
        "head": "86f48e69e8cec305f598fbce84b3d266e85e97b5",
        "paths": [
            ".github/workflows/research-660-options-event-volatility.yml",
            "docs/earnings-event-state-machine.md",
            "docs/options-event-volatility-660.md",
            "research/results/options-event-volatility-660-v1-data-blocked.json",
            "src/quarry/earnings_event_state.py",
            "src/quarry/options_event_volatility.py",
            "tests/test_earnings_event_state.py",
            "tests/test_options_event_volatility.py",
            "tests/test_options_event_volatility_result.py",
        ],
    },
    692: {
        "head": "000be92e92aff2bea9c0b63542b27f2f0d269844",
        "paths": ["docs/stock-selection-research.md", "src/quarry/stock_selection_research.py", "tests/test_stock_selection_research.py"],
    },
    691: {
        "head": "0efa2efc6584d23371b8bbed88a79085ec5222aa",
        "paths": [
            "research/programs/corporate-event-study-659-v1.json",
            "research/results/corporate-event-study-659-v1-data-blocked.json",
            "research/results/corporate-event-study-659-v1-sec-source-result.json",
            "src/quarry/company_event_study.py",
            "src/quarry/sec_edgar_filing.py",
            "tests/test_company_event_study.py",
            "tests/test_sec_edgar_filing.py",
        ],
    },
    686: {
        "head": "1e1cad0a510aad2fbc7aa5c7d4d09079a496b145",
        "paths": ["research/news_cycle/issue_663_inventory.json", "src/quarry/news_cycle.py", "tests/test_news_cycle.py"],
    },
}


def _compact_json(value: object) -> bytes:
    return json.dumps(value, ensure_ascii=False, separators=(",", ":")).encode("utf-8")


def _sha256_json(value: object) -> str:
    return hashlib.sha256(_compact_json(value)).hexdigest()


def _selection_identity(collection: str) -> str:
    # Exact merged/reviewed #298 exhaustive-open + include-drafts selection grammar.
    return _sha256_json(
        {
            "schema_version": 0,
            "provider_kind": "github",
            "provider_instance": "github.com",
            "collection": collection.lower(),
            "work_kind": "pull_request",
            "states": ["open"],
            "draft_policy": "include",
            "coverage": {"mode": "exhaustive"},
        }
    )


def _work_items(observed_at: str) -> list[dict[str, object]]:
    return [
        {
            "id": f"pull/{number}",
            "kind": "pull_request",
            "title": f"Frozen provider pull/{number}",
            "url": f"https://github.com/{CANONICAL_REPOSITORY}/pull/{number}",
            "head_ref": "provider-head",
            "head_sha": record["head"],
            "updated_at": observed_at,
            "draft": False,
            "activity": "confirmed_active",
            "changed_paths": record["paths"],
        }
        for number, record in sorted(FROZEN_WORK.items())
    ]


def _work_fact_identity(work: list[dict[str, object]]) -> str:
    # Exact merged/reviewed #305 work-fact grammar.
    canonical = []
    for item in work:
        paths = item["changed_paths"]
        if len(paths) != len(set(paths)):
            raise RuntimeError(f"duplicate path in {item['id']}")
        canonical.append(
            {
                "id": item["id"],
                "head_sha": str(item["head_sha"]).lower(),
                "activity": item["activity"],
                "changed_paths": sorted(paths),
            }
        )
    canonical.sort(key=lambda item: str(item["id"]))
    return _sha256_json({"schema_version": 0, "work": canonical, "coordination_edges": []})


def _snapshot_identity(selection: str, work_fact: str) -> str:
    return _sha256_json(
        {
            "schema_version": 0,
            "selection_identity": selection,
            "work_fact_identity": work_fact,
        }
    )


def main() -> None:
    observed_at = datetime.now(timezone.utc).isoformat().replace("+00:00", "Z")
    work = _work_items(observed_at)
    selection = _selection_identity(CANONICAL_REPOSITORY)
    work_fact = _work_fact_identity(work)
    snapshot_identity = f"sha256:{_snapshot_identity(selection, work_fact)}"

    inventory = {
        "schema_version": 1,
        "source": PROVIDER_PACKET_SOURCE,
        "observed_at": observed_at,
        "current": {
            "id": "kestrel-quarry-530-523-evidence",
            "kind": "issue_comment_evidence",
            "title": "Kestrel cold-entry pilot and context-acquisition evidence",
            "url": f"https://github.com/{CANONICAL_REPOSITORY}/issues/530",
            "head_ref": "main",
            "head_sha": FROZEN_QUARRY_MAIN,
            "updated_at": observed_at,
            "draft": False,
            "activity": "confirmed_active",
            "changed_paths": [],
        },
        "active_work": work,
        "provider_snapshot_identity": snapshot_identity,
        "coordination_edges": [],
    }

    with tempfile.TemporaryDirectory(prefix="kestrel-cultist-preflight-") as temporary:
        inventory_path = Path(temporary) / "inventory.json"
        inventory_path.write_bytes(json.dumps(inventory, indent=2, ensure_ascii=False).encode("utf-8"))
        completed = subprocess.run(
            [
                "cargo",
                "run",
                "--quiet",
                "--",
                "preflight",
                "--inventory",
                str(inventory_path),
                "--current-provider-snapshot",
                snapshot_identity,
                "--format",
                "json",
                ".",
            ],
            check=False,
            capture_output=True,
            text=True,
        )
        if completed.returncode != 0:
            raise RuntimeError(
                f"Cultist preflight failed ({completed.returncode}): {completed.stderr}\n{completed.stdout}"
            )
        report = json.loads(completed.stdout)

    findings = report.get("findings", [])
    direct_kinds = {
        "preflight-inventory-path-overlap",
        "preflight-inventory-path-overlap-activity-unknown",
    }
    direct_collisions = [finding for finding in findings if finding.get("kind") in direct_kinds]
    currentness_failures = [
        finding
        for finding in findings
        if finding.get("kind") == "preflight-inventory-provider-snapshot-invalid"
    ]
    if currentness_failures:
        raise RuntimeError(f"provider snapshot gate unexpectedly failed: {currentness_failures}")
    if direct_collisions:
        raise RuntimeError(f"comment-only lane unexpectedly collided: {direct_collisions}")

    receipt = {
        "callsign": "Kestrel 🦅",
        "canonical_quarry_repository": CANONICAL_REPOSITORY,
        "quarry_visibility": QUARRY_VISIBILITY,
        "quarry_main": FROZEN_QUARRY_MAIN,
        "cultist_main_under_test": CULTIST_MAIN_UNDER_TEST,
        "provider_packet_source": PROVIDER_PACKET_SOURCE,
        "provider_population": len(work),
        "provider_snapshot_identity": snapshot_identity,
        "provider_heads": {item["id"]: item["head_sha"] for item in work},
        "direct_collisions": len(direct_collisions),
        "explicit_current_lane_edges": 0,
        "cultist_findings": findings,
        "failed_live_producer_runs": FAILED_LIVE_PRODUCER_RUNS,
        "unknowns": [
            "zero path overlap does not establish semantic independence",
            "open-PR provider membership does not resolve no-PR branch activity or ownership",
            "review applicability and CI disposition are separate provider dimensions outside this inventory",
            "the externally frozen private-provider packet can become stale after connected-provider observation",
        ],
        "false_positive_notes": "none observed if receipt emitted: comment-only lane produced no collision/currentness finding",
        "false_negative_notes": "semantic conflicts and unresolved no-PR ownership remain outside exact path-overlap proof",
        "action_changed": False,
        "next_action": "refresh provider before write, then add only nonduplicate #529/#530/#523 evidence; no Quarry path write",
    }
    print("KESTREL_CULTIST_PREFLIGHT_RECEIPT=" + json.dumps(receipt, sort_keys=True, ensure_ascii=False))


if __name__ == "__main__":
    main()
