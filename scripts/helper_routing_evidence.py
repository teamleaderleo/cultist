#!/usr/bin/env python3
"""Summarize task-level routing research receipts; never dispatch or infer policy."""

import argparse
import json
import math
from pathlib import Path


SCHEMA = "cultist-helper-routing-evidence/v1"
CLASSES = {
    "oracle_strength": {"strong", "moderate", "weak"},
    "semantic_ambiguity": {"low", "medium", "high"},
    "coupling": {"local", "subsystem", "cross_system"},
    "failure_cost": {"low", "medium", "high"},
}
OUTCOMES = ("provider_success", "process_completed", "verified", "accepted")
METRICS = ("retries", "repair_minutes", "wall_seconds", "task_tokens")


def require(condition, message):
    if not condition:
        raise ValueError(message)


def fields(value, names, label):
    require(isinstance(value, dict) and set(value) == set(names), f"invalid {label} fields")


def text(value):
    return isinstance(value, str) and bool(value.strip())


def number(value, integral=False):
    return value is None or (
        type(value) in (int, float) and math.isfinite(value) and value >= 0
        and (not integral or type(value) is int)
    )


def validate(receipt):
    fields(receipt, ("schema", "tasks"), "receipt")
    require(receipt["schema"] == SCHEMA, "unsupported schema")
    require(isinstance(receipt["tasks"], list) and receipt["tasks"], "tasks must be nonempty")
    seen = set()
    for task in receipt["tasks"]:
        fields(task, ("task_ref", "evidence_refs", "route", "classification_timing",
                      "classification", "outcomes", "metrics", "provider_usage", "limitations"), "task")
        require(text(task["task_ref"]) and task["task_ref"] not in seen, "duplicate or empty task_ref")
        seen.add(task["task_ref"])
        require(isinstance(task["evidence_refs"], list) and task["evidence_refs"]
                and all(text(ref) for ref in task["evidence_refs"]), "evidence_refs required")
        require(task["route"] in (None, "cheap-first", "frontier-first"), "invalid route")
        require(task["classification_timing"] in (None, "before-dispatch", "retrospective"),
                "invalid classification_timing")
        fields(task["classification"], CLASSES, "classification")
        for key, allowed in CLASSES.items():
            require(task["classification"][key] is None or task["classification"][key] in allowed,
                    f"invalid {key}")
        fields(task["outcomes"], OUTCOMES, "outcomes")
        for key in OUTCOMES:
            require(task["outcomes"][key] is None or type(task["outcomes"][key]) is bool,
                    f"invalid {key}")
        fields(task["metrics"], METRICS, "metrics")
        for key in METRICS:
            require(number(task["metrics"][key], key in ("retries", "task_tokens")), f"invalid {key}")
        require(isinstance(task["limitations"], list) and all(text(s) for s in task["limitations"]),
                "invalid limitations")
        require(isinstance(task["provider_usage"], list), "invalid provider_usage")
        usage_seen = set()
        for usage in task["provider_usage"]:
            fields(usage, ("provider", "metric", "unit", "scope", "value", "evidence_ref"), "usage")
            require(all(text(usage[k]) for k in ("provider", "metric", "scope", "evidence_ref")),
                    "usage identity and evidence required")
            require(usage["unit"] is None or text(usage["unit"]), "invalid usage unit")
            require(number(usage["value"]), "invalid usage value")
            identity = tuple(usage[k] for k in ("provider", "metric", "unit", "scope"))
            require(identity not in usage_seen, "duplicate task usage metric")
            usage_seen.add(identity)


def summarize(receipt):
    validate(receipt)
    groups = {}
    for task in receipt["tasks"]:
        # Never pool tasks with different verification difficulty or retrospective tags.
        identity = (task["route"], task["classification_timing"],
                    *(task["classification"][key] for key in CLASSES))
        groups.setdefault(identity, []).append(task)
    summaries = []
    for tasks in groups.values():
        first = tasks[0]
        metrics = {}
        for key in METRICS:
            known = [task["metrics"][key] for task in tasks if task["metrics"][key] is not None]
            metrics[key] = {"known_tasks": len(known), "unknown_tasks": len(tasks) - len(known),
                            "known_total": sum(known) if known else None}
        outcomes = {key: {
            "true": sum(task["outcomes"][key] is True for task in tasks),
            "false": sum(task["outcomes"][key] is False for task in tasks),
            "unknown": sum(task["outcomes"][key] is None for task in tasks),
        } for key in OUTCOMES}
        summaries.append({"route": first["route"], "classification_timing": first["classification_timing"],
                          "classification": first["classification"], "tasks": len(tasks),
                          "outcomes": outcomes, "metrics": metrics})
    return {
        "schema": SCHEMA,
        "task_count": len(receipt["tasks"]),
        "groups": summaries,
        # Preserve units, scope, provenance and missingness; no currency conversion or
        # cost-per-accepted-task estimate from partial helper-only accounting.
        "provider_usage": [{"task_ref": task["task_ref"], **usage}
                           for task in receipt["tasks"] for usage in task["provider_usage"]],
        "tasks_without_provider_usage": sum(not task["provider_usage"] for task in receipt["tasks"]),
        "source_tasks": receipt["tasks"],
        "routing_conclusion": None,
        "limitation": "Descriptive evidence only; groups are not matched experimental arms. "
                      "Unknown values are not zero. Provider usage may omit review and repair work.",
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("receipt", type=Path)
    args = parser.parse_args()
    try:
        result = summarize(json.loads(args.receipt.read_text()))
    except (ValueError, TypeError, OSError) as error:
        parser.exit(2, f"invalid receipt: {error}\n")
    print(json.dumps(result, indent=2, allow_nan=False))


if __name__ == "__main__":
    main()
