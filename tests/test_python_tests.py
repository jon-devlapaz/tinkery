"""Python test tiers: pull requests skip only listed slow journeys; nothing runs here."""
from pathlib import Path
import runpy
import unittest

ROOT = Path(__file__).resolve().parents[1]


class TierTests(unittest.TestCase):
    def setUp(self):
        api = runpy.run_path(str(ROOT / 'scripts/python-tests.py'), run_name='test_tiers')
        self.selected = api['selected']
        self.post_merge = api['POST_MERGE']
        self.names = lambda tier: [p.name for p in self.selected(tier)]

    def test_full_runs_every_test_file_once(self):
        files = sorted(p.name for p in (ROOT / 'tests').glob('*.py') if p.name != 'vt_screen.py')
        self.assertEqual(sorted(self.names('full')), files)

    def test_pr_skips_exactly_the_post_merge_journeys(self):
        self.assertEqual(set(self.names('full')) - set(self.names('pr')), self.post_merge)
        self.assertIn('goal_confirmation_terminal.py', self.names('pr'))
        self.assertIn('agent_operation_terminal.py', self.names('pr'))

    def test_unknown_tier_and_stale_names_are_refused(self):
        with self.assertRaises(SystemExit):
            self.selected('smoke')
        self.post_merge.add('TEST_removed_terminal.py')
        self.addCleanup(self.post_merge.discard, 'TEST_removed_terminal.py')
        with self.assertRaises(SystemExit):
            self.selected('full')


if __name__ == '__main__':
    unittest.main()
