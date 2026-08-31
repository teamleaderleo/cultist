#!/usr/bin/env python3

import unittest

import codex_prompt_surface_matrix as matrix


class PromptSurfaceMatrixTests(unittest.TestCase):
    def test_component_presence_uses_segment_labels(self):
        result = matrix.component_presence({"segments": [
            {"label": "skills-catalogue"}, {"label": "recommended-plugins"},
        ]})
        self.assertTrue(result["skillsCatalogue"])
        self.assertTrue(result["recommendedPlugins"])
        self.assertFalse(result["appsInstructions"])

    def test_zero_threshold_comparison_reports_observation(self):
        result = matrix.comparison(100, 40)
        self.assertEqual(result["textTokenDeltaFromDefault"], -60)
        self.assertEqual(result["textTokenReductionPercentFromDefault"], 60.0)


if __name__ == "__main__":
    unittest.main()
