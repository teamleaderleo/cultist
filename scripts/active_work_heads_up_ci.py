#!/usr/bin/env python3
"""Build live PR evidence, run Cultist preflight, and render CI advice."""

from __future__ import annotations

import datetime
import json
import os
from pathlib import Path
import subprocess
import tempfile
import time

COORDINATION_PREFIX = "Do not merge while #"

PR_PAGE_QUERY = r"""
query($owner: String!, $name: String!, $after: String) {
  repository(owner: $owner, name: $name) {
    pullRequests(
      states: OPEN
      first: 100
      after: $after
      orderBy: {field: UPDATED_AT, direction: DESC}
    ) {
      nodes {
        number
        title
        url
        body
        headRefName
        headRefOid
        updatedAt
        isDraft
        files(first: 100) {
          nodes { path }
          pageInfo { hasNextPage endCursor }
        }
      }
      pageInfo { hasNextPage endCursor }
    }
  }
}
"""

PR_FILES_QUERY = r"""
query($owner: String!, $name: String!, $number: Int!, $after: String) {
  repository(owner: $owner, name: $name) {
    pullRequest(number: $number) {
      files(first: 100, after: $after) {
        nodes { path }
        pageInfo { hasNextPage endCursor }
      }
    }
  }
}
"""

CURRENT_PR_QUERY = r"""
query($owner: String!, $name: String!, $number: Int!) {
  repository(owner: $owner, name: $name) {
    pullRequest(number: $number) {
      number
      headRefOid
    }
  }
}
"""


def gh_json(args: list[str]) -> object:
    output = subprocess.check_output(["gh", *args], text=True)
    return json.loads(output)


def graphql(query: str, variables: dict[str, object]) -> dict[str, object]:
    args = ["api", "graphql", "-f", f"query={query}"]
    for key, value in variables.items():
        if value is None:
            continue
        args.extend(["-F", f"{key}={value}"])
    result = gh_json(args)
    if not isinstance(result, dict):
        raise RuntimeError("unexpected GitHub GraphQL response")
    return result


def page_info(connection: dict[str, object]) -> tuple[bool, str | None]:
    info = connection.get("pageInfo")
    if not isinstance(info, dict):
        raise RuntimeError("GitHub connection is missing pageInfo")
    cursor = info.get("endCursor")
    return bool(info.get("hasNextPage", False)), str(cursor) if cursor else None


def file_paths_from_connection(connection: dict[str, object]) -> list[str]:
    nodes = connection.get("nodes")
    if not isinstance(nodes, list):
        raise RuntimeError("GitHub files connection is missing nodes")
    return [str(node["path"]) for node in nodes if isinstance(node, dict)]


def remaining_file_paths(
    owner: str,
    name: str,
    number: int,
    after: str | None,
) -> list[str]:
    paths: list[str] = []
    cursor = after
    while cursor is not None:
        result = graphql(
            PR_FILES_QUERY,
            {"owner": owner, "name": name, "number": number, "after": cursor},
        )
        data = result.get("data")
        repository = data.get("repository") if isinstance(data, dict) else None
        pull_request = (
            repository.get("pullRequest") if isinstance(repository, dict) else None
        )
        files = pull_request.get("files") if isinstance(pull_request, dict) else None
        if not isinstance(files, dict):
            raise RuntimeError(f"could not paginate files for PR #{number}")
        paths.extend(file_paths_from_connection(files))
        has_next, cursor = page_info(files)
        if not has_next:
            break
    return paths


def work_item(
    owner: str,
    name: str,
    node: dict[str, object],
) -> dict[str, object]:
    number = int(node["number"])
    files = node.get("files")
    if not isinstance(files, dict):
        raise RuntimeError(f"PR #{number} has no readable files connection")

    paths = file_paths_from_connection(files)
    has_next, cursor = page_info(files)
    if has_next:
        paths.extend(remaining_file_paths(owner, name, number, cursor))

    return {
        "id": f"#{number}",
        "kind": "pull_request",
        "title": str(node["title"]),
        "url": str(node["url"]),
        "head_ref": str(node["headRefName"]),
        "head_sha": str(node["headRefOid"]),
        "updated_at": str(node["updatedAt"]),
        "draft": bool(node.get("isDraft", False)),
        "changed_paths": sorted(set(paths)),
    }


def metadata_item(node: dict[str, object]) -> dict[str, object]:
    number = int(node["number"])
    body = node.get("body")
    return {
        "id": f"#{number}",
        "kind": "pull_request",
        "source": f"github:pull/{number}",
        "head_sha": str(node["headRefOid"]),
        "updated_at": str(node["updatedAt"]),
        "body": "" if body is None else str(body),
    }


