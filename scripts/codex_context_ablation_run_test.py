#!/usr/bin/env python3

import json
import unittest

import codex_context_ablation_run as ablation


class ContextAblationRunTests(unittest.TestCase):
    def test_event_projection_retains_usage_warning_and_first_action(self):
        lines = [
            {"type": "thread.started", "thread_id": "thread-one"},
            {"type": "item.completed", "item": {"type": "error", "message": "skills context budget"}},
            {"type": "item.completed", "item": {"type": "agent_message", "text": json.dumps({"first_action_id": "inspect"})}},
            {"type": "turn.completed", "usage": {"input_tokens": 100, "cached_input_tokens": 0, "output_tokens": 5, "reasoning_output_tokens": 2}},
        ]
        parsed = ablation.parse_events("\n".join(json.dumps(item) for item in lines))
        self.assertEqual(parsed["firstActionId"], "inspect")
        self.assertEqual(parsed["usage"]["input_tokens"], 100)
        self.assertTrue(parsed["skillBudgetWarning"])

    def test_incomplete_stream_is_refused(self):
        with self.assertRaises(ablation.AblationError):
            ablation.parse_events(json.dumps({"type": "thread.started", "thread_id": "only"}))


if __name__ == "__main__":
    unittest.main()
