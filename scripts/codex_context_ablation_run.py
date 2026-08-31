#!/usr/bin/env python3
"""Run one fresh control/treatment Codex context-retirement pair."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import platform
import subprocess
from datetime import datetime, timezone
from pathlib import Path
from typing import Any


class AblationError(RuntimeError):
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


def load_packet(pair_path: Path, run_index: int) -> tuple[str, str]:
    pair = json.loads(pair_path.read_text(encoding="utf-8"))
    runs = pair.get("runs") if isinstance(pair, dict) else None
    if not isinstance(runs, list) or not 0 <= run_index < len(runs):
        raise AblationError("pair run index is unavailable")
    run = runs[run_index]
    packet = run.get("raw_worker_packet")
    claimed = run.get("worker_packet_sha256")
    if not isinstance(packet, str) or not isinstance(claimed, str):
        raise AblationError("pair run lacks exact packet evidence")
    packet_sha = digest(packet.encode("utf-8"))
    if claimed != f"sha256:{packet_sha}":
        raise AblationError("packet digest mismatch")
    return packet, packet_sha


def parse_events(text: str) -> dict[str, Any]:
    events = []
    for line in text.splitlines():
        if line.strip():
            try:
                events.append(json.loads(line))
            except json.JSONDecodeError as error:
                raise AblationError("Codex event stream contains invalid JSON") from error
    thread_ids = [event.get("thread_id") for event in events if event.get("type") == "thread.started"]
    completions = [event.get("usage") for event in events if event.get("type") == "turn.completed"]
    messages = [
        event.get("item", {}).get("text")
        for event in events
        if event.get("type") == "item.completed"
        and event.get("item", {}).get("type") == "agent_message"
    ]
    errors = [
        event.get("item", {}).get("message", "")
        for event in events
        if event.get("type") == "item.completed"
        and event.get("item", {}).get("type") == "error"
    ]
    if len(thread_ids) != 1 or len(completions) != 1 or len(messages) != 1:
        raise AblationError("Codex event stream lacks one complete fresh observation")
    try:
        observation = json.loads(messages[0])
    except json.JSONDecodeError as error:
        raise AblationError("Codex agent message is not the required JSON observation") from error
    first_action = observation.get("first_action_id")
    if not isinstance(first_action, str) or not first_action:
        raise AblationError("observation lacks first_action_id")
    usage = completions[0]
    required_usage = {
        "input_tokens", "cached_input_tokens", "output_tokens", "reasoning_output_tokens"
    }
    if not isinstance(usage, dict) or not required_usage <= set(usage):
        raise AblationError("Codex completion lacks usage counters")
    return {
        "threadId": thread_ids[0],
        "usage": {key: usage[key] for key in sorted(required_usage)},
        "firstActionId": first_action,
        "observationSha256": digest(canonical(observation)),
        "skillBudgetWarning": any("skills context budget" in value for value in errors),
        "errorItemCount": len(errors),
        "eventCount": len(events),
    }


def invocation(
    codex: Path,
    session_root: Path,
    schema: Path,
    model: str,
    reasoning: str,
    override: str | None,
) -> list[str]:
    argv = [
        str(codex), "-a", "never", "exec", "--ephemeral", "--ignore-user-config",
        "--skip-git-repo-check", "--json", "-C", str(session_root), "-s", "read-only",
        "-m", model, "-c", f'model_reasoning_effort="{reasoning}"',
    ]
    if override is not None:
        argv.extend(["-c", override])
    argv.extend(["--output-schema", str(schema), "-"])
    return argv


def run_arm(name: str, argv: list[str], packet: str, output: Path) -> dict[str, Any]:
    process = subprocess.run(
        argv, input=packet, check=False, capture_output=True, text=True
    )
    stdout = process.stdout.encode("utf-8")
    stderr = process.stderr.encode("utf-8")
    private_write(output / f"{name}.events.jsonl", stdout)
    private_write(output / f"{name}.stderr.txt", stderr)
    if process.returncode != 0:
        raise AblationError(f"{name} Codex execution failed with exit {process.returncode}")
    parsed = parse_events(process.stdout)
    parsed.update({
        "stdoutSha256": digest(stdout),
        "stderrSha256": digest(stderr),
        "stderrBytes": len(stderr),
        "invocationSha256": digest(canonical(argv)),
    })
    return parsed


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--pair", type=Path, required=True)
    parser.add_argument("--run-index", type=int, default=0)
    parser.add_argument("--schema", type=Path, required=True)
    parser.add_argument("--prompt-probe-result", type=Path, required=True)
    parser.add_argument("--codex", type=Path, required=True)
    parser.add_argument("--session-root", type=Path, required=True)
    parser.add_argument("--output-dir", type=Path, required=True)
    parser.add_argument("--model", default="gpt-5.6-luna")
    parser.add_argument("--reasoning", default="max")
    parser.add_argument("--treatment-override", default="skills.include_instructions=false")
    parser.add_argument(
        "--order", choices=("control-treatment", "treatment-control"),
        default="control-treatment",
    )
    args = parser.parse_args()
    output = args.output_dir.expanduser().resolve()
    if output.exists():
        raise SystemExit("refusing to overwrite context ablation pair")
    output.mkdir(parents=True, mode=0o700)
    os.chmod(output, 0o700)
    packet, packet_sha = load_packet(args.pair, args.run_index)
    prompt_probe = json.loads(args.prompt_probe_result.read_text(encoding="utf-8"))
    if prompt_probe.get("packetSha256") != packet_sha:
        raise AblationError("prompt-input proof does not bind the selected packet")
    if prompt_probe.get("treatmentOverride") != args.treatment_override:
        raise AblationError("prompt-input proof does not bind the treatment override")
    if prompt_probe.get("comparison", {}).get("effectiveCatalogueSuppression") is not True:
        raise AblationError("prompt-input proof did not establish effective suppression")
    schema = args.schema.resolve()
    session_root = args.session_root.resolve()
    session_root.mkdir(parents=True, exist_ok=True)
    control_argv = invocation(
        args.codex, session_root, schema, args.model, args.reasoning, None
    )
    treatment_argv = invocation(
        args.codex, session_root, schema, args.model, args.reasoning,
        args.treatment_override,
    )
    if args.order == "control-treatment":
        control = run_arm("control", control_argv, packet, output)
        treatment = run_arm("treatment", treatment_argv, packet, output)
    else:
        treatment = run_arm("treatment", treatment_argv, packet, output)
        control = run_arm("control", control_argv, packet, output)
    if control["threadId"] == treatment["threadId"]:
        raise AblationError("fresh arms reused one thread identity")
    control_input = control["usage"]["input_tokens"]
    treatment_input = treatment["usage"]["input_tokens"]
    result = {
        "schema": "cultist-codex-context-ablation-pair/v1",
        "recordedAt": datetime.now(timezone.utc).isoformat(),
        "codexVersion": subprocess.run(
            [str(args.codex), "--version"], check=True, capture_output=True, text=True
        ).stdout.strip(),
        "machine": {
            "node": platform.node(),
            "system": platform.system(),
            "machine": platform.machine(),
        },
        "model": args.model,
        "reasoningEffort": args.reasoning,
        "packetSha256": packet_sha,
        "packetCharacters": len(packet),
        "outputSchemaSha256": digest(schema.read_bytes()),
        "promptInputProofSha256": digest(args.prompt_probe_result.read_bytes()),
        "treatmentOverride": args.treatment_override,
        "executionOrder": args.order,
        "control": control,
        "treatment": treatment,
        "sameFirstAction": control["firstActionId"] == treatment["firstActionId"],
        "inputTokenDelta": treatment_input - control_input,
        "inputTokenReductionPercent": round(
            (control_input - treatment_input) * 100 / control_input, 3
        ) if control_input else 0.0,
        "quietNullFirstActionResult": control["firstActionId"] == treatment["firstActionId"],
        "rawContentEmitted": False,
        "authorizesGeneralization": False,
        "authorizesCapabilityRetirement": False,
        "authorizesProductionPromotion": False,
    }
    result_bytes = canonical(result) + b"\n"
    private_write(output / "result.json", result_bytes)
    receipt = {
        "schema": "cultist-codex-context-ablation-pair-receipt/v1",
        "resultSha256": digest(result_bytes),
        "sameFirstAction": result["sameFirstAction"],
        "controlInputTokens": control_input,
        "treatmentInputTokens": treatment_input,
        "inputTokenReductionPercent": result["inputTokenReductionPercent"],
        "effectiveCatalogueSuppression": True,
        "quietNullFirstActionResult": result["quietNullFirstActionResult"],
        "rawContentEmitted": False,
    }
    private_write(output / "receipt.json", canonical(receipt) + b"\n")
    print(json.dumps(receipt, sort_keys=True))
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, json.JSONDecodeError, AblationError, subprocess.SubprocessError) as error:
        raise SystemExit(str(error)) from error
