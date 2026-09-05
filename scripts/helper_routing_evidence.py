#!/usr/bin/env python3
"""Summarize task-level routing research receipts; never dispatch or infer policy."""

import argparse
import copy
import json
import math
from pathlib import Path
import sys


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


def validate_task(task, seen=None):
    fields(task, ("task_ref", "evidence_refs", "route", "classification_timing",
                  "classification", "outcomes", "metrics", "provider_usage", "limitations"), "task")
    require(text(task["task_ref"]), "duplicate or empty task_ref")
    if seen is not None:
        require(task["task_ref"] not in seen, "duplicate or empty task_ref")
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


def validate(receipt):
    fields(receipt, ("schema", "tasks"), "receipt")
    require(receipt["schema"] == SCHEMA, "unsupported schema")
    require(isinstance(receipt["tasks"], list) and receipt["tasks"], "tasks must be nonempty")
    seen = set()
    for task in receipt["tasks"]:
        validate_task(task, seen)


def reconcile_tasks(tasks):
    """Reconcile multiple task observations by exact task_ref, failing closed on conflict."""
    by_ref = {}
    for task in tasks:
        validate_task(task)
        ref = task["task_ref"]
        if ref not in by_ref:
            by_ref[ref] = copy.deepcopy(task)
            continue
        existing = by_ref[ref]

        if existing["route"] is None:
            existing["route"] = task["route"]
        elif task["route"] is not None and existing["route"] != task["route"]:
            raise ValueError(f"conflicting route for task_ref '{ref}': {existing['route']} vs {task['route']}")

        if existing["classification_timing"] is None:
            existing["classification_timing"] = task["classification_timing"]
        elif task["classification_timing"] is not None and existing["classification_timing"] != task["classification_timing"]:
            raise ValueError(f"conflicting classification_timing for task_ref '{ref}': {existing['classification_timing']} vs {task['classification_timing']}")

        for k in CLASSES:
            if existing["classification"][k] is None:
                existing["classification"][k] = task["classification"][k]
            elif task["classification"][k] is not None and existing["classification"][k] != task["classification"][k]:
                raise ValueError(f"conflicting classification '{k}' for task_ref '{ref}': {existing['classification'][k]} vs {task['classification'][k]}")

        for k in OUTCOMES:
            if existing["outcomes"][k] is None:
                existing["outcomes"][k] = task["outcomes"][k]
            elif task["outcomes"][k] is not None and existing["outcomes"][k] != task["outcomes"][k]:
                raise ValueError(f"conflicting outcome '{k}' for task_ref '{ref}': {existing['outcomes'][k]} vs {task['outcomes'][k]}")

        for k in METRICS:
            if existing["metrics"][k] is None:
                existing["metrics"][k] = task["metrics"][k]
            elif task["metrics"][k] is not None and existing["metrics"][k] != task["metrics"][k]:
                raise ValueError(f"conflicting metric '{k}' for task_ref '{ref}': {existing['metrics'][k]} vs {task['metrics'][k]}")

        for r in task["evidence_refs"]:
            if r not in existing["evidence_refs"]:
                existing["evidence_refs"].append(r)

        for lim in task["limitations"]:
            if lim not in existing["limitations"]:
                existing["limitations"].append(lim)

        existing_identities = {
            tuple(u[k] for k in ("provider", "metric", "unit", "scope")): u
            for u in existing["provider_usage"]
        }
        for u in task["provider_usage"]:
            ident = tuple(u[k] for k in ("provider", "metric", "unit", "scope"))
            if ident in existing_identities:
                prev = existing_identities[ident]
                if prev["value"] != u["value"]:
                    raise ValueError(f"conflicting provider_usage value for {ident} in task_ref '{ref}'")
                if prev["evidence_ref"] != u["evidence_ref"]:
                    raise ValueError(f"conflicting provider_usage evidence_ref for {ident} in task_ref '{ref}'")
            else:
                existing["provider_usage"].append(copy.deepcopy(u))
                existing_identities[ident] = u

    return list(by_ref.values())