def build_inventory_and_metadata(
    repo: str,
    current_number: int,
) -> tuple[dict[str, object], dict[str, object]]:
    owner, name = repo.split("/", 1)
    work: list[dict[str, object]] = []
    metadata_work: list[dict[str, object]] = []
    cursor: str | None = None

    while True:
        result = graphql(
            PR_PAGE_QUERY,
            {"owner": owner, "name": name, "after": cursor},
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

        for node in nodes:
            if not isinstance(node, dict):
                continue
            work.append(work_item(owner, name, node))
            metadata_work.append(metadata_item(node))

        has_next, cursor = page_info(pull_requests)
        if not has_next:
            break
        if cursor is None:
            raise RuntimeError("pull-request pagination promised another page without cursor")

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
        "source": "github_pull_requests_graphql",
        "observed_at": observed_at,
        "current": current,
        "active_work": work,
    }
    metadata = {
        "schema_version": 1,
        "source": "github_pull_requests_graphql",
        "work": metadata_work,
    }
    return inventory, metadata


def build_inventory(repo: str, current_number: int) -> dict[str, object]:
    inventory, _metadata = build_inventory_and_metadata(repo, current_number)
    return inventory


def current_provider_work_from_response(
    repo: str,
    current_number: int,
    result: dict[str, object],
) -> dict[str, object]:
    data = result.get("data")
    repository = data.get("repository") if isinstance(data, dict) else None
    pull_request = repository.get("pullRequest") if isinstance(repository, dict) else None
    if pull_request is None:
        return {
            "repository": repo,
            "work_id": f"#{current_number}",
            "head_sha": None,
        }
    if not isinstance(pull_request, dict):
        raise RuntimeError("current pull-request response has unexpected type")

    number = int(pull_request["number"])
    if number != current_number:
        raise RuntimeError(
            f"current pull-request re-read returned #{number}; expected #{current_number}"
        )
    head = pull_request.get("headRefOid")
    return {
        "repository": repo,
        "work_id": f"#{number}",
        "head_sha": str(head) if head else None,
    }


def read_current_provider_work(
    repo: str,
    current_number: int,
) -> dict[str, object]:
    owner, name = repo.split("/", 1)
    try:
        result = graphql(
            CURRENT_PR_QUERY,
            {"owner": owner, "name": name, "number": current_number},
        )
        return current_provider_work_from_response(repo, current_number, result)
    except (
        subprocess.CalledProcessError,
        json.JSONDecodeError,
        KeyError,
        RuntimeError,
        TypeError,
        ValueError,
    ) as error:
        print(f"provider-current work unavailable: {error}")
        return {
            "repository": repo,
            "work_id": f"#{current_number}",
            "head_sha": None,
        }


def provider_current_matches_inventory(
    inventory: dict[str, object],
    required_repository: str,
    provider_current: dict[str, object],
) -> bool:
    current = inventory.get("current")
    if not isinstance(current, dict):
        raise RuntimeError("current work item must be an object")
    head = provider_current.get("head_sha")
    return (
        str(provider_current.get("repository", "")) == required_repository
        and str(provider_current.get("work_id", "")) == str(current.get("id", ""))
        and isinstance(head, str)
        and bool(head)
        and head == str(current.get("head_sha", ""))
    )


def provider_current_environment(
    base: dict[str, str],
    required_repository: str,
    provider_current: dict[str, object],
) -> dict[str, str]:
    environment = dict(base)
    environment["CULTIST_REQUIRED_PROVIDER_REPOSITORY"] = required_repository
    environment["CULTIST_CURRENT_PROVIDER_REPOSITORY"] = str(
        provider_current["repository"]
    )
    environment["CULTIST_CURRENT_PROVIDER_WORK"] = str(provider_current["work_id"])
    head = provider_current.get("head_sha")
    if isinstance(head, str) and head:
        environment["CULTIST_CURRENT_PROVIDER_HEAD"] = head
    else:
        environment.pop("CULTIST_CURRENT_PROVIDER_HEAD", None)
    return environment


def potential_direct_overlap(inventory: dict[str, object]) -> bool:
    current = inventory["current"]
    active_work = inventory["active_work"]
    if not isinstance(current, dict) or not isinstance(active_work, list):
        raise RuntimeError("inventory work fields have unexpected types")

    current_paths = {str(path) for path in current["changed_paths"]}
    current_id = str(current["id"])
    for work in active_work:
        if not isinstance(work, dict):
            continue
        if str(work["id"]) == current_id:
            continue
        if current_paths.intersection(str(path) for path in work["changed_paths"]):
            return True
    return False


