#!/usr/bin/env python3
"""Run the active-work advisory with both provider-current applicability axes."""

from __future__ import annotations

import datetime
import json
import os
from pathlib import Path
import subprocess
import tempfile
import time

import active_work_heads_up_ci as base
from github_provider_snapshot_ci import derive_provider_snapshot_identities


def build_single_read_inventory_and_metadata(
    repo: str,
    current_number: int,
) -> tuple[dict[str, object], dict[str, object]]:
    """Build one provider population from exactly one GraphQL response."""
    owner, name = repo.split("/", 1)
    result = base.graphql(
        base.PR_PAGE_QUERY,
        {"owner": owner, "name": name, "after": None},
    )
    data = result.get("data")
    repository = data.get("repository") if isinstance(data, dict) else None
    pull_requests = (
        repository.get("pullRequests") if isinstance(repository, dict) else None
    )
    if not isinstance(pull_requests, dict):
        raise RuntimeError("could not retrieve open pull requests")

    nodes = pull_requests.get("nodes")
    if not isinstance(nodes, list):
        raise RuntimeError("open pull-request connection is missing nodes")
    has_next, _cursor = base.page_info(pull_requests)
    if has_next:
        raise RuntimeError(
            "provider population requires pull-request pagination; exact snapshot unavailable"
        )

    work: list[dict[str, object]] = []
    metadata_work: list[dict[str, object]] = []
    for node in nodes:
        if not isinstance(node, dict):
            continue
        number = int(node["number"])
        files = node.get("files")
        if not isinstance(files, dict):
            raise RuntimeError(f"PR #{number} has no readable files connection")
        files_has_next, _files_cursor = base.page_info(files)
        if files_has_next:
            raise RuntimeError(
                f"provider population requires file pagination for PR #{number}; "
                "exact snapshot unavailable"
            )
        work.append(base.work_item(owner, name, node))
        metadata_work.append(base.metadata_item(node))

    current = next((item for item in work if item["id"] == f"#{current_number}"), None)
    if current is None:
        raise RuntimeError(f"current PR #{current_number} was absent from open inventory")

    observed_at = (
        datetime.datetime.now(datetime.timezone.utc)
        .isoformat()
        .replace("+00:00", "Z")
    )
    inventory = {
        "schema_version": 1,
        "source": "github_pull_requests_graphql_single_read",
        "observed_at": observed_at,
        "current": current,
        "active_work": work,
    }
    metadata = {
        "schema_version": 1,
        "source": "github_pull_requests_graphql_single_read",
        "work": metadata_work,
    }
    return inventory, metadata


def coordination_for_snapshot(
    inventory: dict[str, object],
    metadata: dict[str, object],
    *,
    retain_note: bool,
) -> tuple[list[dict[str, object]], str | None, float]:
    if not base.potential_coordination_clause(metadata):
        return [], None, 0.0

    started = time.monotonic()
    try:
        edges, extraction_report = base.extract_coordination_edges(metadata)
        relevant_edges = [
            edge for edge in edges if base.edge_involves_current(inventory, edge)
        ]
        if not retain_note:
            return relevant_edges, None, time.monotonic() - started

        notes: list[str] = []
        current_unresolved = base.unresolved_endpoint_receipts_involving_current(
            inventory, extraction_report
        )
        unresolved_note = base.unresolved_endpoint_note(current_unresolved)
        if unresolved_note:
            notes.append(unresolved_note)
        unknowns = extraction_report.get("unknowns")
        if relevant_edges and isinstance(unknowns, list) and unknowns:
            notes.append(str(unknowns[0]))
        return relevant_edges, " ".join(notes) or None, time.monotonic() - started
    except (subprocess.CalledProcessError, json.JSONDecodeError, RuntimeError) as error:
        if retain_note:
            note = (
                "Reviewed coordination metadata could not be fully analyzed; "
                f"direct path evidence remains usable. Detail: {error}"
            )
            print(f"coordination metadata unavailable: {error}")
            return [], note, time.monotonic() - started
        raise


def read_current_provider_snapshot(
    repo: str,
    current_number: int,
) -> tuple[dict[str, object], list[dict[str, object]]] | None:
    try:
        inventory, metadata = build_single_read_inventory_and_metadata(
            repo, current_number
        )
        edges, _note, _seconds = coordination_for_snapshot(
            inventory, metadata, retain_note=False
        )
        return inventory, edges
    except (
        subprocess.CalledProcessError,
        json.JSONDecodeError,
        KeyError,
        RuntimeError,
        TypeError,
        ValueError,
    ) as error:
        print(f"provider-current population unavailable: {error}")
        return None


def can_skip_product(
    *,
    direct_overlap: bool,
    relevant_edges: list[dict[str, object]],
    exact_current_work: bool,
    required_provider_snapshot: str,
    current_provider_snapshot: str | None,
) -> bool:
    return (
        not direct_overlap
        and not relevant_edges
        and exact_current_work
        and current_provider_snapshot == required_provider_snapshot
    )


