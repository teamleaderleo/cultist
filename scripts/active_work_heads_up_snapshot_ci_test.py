#!/usr/bin/env python3

from __future__ import annotations

from active_work_heads_up_snapshot_ci import can_skip_product


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


if __name__ == "__main__":
    main()
