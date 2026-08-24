#!/usr/bin/env python3

from __future__ import annotations

import re

import active_work_heads_up_ci as base
from active_work_heads_up_ci import (
    PR_PAGE_QUERY,
    build_inventory_and_metadata,
    current_provider_work_from_response,
    provider_current_environment,
    provider_current_matches_inventory,
    quiet_status_line,
    quiet_summary,
    unresolved_endpoint_note,
    unresolved_endpoint_receipts_involving_current,
    work_item,
)

INVENTORY = {
    "observed_at": "2026-08-23T00:00:00Z",
    "source": "test_provider_snapshot",
    "current": {
        "id": "#10",
        "head_sha": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    },
}


def provider_response(number: int, head: str | None) -> dict[str, object]:
    return {
        "data": {
            "repository": {
                "pullRequest": {
                    "number": number,
                    "headRefOid": head,
                }
            }
        }
    }


def pr_page_response(
    *,
    pull_requests_has_next: bool = False,
    nodes: list[dict[str, object]] | None = None,
) -> dict[str, object]:
    if nodes is None:
        nodes = [pr_node(10, "a" * 40)]
    return {
        "data": {
            "repository": {
                "pullRequests": {
                    "nodes": nodes,
                    "pageInfo": {
                        "hasNextPage": pull_requests_has_next,
                        "endCursor": (
                            "pulls-next" if pull_requests_has_next else None
                        ),
                    },
                }
            }
        }
    }


def pr_node(number: int, head: str, *, file_count: int = 1) -> dict[str, object]:
    return {
        "number": number,
        "title": f"work {number}",
        "url": f"https://github.com/owner/repo/pull/{number}",
        "body": "",
        "headRefName": f"work-{number}",
        "headRefOid": head,
        "updatedAt": "2026-08-24T00:00:00Z",
        "isDraft": False,
        "files": {
            "nodes": [
                {"path": f"src/file_{index}.rs"} for index in range(file_count)
            ],
            "pageInfo": {"hasNextPage": False, "endCursor": None},
        },
    }


def assert_runtime_error(callback, expected: str) -> None:
    try:
        callback()
    except RuntimeError as error:
        assert expected in str(error), error
    else:
        raise AssertionError(f"expected RuntimeError containing {expected!r}")


