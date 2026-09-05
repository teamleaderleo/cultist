#!/usr/bin/env python3

import copy
import json
from pathlib import Path
import unittest

import helper_routing_evidence as evidence


RECEIPT = Path(__file__).resolve().parents[1] / "research/helper-routing/2026-09-05-muse-launcher-guard/result.json"


class HelperRoutingEvidenceTests(unittest.TestCase):
    def setUp(self):
        self.receipt = json.loads(RECEIPT.read_text())
        self.task = self.receipt["tasks"][0]

    def test_real_receipt_does_not_invent_comparison_or_free_end_to_end_work(self):
        result = evidence.summarize(self.receipt)
        group = result["groups"][0]
        self.assertIsNone(result["routing_conclusion"])
        self.assertIsNone(group["route"])
        self.assertEqual(group["classification_timing"], "retrospective")
        self.assertEqual(group["outcomes"]["accepted"], {"true": 1, "false": 0, "unknown": 0})
        self.assertEqual(group["outcomes"]["process_completed"]["unknown"], 1)
        self.assertEqual(group["metrics"]["retries"], {"known_tasks": 0, "unknown_tasks": 1, "known_total": None})
        self.assertIsNone(result["provider_usage"][0]["unit"])
        self.assertEqual(result["source_tasks"], self.receipt["tasks"])

    def test_provider_success_does_not_override_failed_process_or_rejected_work(self):
        self.task["outcomes"] = {"provider_success": True, "process_completed": False,
                                 "verified": False, "accepted": False}
        outcomes = evidence.summarize(self.receipt)["groups"][0]["outcomes"]
        self.assertEqual(outcomes["provider_success"]["true"], 1)
        self.assertEqual(outcomes["process_completed"]["false"], 1)
        self.assertEqual(outcomes["accepted"]["true"], 0)

    def test_unknown_metrics_do_not_become_zero_or_enter_known_denominator(self):
        other = copy.deepcopy(self.task)
        other["task_ref"] = "second-task"
        other["metrics"]["retries"] = 0
        other["metrics"]["repair_minutes"] = 7.5
        other["outcomes"]["accepted"] = None
        other["provider_usage"] = []
        self.receipt["tasks"].append(other)
        result = evidence.summarize(self.receipt)
        group = result["groups"][0]
        self.assertEqual(group["metrics"]["repair_minutes"], {"known_tasks": 1, "unknown_tasks": 1, "known_total": 7.5})
        self.assertEqual(group["outcomes"]["accepted"]["unknown"], 1)
        self.assertEqual(result["tasks_without_provider_usage"], 1)

    def test_difficulty_timing_and_route_are_not_pooled(self):
        for key, value in (("route", "cheap-first"), ("classification_timing", "before-dispatch"),
                           ("oracle_strength", "weak")):
            with self.subTest(key=key):
                receipt = copy.deepcopy(self.receipt)
                other = copy.deepcopy(self.task)
                other["task_ref"] = "other"
                (other["classification"] if key == "oracle_strength" else other)[key] = value
                receipt["tasks"].append(other)
                self.assertEqual(len(evidence.summarize(receipt)["groups"]), 2)

    def test_partial_usage_and_different_units_remain_separate(self):
        other = copy.deepcopy(self.task["provider_usage"][0])
        other.update(provider="reviewer", unit="USD", value=1.5, scope="review only")
        self.task["provider_usage"].append(other)
        usage = evidence.summarize(self.receipt)["provider_usage"]
        self.assertEqual([row["unit"] for row in usage], [None, "USD"])
        self.assertEqual([row["value"] for row in usage], [0, 1.5])

    def test_duplicate_task_and_usage_are_rejected(self):
        self.receipt["tasks"].append(copy.deepcopy(self.task))
        with self.assertRaisesRegex(ValueError, "duplicate"):
            evidence.summarize(self.receipt)
        self.receipt["tasks"].pop()
        self.task["provider_usage"].append(copy.deepcopy(self.task["provider_usage"][0]))
        with self.assertRaisesRegex(ValueError, "duplicate"):
            evidence.summarize(self.receipt)

    def test_malformed_observations_fail_closed(self):
        mutations = [
            lambda t: t["metrics"].update(retries=True),
            lambda t: t["metrics"].update(retries=1.5),
            lambda t: t["metrics"].update(wall_seconds=-1),
            lambda t: t["metrics"].update(repair_minutes=float("nan")),
            lambda t: t["provider_usage"][0].update(value=float("inf")),
            lambda t: t["outcomes"].update(accepted="true"),
            lambda t: t["classification"].update(oracle_strength="excellent"),
            lambda t: t.update(evidence_refs=[]),
            lambda t: t.update(extra="ignored?"),
        ]
        for mutation in mutations:
            with self.subTest(mutation=mutation):
                receipt = copy.deepcopy(self.receipt)
                mutation(receipt["tasks"][0])
                with self.assertRaises(ValueError):
                    evidence.summarize(receipt)

    def test_reconcile_tasks_combines_complementary_evidence(self):
        # Base execution receipt (like Stensibly projection)
        exec_task = {
            "task_ref": "https://github.com/teamleaderleo/compute-node-bootstrap/pull/35",
            "evidence_refs": ["stensibly-command:sha256:abc"],
            "route": None,
            "classification_timing": None,
            "classification": {"oracle_strength": None, "semantic_ambiguity": None, "coupling": None, "failure_cost": None},
            "outcomes": {"provider_success": None, "process_completed": None, "verified": True, "accepted": None},
            "metrics": {"retries": None, "repair_minutes": None, "wall_seconds": None, "task_tokens": None},
            "provider_usage": [],
            "limitations": ["OBSERVED: test verification passed."],
        }
        # Review outcome annotation
        review_task = copy.deepcopy(self.task)
        review_task["outcomes"]["verified"] = None  # review didn't independently re-verify, but verified is in exec_task
        review_task["route"] = "cheap-first"
        review_task["classification_timing"] = "before-dispatch"

        reconciled = evidence.reconcile_tasks([exec_task, review_task])
        self.assertEqual(len(reconciled), 1)
        r = reconciled[0]
        self.assertEqual(r["route"], "cheap-first")
        self.assertEqual(r["classification_timing"], "before-dispatch")
        self.assertEqual(r["classification"]["oracle_strength"], "strong")
        self.assertEqual(r["outcomes"]["verified"], True)
        self.assertEqual(r["outcomes"]["accepted"], True)
        self.assertIn("stensibly-command:sha256:abc", r["evidence_refs"])
        self.assertEqual(len(r["provider_usage"]), 1)

    def test_reconcile_tasks_fails_on_conflicting_values(self):
        for field, bad_val in (
            ("route", "frontier-first"),
            ("classification_timing", "before-dispatch"),
        ):
            with self.subTest(field=field):
                t1 = copy.deepcopy(self.task)
                t1["route"] = "cheap-first"
                t1["classification_timing"] = "retrospective"
                t2 = copy.deepcopy(self.task)
                t2[field] = bad_val
                with self.assertRaisesRegex(ValueError, "conflicting"):
                    evidence.reconcile_tasks([t1, t2])

        # Conflicting classification
        t1 = copy.deepcopy(self.task)
        t2 = copy.deepcopy(self.task)
        t2["classification"]["oracle_strength"] = "weak"
        with self.assertRaisesRegex(ValueError, "conflicting classification"):
            evidence.reconcile_tasks([t1, t2])

        # Conflicting outcomes
        t1 = copy.deepcopy(self.task)
        t2 = copy.deepcopy(self.task)
        t2["outcomes"]["accepted"] = False
        with self.assertRaisesRegex(ValueError, "conflicting outcome"):
            evidence.reconcile_tasks([t1, t2])

    def test_merge_receipts_combines_distinct_receipts_and_reconciles(self):
        r1 = copy.deepcopy(self.receipt)
        r2 = copy.deepcopy(self.receipt)
        r2["tasks"][0]["task_ref"] = "second-task"
        merged = evidence.merge_receipts([r1, r2])
        self.assertEqual(len(merged["tasks"]), 2)

        # Merging with duplicate task_ref without reconcile fails
        with self.assertRaisesRegex(ValueError, "duplicate"):
            evidence.merge_receipts([r1, r1], reconcile=False)

        # Merging with duplicate task_ref with reconcile succeeds
        reconciled_receipt = evidence.merge_receipts([r1, r1], reconcile=True)
        self.assertEqual(len(reconciled_receipt["tasks"]), 1)

    def test_compare_cohorts_evaluates_matched_and_unmatched_arms(self):
        # Create cheap-first task and frontier-first task with same classification
        cheap_task = copy.deepcopy(self.task)
        cheap_task["route"] = "cheap-first"
        cheap_task["metrics"]["repair_minutes"] = 10.0
        cheap_task["metrics"]["retries"] = 1

        frontier_task = copy.deepcopy(self.task)
        frontier_task["task_ref"] = "frontier-task"
        frontier_task["route"] = "frontier-first"
        frontier_task["metrics"]["repair_minutes"] = 2.0
        frontier_task["metrics"]["retries"] = 0

        receipt = {"schema": evidence.SCHEMA, "tasks": [cheap_task, frontier_task]}
        result = evidence.summarize(receipt, include_comparisons=True)
        comps = result["comparisons"]

        self.assertEqual(len(comps["matched_comparisons"]), 1)
        matched = comps["matched_comparisons"][0]
        self.assertEqual(matched["classification"]["oracle_strength"], "strong")
        self.assertEqual(len(matched["cohort_groups"]), 2)

        # Check metrics per accepted task
        cheap_group = [g for g in comps["groups"] if g["route"] == "cheap-first"][0]
        self.assertEqual(cheap_group["acceptance_rate"], 1.0)
        self.assertEqual(cheap_group["metrics_per_accepted_task"]["repair_minutes"]["known_total_per_accepted"], 10.0)
        self.assertEqual(cheap_group["metrics_per_accepted_task"]["retries"]["known_total_per_accepted"], 1.0)

    def test_negative_control_records_zero_acceptance_and_verification_failure(self):
        defect_task = copy.deepcopy(self.task)
        defect_task["task_ref"] = "defect-task"
        defect_task["outcomes"] = {
            "provider_success": True,
            "process_completed": False,
            "verified": False,
            "accepted": False,
        }
        receipt = {"schema": evidence.SCHEMA, "tasks": [defect_task]}
        result = evidence.summarize(receipt, include_comparisons=True)
        group = result["comparisons"]["groups"][0]
        self.assertEqual(group["acceptance_rate"], 0.0)
        self.assertEqual(group["verification_rate"], 0.0)
        self.assertIsNone(group["metrics_per_accepted_task"]["repair_minutes"])

    def test_unknown_only_outcomes_yield_null_rates_not_zero(self):
        unknown_task = copy.deepcopy(self.task)
        unknown_task["task_ref"] = "unknown-task"
        unknown_task["outcomes"] = {
            "provider_success": None,
            "process_completed": None,
            "verified": None,
            "accepted": None,
        }
        receipt = {"schema": evidence.SCHEMA, "tasks": [unknown_task]}
        result = evidence.summarize(receipt, include_comparisons=True)
        group = result["comparisons"]["groups"][0]
        self.assertIsNone(group["acceptance_rate"])
        self.assertIsNone(group["verification_rate"])

    def test_mixed_known_unknown_rates_use_known_denominator(self):
        known_task = copy.deepcopy(self.task)
        known_task["task_ref"] = "known-task"
        known_task["outcomes"] = {
            "provider_success": None,
            "process_completed": None,
            "verified": True,
            "accepted": True,
        }
        unknown_task = copy.deepcopy(self.task)
        unknown_task["task_ref"] = "unknown-task-2"
        unknown_task["outcomes"] = {
            "provider_success": None,
            "process_completed": None,
            "verified": None,
            "accepted": None,
        }
        # Same route/timing/classification so both land in one group.
        receipt = {"schema": evidence.SCHEMA, "tasks": [known_task, unknown_task]}
        result = evidence.summarize(receipt, include_comparisons=True)
        group = result["comparisons"]["groups"][0]
        self.assertEqual(group["tasks"], 2)
        self.assertEqual(group["acceptance_rate"], 1.0)
        self.assertEqual(group["verification_rate"], 1.0)

    def test_timing_mismatch_is_not_a_matched_comparison(self):
        cheap_task = copy.deepcopy(self.task)
        cheap_task["route"] = "cheap-first"
        cheap_task["classification_timing"] = "retrospective"
        frontier_task = copy.deepcopy(self.task)
        frontier_task["task_ref"] = "frontier-task"
        frontier_task["route"] = "frontier-first"
        frontier_task["classification_timing"] = "before-dispatch"
        receipt = {"schema": evidence.SCHEMA, "tasks": [cheap_task, frontier_task]}
        result = evidence.summarize(receipt, include_comparisons=True)
        comps = result["comparisons"]
        self.assertEqual(len(comps["matched_comparisons"]), 0)
        self.assertEqual(len(comps["unmatched_cohorts"]), 2)

    def test_cli_execution_via_stdin_and_file(self):
        import subprocess
        script = Path(__file__).resolve().parents[1] / "scripts/helper_routing_evidence.py"
        # Test positional file
        out = subprocess.check_output(["python3", str(script), str(RECEIPT)], text=True)
        data = json.loads(out)
        self.assertEqual(data["task_count"], 1)

        # Test stdin with '-'
        out_stdin = subprocess.check_output(["python3", str(script), "-"], input=RECEIPT.read_text(), text=True)
        self.assertEqual(json.loads(out_stdin)["task_count"], 1)

        # Test stdin default
        out_default = subprocess.check_output(["python3", str(script)], input=RECEIPT.read_text(), text=True)
        self.assertEqual(json.loads(out_default)["task_count"], 1)

        # Test with --compare
        out_comp = subprocess.check_output(["python3", str(script), str(RECEIPT), "--compare"], text=True)
        self.assertIn("comparisons", json.loads(out_comp))


if __name__ == "__main__":
    unittest.main()
