#!/usr/bin/env python3

import unittest

import codex_prompt_input_probe as probe


class PromptInputProbeTests(unittest.TestCase):
    def test_shape_and_comparison_detect_exact_skill_catalogue_retirement(self):
        skill = {"role": "developer", "content": "<skills_instructions>Available skills</skills_instructions>"}
        task = {"role": "user", "content": "bounded task"}
        control = probe.shape([skill, task])
        treatment = probe.shape([task])
        comparison = probe.compare(control, treatment)
        self.assertTrue(comparison["effectiveCatalogueSuppression"])
        self.assertEqual(comparison["removedSkillMarkerItems"], 1)
        self.assertLess(comparison["serializedByteDelta"], 0)

    def test_quiet_null_change_is_retained(self):
        task = {"role": "user", "content": "bounded task"}
        control = probe.shape([task])
        comparison = probe.compare(control, probe.shape([task]))
        self.assertFalse(comparison["effectiveCatalogueSuppression"])
        self.assertEqual(comparison["removedItems"], [])
        self.assertEqual(comparison["serializedByteDelta"], 0)

    def test_volatile_message_ids_do_not_create_false_semantic_changes(self):
        control = probe.shape([{"id": "one", "role": "user", "content": "same"}])
        treatment = probe.shape([{"id": "two", "role": "user", "content": "same"}])
        comparison = probe.compare(control, treatment)
        self.assertEqual(comparison["unchangedItems"], 1)
        self.assertEqual(comparison["removedItems"], [])


if __name__ == "__main__":
    unittest.main()
