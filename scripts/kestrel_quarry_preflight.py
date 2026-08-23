#!/usr/bin/env python3
"""Disposable hosted dogfood carrier for Kestrel's Quarry #530/#523 evidence lane."""

from __future__ import annotations

import hashlib
import json
import subprocess
import tempfile
import urllib.request
from datetime import datetime, timezone
from pathlib import Path

API = "https://api.github.com"
CANONICAL_REPOSITORY = "teamleaderleo/quarry"
PREHOST_QUARRY_MAIN = "1a65f8fe795f615e1f2f2587c1dd2ef341cac08a"
PREHOST_HEADS = {
    807: "e2aee80bb4c85ce0966eb2e53aaf1ef07a6140a8",
    805: "5403d087bb92c0742a1dfeddedee35fa8321760b",
    803: "c76425a5c045c2a8fb08c20cbbeb113fc9778b4f",
    800: "257880fd1cc8e7ed4239706734a43fb2fb302dc5",
    781: "8b5102b6577a4f57eea0cb391fcc1a546981eb5b",
    776: "da7122f6a35bef83dcd6112bbebcc9ee3210efa8",
    774: "3383c9472223a50e3572a95045ef98c04ae9ea73",
    762: "77bd9231428f241c5b6d79393f2748274c8bc9ed",
    757: "aeac1c74006448f247203b04b3a47178a0229064",
    750: "4beafd2fe3a7e414934e687e40dbdfba35493adf",
    747: "9c7f5f1a89ffafebda31d9dbec7e9a10649eaba4",
    742: "0b01f8165131d40e30f234c59206594cca6fa609",
    737: "267ddd480095625712ca81135f721f97f020f24a",
    733: "fc3c9c8c5589072f06c8c48f4b34f7ddeabac3cb",
    729: "f9616200a573f4773b408f8b922d052494ddf6d2",
    727: "625ed88ff001ebfb847dea9816e669d197747dbc",
    721: "86f48e69e8cec305f598fbce84b3d266e85e97b5",
    692: "000be92e92aff2bea9c0b63542b27f2f0d269844",
    691: "0efa2efc6584d23371b8bbed88a79085ec5222aa",
    686: "1e1cad0a510aad2fbc7aa5c7d4d09079a496b145",
}


def _headers() -> dict[str, str]:
    # Quarry is public. Deliberately avoid Cultist's repository-scoped GITHUB_TOKEN here:
    # the first hosted carrier proved that token cannot resolve the historical Quarry alias.
    return {
        "Accept": "application/vnd.github+json",
        "X-GitHub-Api-Version": "2022-11-28",
        "User-Agent": "cultist-kestrel-quarry-preflight",
    }


def _request(url: str) -> tuple[object, str | None]:
    request = urllib.request.Request(url, headers=_headers())
    with urllib.request.urlopen(request, timeout=30) as response:
        payload = json.load(response)
        return payload, response.headers.get("Link")


def _get(path: str) -> object:
    payload, _ = _request(f"{API}{path}")
    return payload


def _paged(path: str) -> list[dict[str, object]]:
    separator = "&" if "?" in path else "?"
    url = f"{API}{path}{separator}per_page=100"
    rows: list[dict[str, object]] = []
    while url:
        payload, link = _request(url)
        if not isinstance(payload, list):
            raise RuntimeError(f"expected list payload for {url}")
        rows.extend(payload)
        next_url = None
        if link:
            for part in link.split(","):
                section, *params = [piece.strip() for piece in part.split(";")]
                if 'rel="next"' in params:
                    next_url = section.removeprefix("<").removesuffix(">")
                    break
        url = next_url
    return rows


def _compact_json(value: object) -> bytes:
    return json.dumps(value, ensure_ascii=False, separators=(",", ":")).encode("utf-8")


def _sha256_json(value: object) -> str:
    return hashlib.sha256(_compact_json(value)).hexdigest()


