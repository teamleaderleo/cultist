#!/usr/bin/env python3
from __future__ import annotations

import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile

import external_github_reference_guard as guard

REPOSITORY = "teamleaderleo/cultist"
EXTERNAL_URL = "https://github.com/example/project/issues/123"
EXTERNAL_REDIRECT = "https://redirect.github.com/example/project/issues/123"
EXTERNAL_SHORTHAND = "example/project#123"
OWNED_URL = "https://github.com/teamleaderleo/stensibly/issues/123"
OWNED_SHORTHAND = "teamleaderleo/stensibly#123"


def refs(text: str) -> list[str]:
    return [
        violation.url
        for violation in guard.scan_interaction_text(
            text,
            source="fixture",
            current_repository=REPOSITORY,
        )
    ]


def test_direct_third_party_url_rejects() -> None:
    assert refs(f"See {EXTERNAL_URL}.") == [EXTERNAL_URL]


def test_redirect_third_party_url_passes() -> None:
    assert refs(f"See {EXTERNAL_REDIRECT}.") == []


def test_non_linking_third_party_wording_passes() -> None:
    assert refs("See example/project issue 123.") == []


def test_third_party_shorthand_rejects() -> None:
    assert refs(f"See {EXTERNAL_SHORTHAND}.") == [EXTERNAL_SHORTHAND]


def test_owned_references_pass() -> None:
    assert refs(f"See {OWNED_URL} and {OWNED_SHORTHAND}.") == []


def test_interaction_code_block_is_not_an_escape() -> None:
    text = f"```text\n{EXTERNAL_URL}\n```"
    assert refs(text) == [EXTERNAL_URL]


def test_interaction_marker_is_not_an_escape() -> None:
    text = f"{guard.ALLOW_MARKER}\n{EXTERNAL_URL}"
    assert refs(text) == [EXTERNAL_URL]


def test_configured_owned_owner_extends_first_party_set() -> None:
    original = os.environ.get("CULTIST_OWNED_GITHUB_OWNERS")
    os.environ["CULTIST_OWNED_GITHUB_OWNERS"] = "example"
    try:
        assert refs(EXTERNAL_URL) == []
    finally:
        if original is None:
            os.environ.pop("CULTIST_OWNED_GITHUB_OWNERS", None)
        else:
            os.environ["CULTIST_OWNED_GITHUB_OWNERS"] = original


def test_cli_stdin_preflight_rejects_before_write() -> None:
    completed = subprocess.run(
        [
            sys.executable,
            "scripts/external_github_reference_guard.py",
            "--repository",
            REPOSITORY,
            "--stdin",
        ],
        input=f"Proposed body: {EXTERNAL_URL}\n",
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )
    assert completed.returncode == 1, completed
    assert EXTERNAL_URL in completed.stdout, completed.stdout
    assert "Do not rely on post-write CI" in completed.stdout, completed.stdout


def test_cli_stdin_preflight_accepts_redirect() -> None:
    completed = subprocess.run(
        [
            sys.executable,
            "scripts/external_github_reference_guard.py",
            "--repository",
            REPOSITORY,
            "--stdin",
        ],
        input=f"Proposed body: {EXTERNAL_REDIRECT}\n",
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )
    assert completed.returncode == 0, completed


