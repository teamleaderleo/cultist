#!/usr/bin/env python3

from __future__ import annotations

import active_work_heads_up_ci as base
from active_work_heads_up_snapshot_ci import (
    build_single_read_inventory_and_metadata,
    can_skip_product,
    read_current_provider_snapshot,
)


def provider_response(
    *,
    pull_requests_has_next: bool = False,
    files_has_next: bool = False,
) -> dict[str, object]:
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
                            "files": {
                                "nodes": [{"path": "src/lib.rs"}],
                                "pageInfo": {
                                    "hasNextPage": files_has_next,
                                    "endCursor": "files-next" if files_has_next else None,
                                },
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
            assert query == base.PR_PAGE_QUERY
            calls.append(variables)
            return provider_response()

        base.graphql = exact_graphql
        inventory, metadata = build_single_read_inventory_and_metadata("owner/repo", 10)
        assert len(calls) == 1, calls
        assert calls[0]["after"] is None
        assert inventory["source"] == "github_pull_requests_graphql_single_read"
        assert inventory["current"]["id"] == "#10"
        assert inventory["active_work"][0]["changed_paths"] == ["src/lib.rs"]
        assert metadata["work"][0]["id"] == "#10"
        assert read_current_provider_snapshot("owner/repo", 10) is not None

        base.graphql = lambda _query, _variables: provider_response(
            pull_requests_has_next=True
        )
        assert_runtime_error(
            lambda: build_single_read_inventory_and_metadata("owner/repo", 10),
            "pull-request pagination",
        )
        assert read_current_provider_snapshot("owner/repo", 10) is None

        base.graphql = lambda _query, _variables: provider_response(files_has_next=True)
        assert_runtime_error(
            lambda: build_single_read_inventory_and_metadata("owner/repo", 10),
            "file pagination for PR #10",
        )
        assert read_current_provider_snapshot("owner/repo", 10) is None
    finally:
        base.graphql = original_graphql


if __name__ == "__main__":
    main()