def potential_coordination_clause(metadata: dict[str, object]) -> bool:
    work = metadata.get("work")
    if not isinstance(work, list):
        raise RuntimeError("metadata work field must be a list")
    return any(
        isinstance(item, dict)
        and any(
            line.strip("\r").startswith(COORDINATION_PREFIX)
            for line in str(item.get("body", "")).splitlines()
        )
        for item in work
    )


def extract_coordination_edges(
    metadata: dict[str, object],
) -> tuple[list[dict[str, object]], dict[str, object]]:
    output = subprocess.check_output(
        ["cargo", "run", "--quiet", "--example", "coordination_edges"],
        input=json.dumps(metadata),
        text=True,
    )
    report = json.loads(output)
    edges = report.get("coordination_edges")
    if not isinstance(edges, list):
        raise RuntimeError("coordination extractor did not return an edge list")
    return [edge for edge in edges if isinstance(edge, dict)], report


def edge_involves_current(inventory: dict[str, object], edge: dict[str, object]) -> bool:
    current = inventory.get("current")
    if not isinstance(current, dict):
        raise RuntimeError("current work item must be an object")
    current_id = str(current["id"])
    return str(edge.get("from", "")) == current_id or str(edge.get("to", "")) == current_id


def unresolved_endpoint_receipts_involving_current(
    inventory: dict[str, object], extraction_report: dict[str, object]
) -> list[dict[str, object]]:
    receipts = extraction_report.get("unresolved_endpoint_receipts")
    if not isinstance(receipts, list) or any(
        not isinstance(receipt, dict) for receipt in receipts
    ):
        raise RuntimeError(
            "coordination extractor did not return unresolved endpoint receipts"
        )
    return [
        receipt
        for receipt in receipts
        if edge_involves_current(inventory, receipt)
    ]


def unresolved_endpoint_note(receipts: list[dict[str, object]]) -> str | None:
    if not receipts:
        return None
    count = len(receipts)
    noun = "endpoint" if count == 1 else "endpoints"
    return (
        "Reviewed coordination metadata for current work references "
        f"{count} {noun} absent from the supplied work inventory; "
        "current coordination relevance remains unresolved."
    )


def quiet_status_line(metadata_note: str | None = None) -> str:
    if metadata_note:
        return "Coordination metadata: UNKNOWN; direct path evidence found no overlap."
    return "No active-work coordination signal worth surfacing."


def quiet_summary(
    inventory: dict[str, object],
    metadata_note: str | None = None,
) -> str:
    lines = [
        "## Cultist active-work heads-up",
        "",
        f"Observed `{inventory['observed_at']}` from `{inventory['source']}`.",
        "",
        quiet_status_line(metadata_note),
        "",
    ]
    if metadata_note:
        lines.extend(
            [
                "> Direct path evidence is quiet; reviewed coordination metadata remains unresolved.",
                "",
                f"> {metadata_note}",
            ]
        )
    else:
        lines.append(
            "> Advisory only. Disjoint paths and absent reviewed metadata edges do not prove semantic independence."
        )
    lines.append("")
    return "\n".join(lines)


def claim_kind_label(kind: object) -> str:
    return str(kind).upper()


def render_product_summary(
    report: dict[str, object],
    inventory: dict[str, object],
    metadata_note: str | None = None,
) -> str:
    findings = report.get("findings")
    if not isinstance(findings, list):
        raise RuntimeError("product preflight report findings must be a list")

    lines = [
        "## Cultist active-work heads-up",
        "",
        f"Observed `{inventory['observed_at']}` from `{inventory['source']}`.",
        "",
    ]

    if not findings:
        lines.append("No active-work coordination signal worth surfacing.")
    else:
        lines.extend([f"**Heads up: {len(findings)} preflight finding(s).**", ""])
        for finding in findings:
            if not isinstance(finding, dict):
                continue
            lines.append(f"### {finding.get('title', finding.get('kind', 'Finding'))}")
            location = finding.get("location")
            if isinstance(location, dict):
                path = location.get("path")
                line = location.get("line")
                if path:
                    suffix = f":{line}" if line else ""
                    lines.append(f"- Location: `{path}{suffix}`")

            claims = finding.get("claims")
            if isinstance(claims, list):
                for claim in claims:
                    if not isinstance(claim, dict):
                        continue
                    lines.append(
                        f"- {claim_kind_label(claim.get('kind'))}: {claim.get('message', '')}"
                    )
                    evidence = claim.get("evidence")
                    if isinstance(evidence, list):
                        for item in evidence:
                            if isinstance(item, dict) and item.get("message"):
                                lines.append(f"  - evidence: {item['message']}")
            question = finding.get("question")
            if question:
                lines.append(f"- Question: {question}")
            lines.append("")

    if metadata_note:
        lines.append(f"> {metadata_note}")
        lines.append("")
    lines.append("> Advisory only. Inspect the evidence and coordinate only when useful.")
    lines.append("")
    return "\n".join(lines)


