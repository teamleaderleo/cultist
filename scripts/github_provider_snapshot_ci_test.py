#!/usr/bin/env python3

from __future__ import annotations

import json

from github_provider_snapshot_ci import derive_provider_snapshot_identities


def inventory() -> dict[str, object]:
    return {
        "schema_version": 1,
        "source": "fixture",
        "observed_at": "2026-08-23T00:00:00Z",
        "current": {
            "id": "#10",
            "head_sha": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "changed_paths": ["src/a.rs"],
        },
        "active_work": [
            {
                "id": "#10",
                "head_sha": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "activity": "confirmed_active",
                "changed_paths": ["src/a.rs", "docs/a.md"],
            },
            {
                "id": "#20",
                "head_sha": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
                "changed_paths": ["src/b.rs"],
            },
        ],
    }


def fake_runner(expected_snapshots: int):
    def run(args: list[str], *, input: str, text: bool) -> str:
        assert args == [
            "cargo",
            "run",
            "--quiet",
            "--example",
            "github_provider_snapshot",
        ]
        assert text is True
        request = json.loads(input)
        assert request["schema_version"] == 1
        assert len(request["snapshots"]) == expected_snapshots
        snapshot = request["snapshots"][0]
        assert snapshot["provider_instance"] == "github.com"
        assert snapshot["collection"] == "teamleaderleo/cultist"
        assert snapshot["work"][0]["number"] == 10
        assert snapshot["work"][1]["activity"] == "confirmed_active"
        return json.dumps(
            {
                "schema_version": 1,
                "snapshots": [
                    {"provider_snapshot_identity": f"sha256:{index:064x}"}
                    for index in range(1, expected_snapshots + 1)
                ],
            }
        )

    return run


def main() -> None:
    first = inventory()
    second = inventory()
    second["active_work"] = list(second["active_work"]) + [
        {
            "id": "#30",
            "head_sha": "cccccccccccccccccccccccccccccccccccccccc",
            "activity": "confirmed_active",
            "changed_paths": ["src/c.rs"],
        }
    ]
    edges = [
        {
            "kind": "depends_on",
            "from": "#10",
            "to": "#20",
            "source": "fixture",
        }
    ]

    identities = derive_provider_snapshot_identities(
        "teamleaderleo/cultist",
        [(first, edges), (second, [])],
        runner=fake_runner(2),
    )
    assert identities == [f"sha256:{1:064x}", f"sha256:{2:064x}"]

    bad = inventory()
    bad["active_work"] = [
        {
            "id": "pull/10",
            "head_sha": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "changed_paths": ["src/a.rs"],
        }
    ]
    try:
        derive_provider_snapshot_identities(
            "teamleaderleo/cultist", [(bad, [])], runner=fake_runner(1)
        )
    except RuntimeError as error:
        assert "#<number>" in str(error)
    else:
        raise AssertionError("noncanonical GitHub work id was accepted")

    # Exercise the real Rust wire -> merged product producer path on hosted CI.
    actual = derive_provider_snapshot_identities(
        "teamleaderleo/cultist", [(inventory(), edges)]
    )[0]
    reordered = inventory()
    reordered_work = list(reordered["active_work"])
    reordered_work.reverse()
    reordered_work[1]["changed_paths"] = list(reversed(reordered_work[1]["changed_paths"]))
    reordered["active_work"] = reordered_work
    same = derive_provider_snapshot_identities(
        "TeamLeaderLeo/Cultist", [(reordered, edges)]
    )[0]
    assert actual == same

    moved = derive_provider_snapshot_identities(
        "teamleaderleo/cultist", [(second, edges)]
    )[0]
    assert actual != moved


if __name__ == "__main__":
    main()
