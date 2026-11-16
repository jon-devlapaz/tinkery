"""Confirmation assertions wait for the input echo, not an already-visible placeholder."""
import os
import unittest
from unittest import mock

import shape_answer_terminal as journey


class ConfirmationWaitTests(unittest.TestCase):
    def test_confirmation_hint_waits_for_typed_words_before_asserting(self):
        original_write = os.write
        original_wait = journey.Run.wait
        pending = {}
        delivered = []

        def hold_confirmation(fd, data):
            if data == b'confirm':
                pending[fd] = data
                return len(data)
            return original_write(fd, data)

        def wait_for_echo(run, text, *args, **kwargs):
            if text == 'your words confirm' and run.master in pending:
                self.assertIn('type confirm, then Enter', run.flat())
                self.assertNotIn('your words confirm', run.flat())
                original_write(run.master, pending.pop(run.master))
                delivered.append(run.master)
            return original_wait(run, text, *args, **kwargs)

        with mock.patch('os.write', side_effect=hold_confirmation), \
             mock.patch.object(journey.Run, 'wait', wait_for_echo), \
             mock.patch.dict(os.environ, NO_COLOR='1'):
            journey.case('confirmation hint', 80, 24)
        self.assertEqual(len(delivered), 1)
        self.assertFalse(pending)


if __name__ == '__main__':
    unittest.main()