def run_product_preflight(
    inventory: dict[str, object],
    required_repository: str,
    provider_current: dict[str, object],
) -> dict[str, object]:
    with tempfile.TemporaryDirectory(prefix="cultist-active-work-") as temporary:
        inventory_path = Path(temporary, "inventory.json")
        inventory_path.write_text(json.dumps(inventory, indent=2) + "\n")
        environment = provider_current_environment(
            dict(os.environ), required_repository, provider_current
        )
        output = subprocess.check_output(
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
                ".",
            ],
            text=True,
            env=environment,
        )
    report = json.loads(output)
    if not isinstance(report, dict):
        raise RuntimeError("product preflight did not return a JSON object")
    return report


def main() -> None:
    repo = os.environ["GITHUB_REPOSITORY"]
    current_number = int(os.environ["CURRENT_PR"])

    inventory_started = time.monotonic()
    inventory, metadata = build_inventory_and_metadata(repo, current_number)
    inventory_seconds = time.monotonic() - inventory_started
    provider_current = read_current_provider_work(repo, current_number)

    current = inventory["current"]
    if not isinstance(current, dict):
        raise RuntimeError("current work item must be an object")
    active_work = inventory["active_work"]
    if not isinstance(active_work, list):
        raise RuntimeError("active_work must be a list")

    current_head = provider_current.get("head_sha")
    current_head_label = str(current_head) if current_head else "<unavailable>"
    print(
        f"inventory: {len(active_work)} open PR(s), current #{current_number}, "
        f"{len(current['changed_paths'])} current path(s)"
    )
    print(
        "provider-current work: "
        f"{provider_current['repository']} {provider_current['work_id']} @ {current_head_label}"
    )

    direct_overlap = potential_direct_overlap(inventory)
    metadata_candidate = potential_coordination_clause(metadata)
    coordination_seconds = 0.0
    metadata_note: str | None = None
    relevant_edges: list[dict[str, object]] = []

    if metadata_candidate:
        started = time.monotonic()
        try:
            edges, extraction_report = extract_coordination_edges(metadata)
            relevant_edges = [
                edge for edge in edges if edge_involves_current(inventory, edge)
            ]
            print(
                "coordination metadata: "
                f"{len(edges)} extracted edge(s), {len(relevant_edges)} involving current work"
            )
            current_unresolved = unresolved_endpoint_receipts_involving_current(
                inventory, extraction_report
            )
            notes: list[str] = []
            unresolved_note = unresolved_endpoint_note(current_unresolved)
            if unresolved_note:
                notes.append(unresolved_note)
            unknowns = extraction_report.get("unknowns")
            if relevant_edges and isinstance(unknowns, list) and unknowns:
                notes.append(str(unknowns[0]))
            if notes:
                metadata_note = " ".join(notes)
        except (subprocess.CalledProcessError, json.JSONDecodeError, RuntimeError) as error:
            metadata_note = (
                "Reviewed coordination metadata could not be fully analyzed; "
                f"direct path evidence remains usable. Detail: {error}"
            )
            print(f"coordination metadata unavailable: {error}")
        coordination_seconds = time.monotonic() - started

    summary_path = os.environ.get("GITHUB_STEP_SUMMARY")
    exact_current_work = provider_current_matches_inventory(
        inventory, repo, provider_current
    )
    if not direct_overlap and not relevant_edges and exact_current_work:
        print(quiet_status_line(metadata_note))
        print(
            f"timing: inventory {inventory_seconds:.2f}s; "
            f"coordination {coordination_seconds:.2f}s; product 0.00s"
        )
        if summary_path:
            with Path(summary_path).open("a") as summary:
                summary.write(quiet_summary(inventory, metadata_note))
        return

    if relevant_edges:
        inventory["coordination_edges"] = relevant_edges

    product_started = time.monotonic()
    report = run_product_preflight(inventory, repo, provider_current)
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
        f"coordination {coordination_seconds:.2f}s; product {product_seconds:.2f}s"
    )

    if summary_path:
        with Path(summary_path).open("a") as summary:
            summary.write(render_product_summary(report, inventory, metadata_note))


if __name__ == "__main__":
    main()
