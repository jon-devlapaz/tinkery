"""Worker test fixtures keep their data and processes isolated."""
from pathlib import Path
import tempfile
import unittest
from unittest.mock import call, patch

import session_fixture as ledger
from worker_terminal import Fixture, wait_for_worker_exit


class WorkerFixtureTests(unittest.TestCase):
    def test_only_a_missing_group_confirms_exit(self):
        with patch('worker_terminal.os.killpg',
                   side_effect=[PermissionError(), None, ProcessLookupError()]) as probe:
            wait_for_worker_exit(1234)
        self.assertEqual(probe.call_args_list, [call(1234, 0)] * 3)

    def test_permission_denied_at_the_deadline_fails(self):
        with patch('worker_terminal.os.killpg', side_effect=PermissionError()), \
             patch('worker_terminal.wait_deadline', return_value=0):
            with self.assertRaisesRegex(AssertionError, 'worker group did not exit'):
                wait_for_worker_exit(1234)

    def test_commits_and_file_changes_stay_in_their_project(self):
        with tempfile.TemporaryDirectory(prefix='tinkery-TEST-worker-projects-') as directory:
            root = Path(directory).resolve()
            first = Fixture(root / 'first')
            (first.project / '_system/scripts/sdlc.py').write_text('TEST changed runtime')
            first.git('commit', '-qam', 'TEST changed one project')
            second = Fixture(root / 'second')
            self.assertEqual(second.git('log', '-1', '--format=%s'), 'TEST runtime boundary')
            self.assertEqual(second.git('status', '--porcelain'), '')
            self.assertEqual(second.git('rev-parse', '--show-toplevel'), str(second.project))

    def test_changing_one_session_preserves_the_next_confirmed_seed(self):
        with tempfile.TemporaryDirectory(prefix='tinkery-TEST-worker-copies-') as directory:
            root = Path(directory)
            for gray, authority in ((False, 'user'), (True, 'delegated')):
                with self.subTest(gray=gray):
                    first = Fixture(root / f'first-{authority}', gray=gray)
                    (first.session / 'seed-contract.md').write_text('TEST changed contract')
                    second = Fixture(root / f'second-{authority}', gray=gray)
                    self.assertTrue(ledger.status(second.session)['handoff']['verified'])
                    notes = next(node for node in ledger.load(second.session)['nodes']
                                 if node['id'] == 'notes')
                    self.assertEqual(notes['authority'], authority)


if __name__ == '__main__':
    unittest.main()
