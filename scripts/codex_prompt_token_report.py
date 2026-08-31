#!/usr/bin/env python3
"""Tokenize model-visible Codex prompt segments without emitting their text."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Protocol


class Encoder(Protocol):
    def encode(self, text: str, **kwargs: Any) -> list[int]: ...


def canonical(value: Any) -> bytes:
    return json.dumps(value, ensure_ascii=False, separators=(",", ":"), sort_keys=True).encode()


def digest(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def label(text: str, role: str, item_index: int, segment_index: int) -> str:
    markers = (
        ("<skills_instructions>", "skills-catalogue"),
        ("<permissions instructions>", "permissions"),
        ("<collaboration_mode>", "collaboration-mode"),
        ("<apps_instructions>", "apps-instructions"),
        ("<plugins_instructions>", "plugins-instructions"),
        ("You are `/root`", "team-orchestration"),
        ("<multi_agent_mode>", "multi-agent-mode"),
        ("<recommended_plugins>", "recommended-plugins"),
        ("<environment_context>", "environment-context"),
    )
    for marker, name in markers:
        if marker in text:
            return name
    return f"{role}-item-{item_index}-segment-{segment_index}"


def extract_segments(raw: Any, encoder: Encoder) -> list[dict[str, Any]]:
    if not isinstance(raw, list):
        raise ValueError("prompt input must be a JSON list")
    segments: list[dict[str, Any]] = []
    for item_index, item in enumerate(raw):
        if not isinstance(item, dict):
            continue
        role = str(item.get("role") or "unknown")
        content = item.get("content")
        values = content if isinstance(content, list) else [content]
        for segment_index, value in enumerate(values):
            text = value.get("text") if isinstance(value, dict) else value
            if not isinstance(text, str):
                continue
            encoded = text.encode("utf-8")
            segments.append({
                "label": label(text, role, item_index, segment_index),
                "role": role,
                "itemIndex": item_index,
                "segmentIndex": segment_index,
                "characters": len(text),
                "utf8Bytes": len(encoded),
                "tokens": len(encoder.encode(text, disallowed_special=())),
                "sha256": digest(encoded),
            })
    return segments


def arm(raw: Any, encoder: Encoder) -> dict[str, Any]:
    segments = extract_segments(raw, encoder)
    return {
        "segments": segments,
        "segmentCount": len(segments),
        "characters": sum(row["characters"] for row in segments),
        "utf8Bytes": sum(row["utf8Bytes"] for row in segments),
        "textTokens": sum(row["tokens"] for row in segments),
    }


def compare(control: dict[str, Any], treatment: dict[str, Any]) -> dict[str, Any]:
    treatment_hashes = {row["sha256"] for row in treatment["segments"]}
    control_hashes = {row["sha256"] for row in control["segments"]}
    retired = [row for row in control["segments"] if row["sha256"] not in treatment_hashes]
    added = [row for row in treatment["segments"] if row["sha256"] not in control_hashes]
    delta = treatment["textTokens"] - control["textTokens"]
    return {
        "textTokenDelta": delta,
        "textTokenReductionPercent": round(
            -delta * 100 / control["textTokens"], 3
        ) if control["textTokens"] else 0.0,
        "retiredSegments": retired,
        "addedSegments": added,
        "unchangedSegmentCount": len(control_hashes & treatment_hashes),
    }


def private_write(path: Path, value: bytes) -> None:
    descriptor = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    with os.fdopen(descriptor, "wb") as sink:
        sink.write(value)
        sink.flush()
        os.fsync(sink.fileno())


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--control", type=Path, required=True)
    parser.add_argument("--treatment", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--encoding", default="o200k_base")
    args = parser.parse_args()
    try:
        import tiktoken
    except ImportError as error:
        raise SystemExit("tiktoken is required for an OpenAI-tokenizer report") from error
    encoder = tiktoken.get_encoding(args.encoding)
    control = arm(json.loads(args.control.read_text(encoding="utf-8")), encoder)
    treatment = arm(json.loads(args.treatment.read_text(encoding="utf-8")), encoder)
    result = {
        "schema": "cultist-codex-prompt-token-report/v1",
        "recordedAt": datetime.now(timezone.utc).isoformat(),
        "encoding": args.encoding,
        "control": control,
        "treatment": treatment,
        "comparison": compare(control, treatment),
        "countsTextSegmentsOnly": True,
        "countsProtocolOverhead": False,
        "rawContentEmitted": False,
        "authorizesCapabilityRetirement": False,
        "authorizesProductionPromotion": False,
    }
    payload = canonical(result) + b"\n"
    args.output.parent.mkdir(parents=True, exist_ok=True)
    private_write(args.output, payload)
    print(json.dumps({
        "schema": "cultist-codex-prompt-token-report-receipt/v1",
        "resultSha256": digest(payload),
        "encoding": args.encoding,
        "controlTextTokens": control["textTokens"],
        "treatmentTextTokens": treatment["textTokens"],
        "textTokenDelta": result["comparison"]["textTokenDelta"],
        "textTokenReductionPercent": result["comparison"]["textTokenReductionPercent"],
        "rawContentEmitted": False,
    }, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
