#!/usr/bin/env python3

import unittest

import codex_prompt_token_report as report


class CharacterEncoder:
    def encode(self, text, **kwargs):
        return list(text)


class PromptTokenReportTests(unittest.TestCase):
    def test_segments_are_labeled_and_text_is_not_retained(self):
        raw = [{"role": "developer", "content": [
            {"type": "input_text", "text": "<skills_instructions>abc"},
            {"type": "input_text", "text": "<permissions instructions>ok"},
        ]}]
        result = report.arm(raw, CharacterEncoder())
        self.assertEqual([row["label"] for row in result["segments"]], [
            "skills-catalogue", "permissions",
        ])
        self.assertNotIn("text", result["segments"][0])
        self.assertEqual(result["textTokens"], 52)

    def test_comparison_retains_exact_retired_segment(self):
        encoder = CharacterEncoder()
        control = report.arm([{"role": "developer", "content": [
            {"text": "<skills_instructions>abc"}, {"text": "keep"},
        ]}], encoder)
        treatment = report.arm([{"role": "developer", "content": [{"text": "keep"}]}], encoder)
        comparison = report.compare(control, treatment)
        self.assertEqual(comparison["textTokenDelta"], -24)
        self.assertEqual(comparison["retiredSegments"][0]["label"], "skills-catalogue")


if __name__ == "__main__":
    unittest.main()
