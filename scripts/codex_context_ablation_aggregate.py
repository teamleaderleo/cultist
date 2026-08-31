#!/usr/bin/env python3
"""Aggregate matched Codex context-ablation pairs without dropping nulls."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import statistics
from datetime import datetime, timezone
from pathlib import Path
from typing import Any


class AggregateError(RuntimeError):
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


def aggregate(results: list[dict[str, Any]]) -> dict[str, Any]:
    if len(results) < 2:
        raise AggregateError("at least two matched pairs are required")
    identity_keys = (
        "codexVersion", "model", "reasoningEffort", "packetSha256",
        "outputSchemaSha256", "treatmentOverride",
    )
    identity = {key: results[0].get(key) for key in identity_keys}
    for result in results:
        if result.get("schema") != "cultist-codex-context-ablation-pair/v1":
            raise AggregateError("pair schema is unsupported")
        if any(result.get(key) != value for key, value in identity.items()):
            raise AggregateError("pair identity drifted")
    orders = [result.get("executionOrder") or "unrecorded" for result in results]
    thread_ids = [
        result[arm]["threadId"]
        for result in results
        for arm in ("control", "treatment")
    ]
    if len(thread_ids) != len(set(thread_ids)):
        raise AggregateError("a fresh thread identity was reused")
    rows = []
    for index, result in enumerate(results, start=1):
        control = result["control"]["usage"]["input_tokens"]
        treatment = result["treatment"]["usage"]["input_tokens"]
        rows.append({
            "pair": index,
            "order": orders[index - 1],
            "controlInputTokens": control,
            "treatmentInputTokens": treatment,
            "inputTokenDelta": treatment - control,
            "inputTokenReductionPercent": result["inputTokenReductionPercent"],
            "sameFirstAction": result["sameFirstAction"],
            "quietNullFirstActionResult": result["quietNullFirstActionResult"],
        })
    deltas = [row["inputTokenDelta"] for row in rows]
    reductions = [row["inputTokenReductionPercent"] for row in rows]
    return {
        "schema": "cultist-codex-context-ablation-aggregate/v1",
        "recordedAt": datetime.now(timezone.utc).isoformat(),
        **identity,
        "pairCount": len(rows),
        "executionOrders": orders,
        "pairs": rows,
        "allFirstActionsPreserved": all(row["sameFirstAction"] for row in rows),
        "quietNullFirstActionResults": sum(row["quietNullFirstActionResult"] for row in rows),
        "meanInputTokenDelta": statistics.mean(deltas),
        "medianInputTokenDelta": statistics.median(deltas),
        "exactDeltaStableAcrossPairs": len(set(deltas)) == 1,
        "meanInputTokenReductionPercent": round(statistics.mean(reductions), 3),
        "medianInputTokenReductionPercent": round(statistics.median(reductions), 3),
        "rawContentEmitted": False,
        "authorizesGeneralization": False,
        "authorizesCapabilityRetirement": False,
        "authorizesProductionPromotion": False,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("result", nargs="+", type=Path)
    args = parser.parse_args()
    try:
        results = [json.loads(path.read_text(encoding="utf-8")) for path in args.result]
        aggregate_result = aggregate(results)
        result_bytes = canonical(aggregate_result) + b"\n"
        args.output.parent.mkdir(parents=True, exist_ok=True)
        private_write(args.output, result_bytes)
    except (OSError, json.JSONDecodeError, AggregateError) as error:
        raise SystemExit(str(error)) from error
    print(json.dumps({
        "schema": "cultist-codex-context-ablation-aggregate-receipt/v1",
        "resultSha256": digest(result_bytes),
        "pairCount": aggregate_result["pairCount"],
        "allFirstActionsPreserved": aggregate_result["allFirstActionsPreserved"],
        "meanInputTokenDelta": aggregate_result["meanInputTokenDelta"],
        "meanInputTokenReductionPercent": aggregate_result["meanInputTokenReductionPercent"],
        "exactDeltaStableAcrossPairs": aggregate_result["exactDeltaStableAcrossPairs"],
        "rawContentEmitted": False,
    }, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
