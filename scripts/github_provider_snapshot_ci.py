#!/usr/bin/env python3
"""Thin CI bridge from GitHub provider facts to the canonical Rust snapshot producer."""

from __future__ import annotations

import json
import subprocess
from typing import Callable

PRODUCER_SCHEMA_VERSION = 1


def _work_number(work_id: object) -> int:
    value = str(work_id)
    if not value.startswith("#"):
        raise RuntimeError(f"GitHub work id `{value}` must use `#<number>` form")
    digits = value[1:]
    if not digits or digits.startswith("0") or not digits.isascii() or not digits.isdigit():
        raise RuntimeError(f"GitHub work id `{value}` must use a positive canonical decimal number")
    return int(digits)


def _snapshot_input(
    repo: str,
    inventory: dict[str, object],
    coordination_edges: list[dict[str, object]],
) -> dict[str, object]:
    active_work = inventory.get("active_work")
    if not isinstance(active_work, list):
        raise RuntimeError("active-work inventory must contain a work list")

    work: list[dict[str, object]] = []
    for item in active_work:
        if not isinstance(item, dict):
            raise RuntimeError("active-work item must be an object")
        paths = item.get("changed_paths")
        if not isinstance(paths, list):
            raise RuntimeError("active-work item changed_paths must be a list")
        work.append(
            {
                "number": _work_number(item.get("id")),
                "head_sha": str(item.get("head_sha", "")),
                "activity": str(item.get("activity", "confirmed_active")),
                "changed_paths": [str(path) for path in paths],
            }
        )

    edges: list[dict[str, object]] = []
    for edge in coordination_edges:
        if not isinstance(edge, dict):
            raise RuntimeError("coordination edge must be an object")
        edges.append(
            {
                "kind": str(edge.get("kind", "")),
                "from_number": _work_number(edge.get("from")),
                "to_number": _work_number(edge.get("to")),
                "source": str(edge.get("source", "")),
            }
        )

    return {
        "provider_instance": "github.com",
        "collection": repo,
        "work": work,
        "coordination_edges": edges,
    }


def derive_provider_snapshot_identities(
    repo: str,
    snapshots: list[tuple[dict[str, object], list[dict[str, object]]]],
    runner: Callable[..., str] = subprocess.check_output,
) -> list[str]:
    request = {
        "schema_version": PRODUCER_SCHEMA_VERSION,
        "snapshots": [
            _snapshot_input(repo, inventory, edges)
            for inventory, edges in snapshots
        ],
    }
    output = runner(
        ["cargo", "run", "--quiet", "--example", "github_provider_snapshot"],
        input=json.dumps(request),
        text=True,
    )
    parsed = json.loads(output)
    if not isinstance(parsed, dict) or parsed.get("schema_version") != 1:
        raise RuntimeError("GitHub provider snapshot producer returned an unsupported envelope")
    receipts = parsed.get("snapshots")
    if not isinstance(receipts, list) or len(receipts) != len(snapshots):
        raise RuntimeError("GitHub provider snapshot producer returned the wrong receipt count")

    identities: list[str] = []
    for receipt in receipts:
        if not isinstance(receipt, dict):
            raise RuntimeError("GitHub provider snapshot receipt must be an object")
        identity = receipt.get("provider_snapshot_identity")
        if not isinstance(identity, str) or not identity.startswith("sha256:"):
            raise RuntimeError("GitHub provider snapshot producer returned a malformed identity")
        identities.append(identity)
    return identities