def merge_receipts(receipts, reconcile=False):
    """Merge multiple receipt dictionaries into one."""
    require(isinstance(receipts, list) and receipts, "receipts must be nonempty list")
    for r in receipts:
        require(isinstance(r, dict) and r.get("schema") == SCHEMA, "unsupported schema in receipt")
    all_tasks = [task for r in receipts for task in r.get("tasks", [])]
    tasks = reconcile_tasks(all_tasks) if reconcile else all_tasks
    receipt = {"schema": SCHEMA, "tasks": tasks}
    validate(receipt)
    return receipt


def compare_cohorts(groups):
    """Analyze group efficiency, matched comparisons, and negative controls."""
    group_analyses = []
    by_classification = {}

    for g in groups:
        tasks_count = g["tasks"]
        accepted_true = g["outcomes"]["accepted"]["true"]
        verified_true = g["outcomes"]["verified"]["true"]

        metrics_per_accepted = {}
        for k in METRICS:
            m = g["metrics"][k]
            if accepted_true > 0 and m["known_total"] is not None:
                metrics_per_accepted[k] = {
                    "known_total_per_accepted": round(m["known_total"] / accepted_true, 4),
                    "known_tasks": m["known_tasks"],
                    "accepted_tasks": accepted_true,
                }
            else:
                metrics_per_accepted[k] = None

        analysis = {
            "route": g["route"],
            "classification_timing": g["classification_timing"],
            "classification": g["classification"],
            "tasks": tasks_count,
            "acceptance_rate": round(accepted_true / tasks_count, 4) if tasks_count > 0 else None,
            "verification_rate": round(verified_true / tasks_count, 4) if tasks_count > 0 else None,
            "outcomes": g["outcomes"],
            "metrics_per_accepted_task": metrics_per_accepted,
        }
        group_analyses.append(analysis)

        class_key = tuple(g["classification"][k] for k in sorted(CLASSES))
        by_classification.setdefault(class_key, []).append(analysis)

    matched_pairs = []
    unmatched_cohorts = []
    for _, cohort_groups in by_classification.items():
        routes = {cg["route"] for cg in cohort_groups}
        if len(cohort_groups) > 1 and len(routes) > 1:
            matched_pairs.append({
                "classification": cohort_groups[0]["classification"],
                "cohort_groups": cohort_groups,
            })
        else:
            unmatched_cohorts.append({
                "classification": cohort_groups[0]["classification"],
                "routes": list(routes),
                "tasks": sum(cg["tasks"] for cg in cohort_groups),
                "limitation": "Single arm or unrouted only; no matched counter-route under this difficulty class."
            })

    return {
        "groups": group_analyses,
        "matched_comparisons": matched_pairs,
        "unmatched_cohorts": unmatched_cohorts,
    }


def summarize(receipt, include_comparisons=False):
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
    result = {
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
    if include_comparisons:
        result["comparisons"] = compare_cohorts(summaries)
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("receipts", nargs="*", default=["-"],
                        help="path(s) to receipt JSON file(s) or '-' for stdin")
    parser.add_argument("--reconcile", action="store_true",
                        help="reconcile/merge multiple task observations by exact task_ref")
    parser.add_argument("--compare", action="store_true",
                        help="include matched-cohort comparison and efficiency metrics")
    args = parser.parse_args()
    receipt_objects = []
    try:
        for r_path in args.receipts:
            if r_path == "-":
                raw = sys.stdin.read()
                if not raw.strip():
                    parser.exit(2, "empty input from stdin\n")
                receipt_objects.append(json.loads(raw))
            else:
                p = Path(r_path)
                receipt_objects.append(json.loads(p.read_text()))
        if len(receipt_objects) == 1 and not args.reconcile:
            merged = receipt_objects[0]
            validate(merged)
        else:
            merged = merge_receipts(receipt_objects, reconcile=args.reconcile)
        result = summarize(merged, include_comparisons=args.compare)
    except (ValueError, TypeError, OSError, json.JSONDecodeError) as error:
        parser.exit(2, f"invalid receipt: {error}\n")
    print(json.dumps(result, indent=2, allow_nan=False))


if __name__ == "__main__":
    main()
