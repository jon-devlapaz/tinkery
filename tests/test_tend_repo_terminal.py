"""Deterministic process-exit interleavings; no application, helper or provider runs."""
import importlib.util
import os
from pathlib import Path
import unittest
from unittest import mock

ROOT = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location('tend_repo_journey', ROOT / 'tests/tend_repo_terminal.py')
JOURNEY = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(JOURNEY)


class ExitWaitTests(unittest.TestCase):
    def terminal(self, polls, synchronized=False):
        terminal = JOURNEY.Terminal.__new__(JOURNEY.Terminal)
        terminal.master = -1
        terminal.screen = JOURNEY.Screen(80, 24)
        terminal.screen.synchronized = synchronized
        terminal.process = mock.Mock()
        terminal.process.poll.side_effect = polls
        return terminal

    def test_exit_between_frame_checks_is_success_not_an_assertion(self):
        terminal = self.terminal([None, 0])
        with mock.patch.object(JOURNEY.select, 'select', return_value=([], [], [])):
            self.assertEqual(terminal.wait_exit(), 0)

    def test_exit_does_not_require_a_completed_screen_frame(self):
        terminal = self.terminal([0], synchronized=True)
        with mock.patch.object(JOURNEY.select, 'select', return_value=([], [], [])):
            self.assertEqual(terminal.wait_exit(), 0)

    def test_exit_wait_drains_pending_output_while_process_is_alive(self):
        terminal = self.terminal([None, 0])
        with mock.patch.object(JOURNEY.select, 'select', return_value=([-1], [], [])), \
             mock.patch.object(JOURNEY.os, 'read', return_value=b'TEST final output') as read:
            self.assertEqual(terminal.wait_exit(), 0)
        read.assert_called_once_with(-1, 65536)
        self.assertIn('TEST final output', terminal.screen.text())

    def test_nonzero_exit_is_not_converted_to_success(self):
        terminal = self.terminal([7])
        with mock.patch.object(JOURNEY.select, 'select', return_value=([], [], [])):
            self.assertEqual(terminal.wait_exit(), 7)

    def test_frame_wait_still_rejects_incomplete_frames(self):
        terminal = self.terminal(None, synchronized=True)
        terminal.process.poll.return_value = None
        with mock.patch.object(JOURNEY.time, 'monotonic', side_effect=[0, 0, 11]), \
             mock.patch.object(JOURNEY.select, 'select', return_value=([], [], [])):
            with self.assertRaises(AssertionError):
                terminal.wait(lambda _: True)

    def test_a_running_process_still_has_a_deadline(self):
        terminal = self.terminal(None)
        terminal.process.poll.return_value = None
        with mock.patch.object(JOURNEY.time, 'monotonic', side_effect=[0, 0, 11]), \
             mock.patch.object(JOURNEY.select, 'select', return_value=([], [], [])):
            with self.assertRaises(AssertionError):
                terminal.wait_exit()


if __name__ == '__main__':
    unittest.main()
