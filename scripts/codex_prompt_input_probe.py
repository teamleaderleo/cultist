#!/usr/bin/env python3
"""Measure a one-variable Codex model-visible prompt-input ablation."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import subprocess
from datetime import datetime, timezone
from pathlib import Path
from typing import Any


SHA256 = re.compile(r"^[0-9a-f]{64}$")
SKILL_MARKERS = ("<skills_instructions>", "## Skills", "Available skills", "skills/config/write")
VOLATILE_KEYS = {"id", "internal_chat_message_metadata_passthrough"}


class ProbeError(RuntimeError):
    pass


def canonical(value: Any) -> bytes:
    return json.dumps(value, ensure_ascii=False, separators=(",", ":"), sort_keys=True).encode("utf-8")


def digest(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def private_write(path: Path, value: bytes) -> None:
    descriptor = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    with os.fdopen(descriptor, "wb") as sink:
        sink.write(value)
        sink.flush()
        os.fsync(sink.fileno())


def strings(value: Any):
    if isinstance(value, str):
        yield value
    elif isinstance(value, list):
        for item in value:
            yield from strings(item)
    elif isinstance(value, dict):
        for item in value.values():
            yield from strings(item)


def normalized(value: Any) -> Any:
    if isinstance(value, list):
        return [normalized(item) for item in value]
    if isinstance(value, dict):
        return {
            key: normalized(item)
            for key, item in value.items()
            if key not in VOLATILE_KEYS
        }
    return value


def item_shape(item: Any, index: int) -> dict[str, Any]:
    encoded = canonical(item)
    semantic = canonical(normalized(item))
    text = "\n".join(strings(item))
    return {
        "index": index,
        "type": item.get("type") if isinstance(item, dict) else type(item).__name__,
        "role": item.get("role") if isinstance(item, dict) else None,
        "keys": sorted(item) if isinstance(item, dict) else [],
        "serializedBytes": len(encoded),
        "textCharacters": len(text),
        "rawSha256": digest(encoded),
        "sha256": digest(semantic),
        "skillMarkerPresent": any(marker in text for marker in SKILL_MARKERS),
    }


def shape(raw: Any) -> dict[str, Any]:
    if not isinstance(raw, list):
        raise ProbeError("prompt-input output must be a JSON list")
    items = [item_shape(item, index) for index, item in enumerate(raw)]
    encoded = canonical(raw)
    return {
        "items": items,
        "itemCount": len(items),
        "serializedBytes": len(encoded),
        "textCharacters": sum(item["textCharacters"] for item in items),
        "skillMarkerItems": sum(1 for item in items if item["skillMarkerPresent"]),
        "sha256": digest(encoded),
    }


def extract_packet(pair_path: Path, run_index: int) -> tuple[str, str]:
    pair = json.loads(pair_path.read_text(encoding="utf-8"))
    runs = pair.get("runs") if isinstance(pair, dict) else None
    if not isinstance(runs, list) or not 0 <= run_index < len(runs):
        raise ProbeError("pair run index is unavailable")
    run = runs[run_index]
    packet = run.get("raw_worker_packet")
    claimed = run.get("worker_packet_sha256")
    if not isinstance(packet, str) or not isinstance(claimed, str) or not claimed.startswith("sha256:"):
        raise ProbeError("pair run lacks an exact worker packet")
    calculated = digest(packet.encode("utf-8"))
    if claimed != f"sha256:{calculated}":
        raise ProbeError("worker packet digest does not match exact bytes")
    return packet, calculated


def run_prompt_input(codex: Path, cwd: Path, prompt: str, override: str | None) -> tuple[Any, dict[str, Any]]:
    argv = [str(codex), "debug", "prompt-input"]
    if override is not None:
        argv.extend(["-c", override])
    argv.append(prompt)
    process = subprocess.run(argv, cwd=cwd, check=False, capture_output=True, text=True)
    if process.returncode != 0:
        raise ProbeError(f"prompt-input failed with exit {process.returncode}: {process.stderr[:240]}")
    if process.stderr:
        raise ProbeError("prompt-input emitted stderr; preserve it outside an admitted pair")
    try:
        raw = json.loads(process.stdout)
    except json.JSONDecodeError as error:
        raise ProbeError("prompt-input returned invalid JSON") from error
    return raw, shape(raw)


def compare(control: dict[str, Any], treatment: dict[str, Any]) -> dict[str, Any]:
    control_hashes = {item["sha256"] for item in control["items"]}
    treatment_hashes = {item["sha256"] for item in treatment["items"]}
    removed = [item for item in control["items"] if item["sha256"] not in treatment_hashes]
    added = [item for item in treatment["items"] if item["sha256"] not in control_hashes]
    return {
        "serializedByteDelta": treatment["serializedBytes"] - control["serializedBytes"],
        "serializedByteReductionPercent": round(
            (control["serializedBytes"] - treatment["serializedBytes"]) * 100 / control["serializedBytes"], 3
        ) if control["serializedBytes"] else 0.0,
        "textCharacterDelta": treatment["textCharacters"] - control["textCharacters"],
        "removedItems": removed,
        "addedItems": added,
        "unchangedItems": len(control_hashes & treatment_hashes),
        "removedSkillMarkerItems": sum(1 for item in removed if item["skillMarkerPresent"]),
        "treatmentSkillMarkerItems": treatment["skillMarkerItems"],
        "effectiveCatalogueSuppression": (
            control["skillMarkerItems"] > 0 and treatment["skillMarkerItems"] == 0
        ),
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--pair", type=Path, required=True)
    parser.add_argument("--run-index", type=int, default=0)
    parser.add_argument("--codex", type=Path, required=True)
    parser.add_argument("--cwd", type=Path, required=True)
    parser.add_argument("--output-dir", type=Path, required=True)
    parser.add_argument("--treatment-override", default="skills.include_instructions=false")
    args = parser.parse_args()
    output = args.output_dir.expanduser().resolve()
    if output.exists():
        raise SystemExit("refusing to overwrite prompt-input probe")
    output.mkdir(parents=True, mode=0o700)
    os.chmod(output, 0o700)
    packet, packet_sha = extract_packet(args.pair, args.run_index)
    control_raw, control = run_prompt_input(args.codex, args.cwd, packet, None)
    treatment_raw, treatment = run_prompt_input(
        args.codex, args.cwd, packet, args.treatment_override
    )
    private_write(output / "control.raw.json", canonical(control_raw) + b"\n")
    private_write(output / "treatment.raw.json", canonical(treatment_raw) + b"\n")
    result = {
        "schema": "cultist-codex-prompt-input-ablation/v1",
        "recordedAt": datetime.now(timezone.utc).isoformat(),
        "codexVersion": subprocess.run(
            [str(args.codex), "--version"], check=True, capture_output=True, text=True
        ).stdout.strip(),
        "packetSha256": packet_sha,
        "packetCharacters": len(packet),
        "runIndex": args.run_index,
        "treatmentOverride": args.treatment_override,
        "control": control,
        "treatment": treatment,
        "comparison": compare(control, treatment),
        "rawContentEmitted": False,
        "authorizesCapabilityRetirement": False,
        "authorizesProductionPromotion": False,
    }
    result_bytes = canonical(result) + b"\n"
    private_write(output / "result.json", result_bytes)
    receipt = {
        "schema": "cultist-codex-prompt-input-ablation-receipt/v1",
        "resultSha256": digest(result_bytes),
        "controlPromptInputSha256": control["sha256"],
        "treatmentPromptInputSha256": treatment["sha256"],
        "serializedByteReductionPercent": result["comparison"]["serializedByteReductionPercent"],
        "effectiveCatalogueSuppression": result["comparison"]["effectiveCatalogueSuppression"],
        "rawContentEmitted": False,
    }
    private_write(output / "receipt.json", canonical(receipt) + b"\n")
    print(json.dumps(receipt, sort_keys=True))
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, json.JSONDecodeError, ProbeError, subprocess.SubprocessError) as error:
        raise SystemExit(str(error)) from error
