#!/usr/bin/env python3

from __future__ import annotations

import active_work_heads_up_ci as base
from active_work_heads_up_snapshot_ci import (
    SINGLE_READ_PR_PAGE_QUERY,
    build_single_read_inventory_and_metadata,
    can_skip_product,
    read_current_provider_snapshot,
)


def provider_response(
    *,
    pull_requests_has_next: bool = False,
    file_count: int = 1,
    first_total: int | None = None,
    last_total: int | None = None,
) -> dict[str, object]:
    paths = [f"src/file_{index:03d}.rs" for index in range(file_count)]
    first_paths = paths[:100]
    last_paths = paths[-100:] if paths else []
    return {
        "data": {
            "repository": {
                "pullRequests": {
                    "nodes": [
                        {
                            "number": 10,
                            "title": "current work",
                            "url": "https://github.com/owner/repo/pull/10",
                            "body": "",
                            "headRefName": "current",
                            "headRefOid": "a" * 40,
                            "updatedAt": "2026-08-24T00:00:00Z",
                            "isDraft": False,
                            "filesFirst": {
                                "totalCount": file_count if first_total is None else first_total,
                                "nodes": [{"path": path} for path in first_paths],
                            },
                            "filesLast": {
                                "totalCount": file_count if last_total is None else last_total,
                                "nodes": [{"path": path} for path in last_paths],
                            },
                        }
                    ],
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


def assert_runtime_error(callback, expected: str) -> None:
    try:
        callback()
    except RuntimeError as error:
        assert expected in str(error), error
    else:
        raise AssertionError(f"expected RuntimeError containing {expected!r}")


def main() -> None:
    identity = "sha256:" + "a" * 64
    moved = "sha256:" + "b" * 64

    assert can_skip_product(
        direct_overlap=False,
        relevant_edges=[],
        exact_current_work=True,
        required_provider_snapshot=identity,
        current_provider_snapshot=identity,
    )

    assert not can_skip_product(
        direct_overlap=False,
        relevant_edges=[],
        exact_current_work=True,
        required_provider_snapshot=identity,
        current_provider_snapshot=moved,
    )
    assert not can_skip_product(
        direct_overlap=False,
        relevant_edges=[],
        exact_current_work=True,
        required_provider_snapshot=identity,
        current_provider_snapshot=None,
    )
    assert not can_skip_product(
        direct_overlap=False,
        relevant_edges=[],
        exact_current_work=False,
        required_provider_snapshot=identity,
        current_provider_snapshot=identity,
    )
    assert not can_skip_product(
        direct_overlap=True,
        relevant_edges=[],
        exact_current_work=True,
        required_provider_snapshot=identity,
        current_provider_snapshot=identity,
    )
    assert not can_skip_product(
        direct_overlap=False,
        relevant_edges=[{"kind": "depends_on"}],
        exact_current_work=True,
        required_provider_snapshot=identity,
        current_provider_snapshot=identity,
    )

    original_graphql = base.graphql
    try:
        calls: list[dict[str, object]] = []

        def exact_graphql(query: str, variables: dict[str, object]) -> dict[str, object]:
            assert query == SINGLE_READ_PR_PAGE_QUERY
            calls.append(variables)
            return provider_response()

        base.graphql = exact_graphql
        inventory, metadata = build_single_read_inventory_and_metadata("owner/repo", 10)
        assert len(calls) == 1, calls
        assert calls[0] == {"owner": "owner", "name": "repo"}, calls[0]
        assert inventory["source"] == "github_pull_requests_graphql_single_read"
        assert inventory["current"]["id"] == "#10"
        assert inventory["active_work"][0]["changed_paths"] == ["src/file_000.rs"]
        assert metadata["work"][0]["id"] == "#10"
        assert read_current_provider_snapshot("owner/repo", 10) is not None

        base.graphql = lambda _query, _variables: provider_response(file_count=102)
        inventory, _metadata = build_single_read_inventory_and_metadata("owner/repo", 10)
        changed_paths = inventory["current"]["changed_paths"]
        assert len(changed_paths) == 102, changed_paths
        assert "src/file_000.rs" in changed_paths
        assert "src/file_101.rs" in changed_paths
        assert read_current_provider_snapshot("owner/repo", 10) is not None

        base.graphql = lambda _query, _variables: provider_response(
            pull_requests_has_next=True
        )
        assert_runtime_error(
            lambda: build_single_read_inventory_and_metadata("owner/repo", 10),
            "pull-request pagination",
        )
        assert read_current_provider_snapshot("owner/repo", 10) is None

        base.graphql = lambda _query, _variables: provider_response(file_count=201)
        assert_runtime_error(
            lambda: build_single_read_inventory_and_metadata("owner/repo", 10),
            "one-response file coverage incomplete for PR #10: 200 of 201",
        )
        assert read_current_provider_snapshot("owner/repo", 10) is None

        base.graphql = lambda _query, _variables: provider_response(
            file_count=102,
            last_total=103,
        )
        assert_runtime_error(
            lambda: build_single_read_inventory_and_metadata("owner/repo", 10),
            "provider file counts disagree within one response: 102 != 103",
        )
        assert read_current_provider_snapshot("owner/repo", 10) is None
    finally:
        base.graphql = original_graphql


if __name__ == "__main__":
    main()
