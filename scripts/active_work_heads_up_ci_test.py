#!/usr/bin/env python3

from active_work_heads_up_ci import quiet_status_line, quiet_summary

INVENTORY = {
    "observed_at": "2026-08-23T00:00:00Z",
    "source": "test_provider_snapshot",
}


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

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
