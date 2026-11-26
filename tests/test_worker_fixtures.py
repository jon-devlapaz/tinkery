"""Worker test cases each own a verified seed they can change."""
from pathlib import Path
import tempfile
import unittest

import session_fixture as ledger
from worker_terminal import Fixture


class WorkerFixtureTests(unittest.TestCase):
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
