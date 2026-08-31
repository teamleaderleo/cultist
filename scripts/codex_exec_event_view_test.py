#!/usr/bin/env python3

import json
import unittest

import codex_exec_event_view as view


def line(value):
    return json.dumps(value, separators=(",", ":")).encode() + b"\n"


class CodexExecEventViewTests(unittest.TestCase):
    def fixture(self):
        return b"warning: private path\n" + b"".join([
            line({
                "type": "item.completed",
                "item": {
                    "type": "command_execution",
                    "status": "completed",
                    "command": "secret-command",
                    "aggregated_output": "huge private output",
                },
            }),
            line({
                "type": "item.completed",
                "item": {"type": "agent_message", "text": "secret-intermediate"},
            }),
            line({
                "type": "item.completed",
                "item": {"type": "agent_message", "text": '{"answer":"ok"}'},
            }),
            line({
                "type": "turn.completed",
                "usage": {"input_tokens": 123, "output_tokens": 7},
            }),
        ])

    def test_default_is_content_free(self):
        result = view.project(self.fixture(), include_final=False, max_final_chars=80)
        rendered = json.dumps(result)
        self.assertNotIn("secret-command", rendered)
        self.assertNotIn("private output", rendered)
        self.assertNotIn("secret-intermediate", rendered)
        self.assertNotIn('"answer": "ok"', rendered)
        self.assertEqual(result["commands"]["completed"], 1)
        self.assertEqual(result["commands"]["aggregatedOutputBytesOmitted"], 19)
        self.assertEqual(result["parse"]["nonJsonLines"], 1)
        self.assertEqual(result["usage"]["input_tokens"], 123)
        self.assertFalse(result["rawContentEmitted"])

    def test_explicit_final_preserves_structured_result_only(self):
        result = view.project(self.fixture(), include_final=True, max_final_chars=80)
        self.assertEqual(result["finalStructured"], {"answer": "ok"})
        self.assertFalse(result["finalTruncated"])
        rendered = json.dumps(result)
        self.assertNotIn("secret-command", rendered)
        self.assertNotIn("private output", rendered)
        self.assertNotIn("secret-intermediate", rendered)
        self.assertTrue(result["rawContentEmitted"])
        self.assertIn("intermediate-agent-messages", result["omittedClasses"])

    def test_unstructured_final_is_bounded(self):
        raw = line({
            "type": "item.completed",
            "item": {"type": "agent_message", "text": "abcdefgh"},
        })
        result = view.project(raw, include_final=True, max_final_chars=4)
        self.assertEqual(result["finalText"], "abcd")
        self.assertTrue(result["finalTruncated"])

    def test_oversized_structured_final_does_not_bypass_bound(self):
        raw = line({
            "type": "item.completed",
            "item": {"type": "agent_message", "text": '{"answer":"abcdefgh"}'},
        })
        result = view.project(raw, include_final=True, max_final_chars=8)
        self.assertNotIn("finalStructured", result)
        self.assertEqual(result["finalText"], '{"answer')
        self.assertTrue(result["finalTruncated"])

    def test_json_shaped_noise_is_counted_as_malformed(self):
        result = view.project(b"{broken}\nplain\n", include_final=False, max_final_chars=80)
        self.assertEqual(result["parse"]["nonJsonLines"], 2)
        self.assertEqual(result["parse"]["malformedJsonLines"], 1)


if __name__ == "__main__":
    unittest.main()
