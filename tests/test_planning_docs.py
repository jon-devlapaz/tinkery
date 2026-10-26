from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]


class PlanningDocumentationTests(unittest.TestCase):
    def test_release_index_points_to_issue_authority_and_permission_rules(self):
        source = (ROOT / "docs/design/first-release-issues.md").read_text()
        self.assertIn("The linked issues own scope, status, approval", source)
        self.assertIn("Publication does not accept a design or authorize implementation", source)
        self.assertIn("[work intake](../../AGENTS.md#work-intake)", source)

    def test_work_intake_keeps_approval_separate_from_publication(self):
        source = (ROOT / "AGENTS.md").read_text()
        self.assertIn("Unapproved work stays `needs-approval`.", source)
        self.assertIn("Publication, design", source)
        self.assertIn("acceptance, implementation, data use and budget are separate decisions.",
                      source)


if __name__ == "__main__":
    unittest.main()
