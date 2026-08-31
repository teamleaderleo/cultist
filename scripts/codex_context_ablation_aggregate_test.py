#!/usr/bin/env python3

import copy
import unittest

import codex_context_ablation_aggregate as aggregate


def pair(order: str, suffix: str):
    return {
        "schema": "cultist-codex-context-ablation-pair/v1",
        "codexVersion": "codex-cli test",
        "model": "model",
        "reasoningEffort": "max",
        "packetSha256": "a" * 64,
        "outputSchemaSha256": "b" * 64,
        "treatmentOverride": "skills.include_instructions=false",
        "executionOrder": order,
        "control": {"threadId": f"control-{suffix}", "usage": {"input_tokens": 1000}},
        "treatment": {"threadId": f"treatment-{suffix}", "usage": {"input_tokens": 900}},
        "inputTokenReductionPercent": 10.0,
        "sameFirstAction": True,
        "quietNullFirstActionResult": True,
    }


class ContextAblationAggregateTests(unittest.TestCase):
    def test_nulls_and_unrecorded_order_are_retained(self):
        first = pair("control-treatment", "a")
        first.pop("executionOrder")
        result = aggregate.aggregate([
            first, pair("treatment-control", "b")
        ])
        self.assertEqual(result["quietNullFirstActionResults"], 2)
        self.assertTrue(result["exactDeltaStableAcrossPairs"])
        self.assertEqual(result["meanInputTokenDelta"], -100)
        self.assertEqual(result["executionOrders"], ["unrecorded", "treatment-control"])

    def test_identity_drift_is_refused(self):
        changed = pair("treatment-control", "b")
        changed["model"] = "other"
        with self.assertRaises(aggregate.AggregateError):
            aggregate.aggregate([pair("control-treatment", "a"), changed])


if __name__ == "__main__":
    unittest.main()