def run_product_preflight(
    inventory: dict[str, object],
    required_repository: str,
    provider_current: dict[str, object],
    current_provider_snapshot: str | None,
) -> dict[str, object]:
    with tempfile.TemporaryDirectory(prefix="cultist-active-work-") as temporary:
        inventory_path = Path(temporary, "inventory.json")
        inventory_path.write_text(json.dumps(inventory, indent=2) + "\n")
        environment = base.provider_current_environment(
            dict(os.environ), required_repository, provider_current
        )
        command = [
            "cargo",
            "run",
            "--quiet",
            "--",
            "preflight",
            "--inventory",
            str(inventory_path),
        ]
        if current_provider_snapshot is not None:
            command.extend(["--current-provider-snapshot", current_provider_snapshot])
        command.extend(["--format", "json", "."])
        output = subprocess.check_output(command, text=True, env=environment)
    report = json.loads(output)
    if not isinstance(report, dict):
        raise RuntimeError("product preflight did not return a JSON object")
    return report


def main() -> None:
    repo = os.environ["GITHUB_REPOSITORY"]
    current_number = int(os.environ["CURRENT_PR"])

    inventory_started = time.monotonic()
    inventory, metadata = build_single_read_inventory_and_metadata(repo, current_number)
    inventory_seconds = time.monotonic() - inventory_started

    relevant_edges, metadata_note, coordination_seconds = coordination_for_snapshot(
        inventory, metadata, retain_note=True
    )
    if relevant_edges:
        inventory["coordination_edges"] = relevant_edges

    population_started = time.monotonic()
    current_population = read_current_provider_snapshot(repo, current_number)
    population_seconds = time.monotonic() - population_started

    snapshots = [(inventory, relevant_edges)]
    if current_population is not None:
        snapshots.append(current_population)
    identities = derive_provider_snapshot_identities(repo, snapshots)
    required_provider_snapshot = identities[0]
    current_provider_snapshot = identities[1] if current_population is not None else None
    inventory["provider_snapshot_identity"] = required_provider_snapshot

    # Preserve #333's independent terminal work-head read after the broader population read.
    provider_current = base.read_current_provider_work(repo, current_number)

    current = inventory.get("current")
    active_work = inventory.get("active_work")
    if not isinstance(current, dict) or not isinstance(active_work, list):
        raise RuntimeError("active-work inventory has unexpected work fields")

    current_head = provider_current.get("head_sha")
    current_head_label = str(current_head) if current_head else "<unavailable>"
    current_snapshot_label = current_provider_snapshot or "<unavailable>"
    print(
        f"inventory: {len(active_work)} open PR(s), current #{current_number}, "
        f"{len(current['changed_paths'])} current path(s)"
    )
    print(
        "provider-current work: "
        f"{provider_current['repository']} {provider_current['work_id']} @ {current_head_label}"
    )
    print(f"provider snapshot required: {required_provider_snapshot}")
    print(f"provider snapshot current:  {current_snapshot_label}")
    if relevant_edges:
        print(
            "coordination metadata: "
            f"{len(relevant_edges)} resolved edge(s) involving current work"
        )

    direct_overlap = base.potential_direct_overlap(inventory)
    exact_current_work = base.provider_current_matches_inventory(
        inventory, repo, provider_current
    )

    summary_path = os.environ.get("GITHUB_STEP_SUMMARY")
    if can_skip_product(
        direct_overlap=direct_overlap,
        relevant_edges=relevant_edges,
        exact_current_work=exact_current_work,
        required_provider_snapshot=required_provider_snapshot,
        current_provider_snapshot=current_provider_snapshot,
    ):
        print(base.quiet_status_line(metadata_note))
        print(
            f"timing: inventory {inventory_seconds:.2f}s; "
            f"coordination {coordination_seconds:.2f}s; "
            f"population {population_seconds:.2f}s; product 0.00s"
        )
        if summary_path:
            with Path(summary_path).open("a") as summary:
                summary.write(base.quiet_summary(inventory, metadata_note))
        return

    product_started = time.monotonic()
    report = run_product_preflight(
        inventory,
        repo,
        provider_current,
        current_provider_snapshot,
    )
    product_seconds = time.monotonic() - product_started
    findings = report.get("findings")
    finding_count = len(findings) if isinstance(findings, list) else 0
    print(f"HEADS UP: {finding_count} product preflight finding(s)")
    if isinstance(findings, list):
        for finding in findings:
            if isinstance(finding, dict):
                print(
                    f"  {finding.get('kind', 'finding')}: "
                    f"{finding.get('title', '')}"
                )
    print(
        f"timing: inventory {inventory_seconds:.2f}s; "
        f"coordination {coordination_seconds:.2f}s; "
        f"population {population_seconds:.2f}s; product {product_seconds:.2f}s"
    )

    if summary_path:
        with Path(summary_path).open("a") as summary:
            summary.write(base.render_product_summary(report, inventory, metadata_note))


if __name__ == "__main__":
    main()