def _selection_identity(collection: str) -> str:
    # Exact merged/reviewed #298 exhaustive-open + include-drafts selection grammar.
    document = {
        "schema_version": 0,
        "provider_kind": "github",
        "provider_instance": "github.com",
        "collection": collection.lower(),
        "work_kind": "pull_request",
        "states": ["open"],
        "draft_policy": "include",
        "coverage": {"mode": "exhaustive"},
    }
    return _sha256_json(document)


def _work_fact_identity(work: list[dict[str, object]]) -> str:
    # Exact merged/reviewed #305 work-fact grammar: canonical PR id/head/activity/path set.
    canonical = []
    for item in work:
        paths = item["changed_paths"]
        if not isinstance(paths, list):
            raise RuntimeError("changed_paths must be a list")
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
    return _sha256_json(
        {
            "schema_version": 0,
            "work": canonical,
            "coordination_edges": [],
        }
    )


def _snapshot_identity(selection: str, work_fact: str) -> str:
    # Exact merged/reviewed provider_snapshot_composition.rs grammar.
    return _sha256_json(
        {
            "schema_version": 0,
            "selection_identity": selection,
            "work_fact_identity": work_fact,
        }
    )


def _provider_snapshot(repository: str) -> tuple[str, list[dict[str, object]], str]:
    branch = _get(f"/repos/{repository}/branches/main")
    if not isinstance(branch, dict):
        raise RuntimeError("branch response was not an object")
    main_sha = str(branch["commit"]["sha"])

    pulls = _paged(f"/repos/{repository}/pulls?state=open")
    work: list[dict[str, object]] = []
    for pull in pulls:
        number = int(pull["number"])
        files = _paged(f"/repos/{repository}/pulls/{number}/files")
        changed_paths = [str(row["filename"]) for row in files]
        if not changed_paths:
            raise RuntimeError(f"open pull/{number} has no changed paths")
        work.append(
            {
                "id": f"pull/{number}",
                "kind": "pull_request",
                "activity": "confirmed_active",
                "title": str(pull["title"]),
                "url": str(pull["html_url"]),
                "head_ref": str(pull["head"]["ref"]),
                "head_sha": str(pull["head"]["sha"]),
                "updated_at": str(pull["updated_at"]),
                "draft": bool(pull["draft"]),
                "changed_paths": changed_paths,
            }
        )

    selection = _selection_identity(repository)
    work_fact = _work_fact_identity(work)
    snapshot = _snapshot_identity(selection, work_fact)
    return main_sha, work, f"sha256:{snapshot}"


def _prehost_delta(main_sha: str, work: list[dict[str, object]]) -> dict[str, object]:
    actual = {int(str(item["id"]).split("/", 1)[1]): str(item["head_sha"]) for item in work}
    expected_numbers = set(PREHOST_HEADS)
    actual_numbers = set(actual)
    moved = {
        str(number): {"before": PREHOST_HEADS[number], "hosted": actual[number]}
        for number in sorted(expected_numbers & actual_numbers)
        if PREHOST_HEADS[number] != actual[number]
    }
    return {
        "quarry_main_moved": main_sha != PREHOST_QUARRY_MAIN,
        "quarry_main_before": PREHOST_QUARRY_MAIN,
        "quarry_main_hosted": main_sha,
        "added_prs": sorted(actual_numbers - expected_numbers),
        "removed_prs": sorted(expected_numbers - actual_numbers),
        "moved_heads": moved,
    }


