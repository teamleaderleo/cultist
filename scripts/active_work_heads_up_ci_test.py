#!/usr/bin/env python3

from active_work_heads_up_ci import (
    quiet_status_line,
    quiet_summary,
    unresolved_endpoint_note,
    unresolved_endpoint_receipts_involving_current,
)

INVENTORY = {
    "observed_at": "2026-08-23T00:00:00Z",
    "source": "test_provider_snapshot",
    "current": {"id": "#10"},
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

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