def main() -> int:
    clean_status = quiet_status_line()
    assert clean_status == "No active-work coordination signal worth surfacing."
    clean_summary = quiet_summary(INVENTORY)
    assert clean_status in clean_summary
    assert "UNKNOWN" not in clean_summary

    failure_note = (
        "Reviewed coordination metadata could not be fully analyzed; "
        "direct path evidence remains usable. Detail: synthetic extractor failure"
    )
    unknown_status = quiet_status_line(failure_note)
    assert unknown_status == (
        "Coordination metadata: UNKNOWN; direct path evidence found no overlap."
    )
    assert "No active-work coordination signal worth surfacing." not in unknown_status

    unknown_summary = quiet_summary(INVENTORY, failure_note)
    assert unknown_status in unknown_summary
    assert failure_note in unknown_summary
    assert "No active-work coordination signal worth surfacing." not in unknown_summary
    assert "reviewed coordination metadata remains unresolved" in unknown_summary

    extraction_report = {
        "unresolved_endpoint_receipts": [
            {"from": "#10", "to": "#999"},
            {"from": "#20", "to": "#998"},
        ]
    }
    current_unresolved = unresolved_endpoint_receipts_involving_current(
        INVENTORY, extraction_report
    )
    assert current_unresolved == [{"from": "#10", "to": "#999"}]
    endpoint_note = unresolved_endpoint_note(current_unresolved)
    assert endpoint_note is not None
    assert "1 endpoint absent from the supplied work inventory" in endpoint_note
    endpoint_summary = quiet_summary(INVENTORY, endpoint_note)
    assert "Coordination metadata: UNKNOWN" in endpoint_summary
    assert "No active-work coordination signal worth surfacing." not in endpoint_summary

    unrelated_report = {
        "unresolved_endpoint_receipts": [{"from": "#20", "to": "#998"}]
    }
    unrelated = unresolved_endpoint_receipts_involving_current(
        INVENTORY, unrelated_report
    )
    assert unrelated == []
    assert unresolved_endpoint_note(unrelated) is None
    assert quiet_status_line(unresolved_endpoint_note(unrelated)) == clean_status

    exact = current_provider_work_from_response(
        "owner/repo",
        10,
        provider_response(10, "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
    )
    assert exact == {
        "repository": "owner/repo",
        "work_id": "#10",
        "head_sha": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    }
    assert provider_current_matches_inventory(INVENTORY, "owner/repo", exact)

    moved = dict(exact)
    moved["head_sha"] = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
    assert not provider_current_matches_inventory(INVENTORY, "owner/repo", moved)

    unavailable = current_provider_work_from_response(
        "owner/repo",
        10,
        {"data": {"repository": {"pullRequest": None}}},
    )
    assert unavailable == {
        "repository": "owner/repo",
        "work_id": "#10",
        "head_sha": None,
    }
    assert not provider_current_matches_inventory(
        INVENTORY, "owner/repo", unavailable
    )

    wrong_repository = dict(exact)
    wrong_repository["repository"] = "owner/other"
    assert not provider_current_matches_inventory(
        INVENTORY, "owner/repo", wrong_repository
    )

    wrong_work = dict(exact)
    wrong_work["work_id"] = "#11"
    assert not provider_current_matches_inventory(INVENTORY, "owner/repo", wrong_work)

    try:
        current_provider_work_from_response(
            "owner/repo",
            10,
            provider_response(11, "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
        )
    except RuntimeError as error:
        assert "returned #11; expected #10" in str(error)
    else:
        raise AssertionError("wrong provider work identity must fail closed")

    stale_environment = {
        "PATH": "/bin",
        "CULTIST_CURRENT_PROVIDER_HEAD": "checkout-or-stale-head",
    }
    exact_environment = provider_current_environment(
        stale_environment, "owner/repo", exact
    )
    assert exact_environment["CULTIST_REQUIRED_PROVIDER_REPOSITORY"] == "owner/repo"
    assert exact_environment["CULTIST_CURRENT_PROVIDER_REPOSITORY"] == "owner/repo"
    assert exact_environment["CULTIST_CURRENT_PROVIDER_WORK"] == "#10"
    assert exact_environment["CULTIST_CURRENT_PROVIDER_HEAD"] == exact["head_sha"]

    unknown_environment = provider_current_environment(
        stale_environment, "owner/repo", unavailable
    )
    assert "CULTIST_CURRENT_PROVIDER_HEAD" not in unknown_environment
    assert unknown_environment["CULTIST_CURRENT_PROVIDER_WORK"] == "#10"

    # A GraphQL document that uses an undeclared variable is rejected with
    # variableNotDefined before any runtime fail-closed logic can run, so
    # prove the pagination document declares every variable it uses.
    header = re.search(r"query\s*\(([^)]*)\)", PR_PAGE_QUERY)
    assert header is not None, PR_PAGE_QUERY
    declared = set(re.findall(r"\$(\w+)\s*:", header.group(1)))
    used = set(re.findall(r"\$(\w+)", PR_PAGE_QUERY[header.end():]))
    undeclared = sorted(used - declared)
    assert not undeclared, f"PR_PAGE_QUERY uses undeclared variables: {undeclared}"

    # Exact work identity binds head and changed paths to one provider
    # response. Any required pagination must fail closed instead of mixing
    # coordinates across separate reads.
    original_graphql = base.graphql
    try:
        continuation_calls: list[dict[str, object]] = []

        def continuation_graphql(
            query: str, variables: dict[str, object]
        ) -> dict[str, object]:
            assert query == PR_PAGE_QUERY
            continuation_calls.append(variables)
            return pr_page_response(pull_requests_has_next=True)

        base.graphql = continuation_graphql

        continued = pr_node(10, "a" * 40)
        continued["files"] = {
            "nodes": [{"path": "src/file_0.rs"}],
            "pageInfo": {"hasNextPage": True, "endCursor": "files-next"},
        }
        assert_runtime_error(
            lambda: work_item(continued),
            "PR #10 requires file pagination; exact snapshot unavailable",
        )

        assert_runtime_error(
            lambda: build_inventory_and_metadata("owner/repo", 10),
            "provider population requires pull-request pagination",
        )
        assert len(continuation_calls) == 1, continuation_calls
    finally:
        base.graphql = original_graphql

    observed = pr_page_response(
        nodes=[
            pr_node(10, "a" * 40, file_count=2),
            pr_node(20, "b" * 40),
        ]
    )
    calls: list[dict[str, object]] = []

    def single_page_graphql(query: str, variables: dict[str, object]) -> dict[str, object]:
        assert query == PR_PAGE_QUERY
        assert "after" not in variables, variables
        calls.append(variables)
        return observed

    base.graphql = single_page_graphql
    try:
        inventory, metadata = build_inventory_and_metadata("owner/repo", 10)
    finally:
        base.graphql = original_graphql
    assert len(calls) == 1, calls
    assert [item["id"] for item in inventory["active_work"]] == ["#10", "#20"]
    current_item = inventory["current"]
    assert current_item["head_sha"] == "a" * 40, current_item
    assert current_item["changed_paths"] == [
        "src/file_0.rs",
        "src/file_1.rs",
    ], current_item
    assert metadata["work"][0]["head_sha"] == "a" * 40
    assert inventory["source"] == "github_pull_requests_graphql"

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