def main() -> None:
    canonical = CANONICAL_REPOSITORY
    observed_at = datetime.now(timezone.utc).isoformat().replace("+00:00", "Z")
    quarry_main, work, snapshot_identity = _provider_snapshot(canonical)
    delta = _prehost_delta(quarry_main, work)

    inventory = {
        "schema_version": 1,
        "source": (
            "Kestrel hosted public-GitHub snapshot of canonical Quarry; exhaustive open pull requests, "
            "drafts included; #298/#305/provider_snapshot_composition grammar"
        ),
        "observed_at": observed_at,
        "current": {
            "id": "kestrel-quarry-530-523-evidence",
            "kind": "issue_comment_evidence",
            "activity": "confirmed_active",
            "title": "Kestrel cold-entry pilot and context-acquisition evidence",
            "url": f"https://github.com/{canonical}/issues/530",
            "head_ref": "main",
            "head_sha": quarry_main,
            "updated_at": observed_at,
            "draft": False,
            "changed_paths": [],
        },
        "active_work": work,
        "provider_snapshot_identity": snapshot_identity,
        "coordination_edges": [],
    }

    with tempfile.TemporaryDirectory(prefix="kestrel-cultist-preflight-") as temporary:
        inventory_path = Path(temporary) / "inventory.json"
        inventory_path.write_bytes(json.dumps(inventory, indent=2, ensure_ascii=False).encode("utf-8"))
        command = [
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
        ]
        completed = subprocess.run(command, check=False, capture_output=True, text=True)
        if completed.returncode != 0:
            raise RuntimeError(
                f"Cultist preflight failed ({completed.returncode}): {completed.stderr}\n{completed.stdout}"
            )
        report = json.loads(completed.stdout)

    findings = report.get("findings", [])
    direct_collision_kinds = {
        "preflight-inventory-path-overlap",
        "preflight-inventory-path-overlap-activity-unknown",
    }
    direct_collisions = [finding for finding in findings if finding.get("kind") in direct_collision_kinds]
    provider_gate_failures = [
        finding
        for finding in findings
        if finding.get("kind") == "preflight-inventory-provider-snapshot-invalid"
    ]
    if provider_gate_failures:
        raise RuntimeError(f"provider snapshot gate unexpectedly failed: {provider_gate_failures}")
    if direct_collisions:
        raise RuntimeError(f"comment-only lane unexpectedly collided: {direct_collisions}")

    receipt = {
        "callsign": "Kestrel 🦅",
        "canonical_quarry_repository": canonical,
        "canonical_authority_source": "connected GitHub provider refresh before hosted carrier",
        "hosted_provider_access": "unauthenticated public canonical GitHub API; repository-scoped Cultist token intentionally unused",
        "quarry_main": quarry_main,
        "cultist_main_under_test": "97843bcdbe356b410d7e7dafd06ff64929117c41",
        "observed_at": observed_at,
        "provider_population": len(work),
        "provider_snapshot_identity": snapshot_identity,
        "provider_heads": {item["id"]: item["head_sha"] for item in sorted(work, key=lambda row: str(row["id"]))},
        "prehost_delta": delta,
        "direct_collisions": len(direct_collisions),
        "explicit_current_lane_edges": 0,
        "cultist_findings": findings,
        "unknowns": [
            "zero path overlap does not establish semantic independence",
            "open-PR provider membership does not resolve no-PR branch activity or ownership",
            "review applicability and CI disposition are separate provider dimensions outside this inventory",
            "provider state can move after observed_at",
        ],
        "false_positive_notes": "none observed if receipt emitted: comment-only lane produced no path or provider-currentness finding",
        "false_negative_notes": "semantic conflicts and unresolved no-PR ownership remain outside path-overlap proof",
        "context_changed_since_prehost_refresh": any(
            [
                delta["quarry_main_moved"],
                bool(delta["added_prs"]),
                bool(delta["removed_prs"]),
                bool(delta["moved_heads"]),
            ]
        ),
        "action_changed": False,
        "next_action": "write only nonduplicate #530/#523 evidence; no Quarry repository path write",
    }
    print("KESTREL_CULTIST_PREFLIGHT_RECEIPT=" + json.dumps(receipt, sort_keys=True, ensure_ascii=False))


if __name__ == "__main__":
    main()
