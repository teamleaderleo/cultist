#!/usr/bin/env python3
"""Project a mixed Codex ``exec --json`` log without emitting tool payloads."""

from __future__ import annotations

import argparse
import hashlib
import json
from collections import Counter
from pathlib import Path
from typing import Any


def digest(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def encoded_size(value: Any) -> int:
    if not isinstance(value, str):
        return 0
    return len(value.encode("utf-8"))


def project(raw: bytes, *, include_final: bool, max_final_chars: int) -> dict[str, Any]:
    event_types: Counter[str] = Counter()
    item_types: Counter[str] = Counter()
    item_statuses: Counter[str] = Counter()
    non_json_lines = 0
    non_json_bytes = 0
    malformed_json_lines = 0
    command_count = 0
    command_output_bytes = 0
    command_text_bytes = 0
    agent_messages: list[str] = []
    usage: dict[str, Any] | None = None

    lines = raw.splitlines()
    for line in lines:
        try:
            value = json.loads(line)
        except (UnicodeDecodeError, json.JSONDecodeError):
            non_json_lines += 1
            non_json_bytes += len(line)
            if line.lstrip().startswith((b"{", b"[")):
                malformed_json_lines += 1
            continue
        if not isinstance(value, dict):
            continue
        event_type = str(value.get("type") or "unknown")
        event_types[event_type] += 1
        item = value.get("item")
        if isinstance(item, dict):
            item_type = str(item.get("type") or "unknown")
            item_types[item_type] += 1
            status = item.get("status")
            if isinstance(status, str):
                item_statuses[status] += 1
            if event_type == "item.completed" and item_type == "command_execution":
                command_count += 1
                command_text_bytes += encoded_size(item.get("command"))
                command_output_bytes += encoded_size(item.get("aggregated_output"))
            if event_type == "item.completed" and item_type == "agent_message":
                text = item.get("text")
                if isinstance(text, str):
                    agent_messages.append(text)
        if event_type == "turn.completed" and isinstance(value.get("usage"), dict):
            usage = value["usage"]

    final_text = agent_messages[-1] if agent_messages else None
    result: dict[str, Any] = {
        "schema": "cultist-codex-exec-event-view/v1",
        "source": {
            "utf8Bytes": len(raw),
            "lines": len(lines),
            "sha256": digest(raw),
        },
        "parse": {
            "jsonEvents": sum(event_types.values()),
            "nonJsonLines": non_json_lines,
            "nonJsonBytes": non_json_bytes,
            "malformedJsonLines": malformed_json_lines,
        },
        "eventTypes": dict(sorted(event_types.items())),
        "itemTypes": dict(sorted(item_types.items())),
        "itemStatuses": dict(sorted(item_statuses.items())),
        "commands": {
            "completed": command_count,
            "commandTextBytesOmitted": command_text_bytes,
            "aggregatedOutputBytesOmitted": command_output_bytes,
        },
        "agentMessages": {
            "completed": len(agent_messages),
            "finalAvailable": final_text is not None,
            "finalUtf8Bytes": encoded_size(final_text),
            "finalSha256": digest(final_text.encode()) if final_text is not None else None,
        },
        "usage": usage,
        "omittedClasses": [
            "non-json-lines",
            "command-text",
            "command-output",
            "agent-message-content",
        ],
        "rawContentEmitted": False,
    }
    if include_final and final_text is not None:
        if len(final_text) > max_final_chars:
            result["finalText"] = final_text[:max_final_chars]
            result["finalTruncated"] = True
        else:
            try:
                result["finalStructured"] = json.loads(final_text)
                result["finalTruncated"] = False
            except json.JSONDecodeError:
                result["finalText"] = final_text
                result["finalTruncated"] = False
        result["rawContentEmitted"] = True
        result["omittedClasses"].remove("agent-message-content")
        if len(agent_messages) > 1:
            result["omittedClasses"].append("intermediate-agent-messages")
    return result


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("log", type=Path)
    parser.add_argument(
        "--include-final",
        action="store_true",
        help="emit only the final agent message; structured JSON is preserved",
    )
    parser.add_argument("--max-final-chars", type=int, default=8_000)
    args = parser.parse_args()
    if args.max_final_chars < 1:
        parser.error("--max-final-chars must be positive")
    print(json.dumps(
        project(
            args.log.read_bytes(),
            include_final=args.include_final,
            max_final_chars=args.max_final_chars,
        ),
        ensure_ascii=False,
        indent=2,
        sort_keys=True,
    ))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
