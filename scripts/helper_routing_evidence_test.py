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


if __name__ == "__main__":
    unittest.main()