def test_kestrel_quarry_inventory_preflight_receipt() -> None:
    inventory = {
        "schema_version": 1,
        "source": "manual_provider_refresh:quarry-open-prs+declared-edges",
        "observed_at": "2026-08-22T18:37:49Z",
        "current": {
            "id": "kestrel/quarry-532-cold-entry",
            "kind": "planned_review_and_experiment",
            "title": "Kestrel cold-entry review and Quarry orientation experiment",
            "url": "quarry:issue/532",
            "head_ref": "main",
            "head_sha": "2b072b6ab6f01060e67c11b4306a3338e164700b",
            "updated_at": "2026-08-22T18:37:49Z",
            "draft": False,
            "changed_paths": [
                "src/quarry/agent_brief.py",
                "tests/test_agent_brief.py",
            ],
        },
        "active_work": [
            {
                "id": "pull/671",
                "kind": "pull_request",
                "title": "feat: retain durable exact research abstain decisions",
                "url": "quarry:pull/671",
                "head_ref": "kiln/633-durable-abstain-final",
                "head_sha": "0a95dcd52e282bbeabfbcd9662c44f472ca029f5",
                "updated_at": "2026-08-22T18:26:35Z",
                "draft": False,
                "changed_paths": [
                    "src/quarry/_exact_research_engine.py",
                    "src/quarry/_exact_research_execution.py",
                    "src/quarry/exact_research_contract.py",
                    "src/quarry/exact_research_result_artifact.py",
                    "tests/test_exact_research_abstain.py",
                    "tests/test_exact_research_result_artifact_abstain.py",
                ],
            },
            {
                "id": "pull/682",
                "kind": "pull_request",
                "title": "research: retain #656 memory read-through data blocker",
                "url": "quarry:pull/682",
                "head_ref": "research/656-mu-memory-readthrough-v1-current",
                "head_sha": "5bd7087e5719dd8123ffaf0a72affbb5cf895cc0",
                "updated_at": "2026-08-22T18:31:56Z",
                "draft": False,
                "changed_paths": [
                    "research/experiments/semiconductor-memory-readthrough-v1-data-blocker.json",
                    "research/programs/semiconductor-memory-readthrough-v1.json",
                    "tests/test_semiconductor_memory_readthrough_program.py",
                ],
            },
            {
                "id": "pull/684",
                "kind": "pull_request",
                "title": "Research carrier: #661 daily volatility compression pilot",
                "url": "quarry:pull/684",
                "head_ref": "research/661-daily-compression-pilot-r2",
                "head_sha": "519640d38f5de8341d99ab798ce138e6f172b33b",
                "updated_at": "2026-08-22T18:27:31Z",
                "draft": False,
                "changed_paths": ["tests/test_research_661_carrier.py"],
            },
            {
                "id": "pull/685",
                "kind": "pull_request",
                "title": "WIP carrier: harden #633 frozen prospective BTC momentum ledger",
                "url": "quarry:pull/685",
                "head_ref": "wip/633-prospective-hardening-current",
                "head_sha": "a6f35d47ec9487899d719627b95cc79eb5cedb6f",
                "updated_at": "2026-08-22T18:28:59Z",
                "draft": False,
                "changed_paths": [
                    "src/quarry/btc_momentum_prospective.py",
                    "src/quarry/btc_momentum_prospective_campaign.py",
                    "tests/test_btc_momentum_prospective.py",
                    "tests/test_btc_momentum_prospective_campaign.py",
                ],
            },
            {
                "id": "pull/686",
                "kind": "pull_request",
                "title": "research: add #663 deterministic news-cycle novelty baseline",
                "url": "quarry:pull/686",
                "head_ref": "codex/issue-663-news-cycle-novelty-baseline",
                "head_sha": "b2ae98130994b7a2e30ee4169f33313cb674655d",
                "updated_at": "2026-08-22T18:29:00Z",
                "draft": False,
                "changed_paths": [
                    "research/news_cycle/issue_663_inventory.json",
                    "src/quarry/news_cycle.py",
                    "tests/test_news_cycle.py",
                ],
            },
            {
                "id": "pull/689",
                "kind": "pull_request",
                "title": "research: admit frozen #645 BTC hourly source",
                "url": "quarry:pull/689",
                "head_ref": "agent/645-hourly-btc-campaign-20260823",
                "head_sha": "a91e0133ee228f826c1b4a54ba6a05ee7e9dac6e",
                "updated_at": "2026-08-22T18:32:17Z",
                "draft": False,
                "changed_paths": [
                    ".github/workflows/hourly-btc-source.yml",
                    "configs/research/hourly_btc_baselines_v1.json",
                    "src/quarry/hourly_source.py",
                    "tests/test_hourly_source.py",
                ],
            },
            {
                "id": "pull/690",
                "kind": "pull_request",
                "title": "[dogfood] Echo one-shot Cultist preflight carrier",
                "url": "quarry:pull/690",
                "head_ref": "echo/cultist-preflight-20260823",
                "head_sha": "164a34115421bd7adb6d69d55643dc3094e85733",
                "updated_at": "2026-08-22T18:31:13Z",
                "draft": True,
                "changed_paths": [".github/workflows/echo-cultist-preflight.yml"],
            },
            {
                "id": "pull/691",
                "kind": "pull_request",
                "title": "feat: add transparent corporate event study baseline",
                "url": "quarry:pull/691",
                "head_ref": "research/659-corporate-event-study-v1",
                "head_sha": "4de36d57c01a0032fc4f23c56c285ed0bd811961",
                "updated_at": "2026-08-22T18:31:21Z",
                "draft": False,
                "changed_paths": [
                    "research/programs/corporate-event-study-659-v1.json",
                    "research/results/corporate-event-study-659-v1-data-blocked.json",
                    "src/quarry/company_event_study.py",
                    "tests/test_company_event_study.py",
                ],
            },
            {
                "id": "pull/692",
                "kind": "pull_request",
                "title": "research: freeze #665 stock-selection v1 admission gate",
                "url": "quarry:pull/692",
                "head_ref": "research/665-stock-selection-v1",
                "head_sha": "a4aa9f7c5c6f49b83c084d3628f6ebe6a05af2c9",
                "updated_at": "2026-08-22T18:34:28Z",
                "draft": False,
                "changed_paths": [
                    "docs/stock-selection-research.md",
                    "src/quarry/stock_selection_research.py",
                    "tests/test_stock_selection_research.py",
                ],
            },
        ],
        "coordination_edges": [
            {
                "kind": "depends_on",
                "from": "pull/685",
                "to": "pull/671",
                "source": "provider:pull/671-prerequisite-for-633",
            }
        ],
    }

    with tempfile.TemporaryDirectory(prefix="kestrel-quarry-preflight-") as temporary:
        root = Path(temporary)
        inventory_path = root / "active-work.json"
        quarry_root = root / "quarry"
        quarry_root.mkdir()
        subprocess.run(["git", "init", "-q", str(quarry_root)], check=True)
        inventory_path.write_text(
            json.dumps(inventory, indent=2, sort_keys=True) + "\n",
            encoding="utf-8",
        )
        completed = subprocess.run(
            [
                "cargo",
                "run",
                "--quiet",
                "--",
                "preflight",
                "--inventory",
                str(inventory_path),
                "--format",
                "json",
                str(quarry_root),
            ],
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            check=False,
        )
        assert completed.returncode == 0, completed.stderr
        report = json.loads(completed.stdout)
        assert isinstance(report.get("findings"), list), report
        print("KESTREL_QUARRY_PREFLIGHT_RECEIPT=" + json.dumps(report, sort_keys=True))


def main() -> int:
    tests = [value for name, value in globals().items() if name.startswith("test_")]
    for test in sorted(tests, key=lambda value: value.__name__):
        test()
    print(f"external GitHub interaction preflight: {len(tests)} controls passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
