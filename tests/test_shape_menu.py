"""Run.menu advances only after complete frames, with bounded failure and no discard answer."""
from pathlib import Path
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import patch

from shape_answer_terminal import ESC, Run
from vt_screen import Screen


def frame(text):
    return b'\x1b[?2026h\x1b[2J\x1b[H' + text.encode() + b'\x1b[?2026l'


class MenuTests(unittest.TestCase):
    def run_menu(self, transitions, initial=None):
        screen = Screen(100, 30)
        screen.feed(initial or frame('Shape help'))
        clock, pending, sent = [0.0], [], []
        run = SimpleNamespace(screen=screen, master=123, process=SimpleNamespace(poll=lambda: None))
        run.flat = lambda: ' '.join(screen.text().split())
        run.wait = lambda text, timeout=20: Run.wait(run, text, timeout)

        def select(*_):
            clock[0] += .02
            return ([123], [], []) if pending and pending[0][0] <= clock[0] else ([], [], [])

        def write(_, data):
            sent.append(clock[0])
            self.assertFalse(screen.synchronized, 'key sent through an incomplete frame')
            self.assertNotIn('lose this draft?', run.flat(), 'key answered discard prompt')
            self.assertEqual(data, ESC)
            if transitions:
                pending.extend((clock[0] + delay, chunk) for delay, chunk in transitions.pop(0))
            return len(data)

        with patch('shape_answer_terminal.time.monotonic', lambda: clock[0]), \
             patch('shape_answer_terminal.select.select', select), \
             patch('shape_answer_terminal.os.write', write), \
             patch('shape_answer_terminal.os.read', lambda *_: pending.pop(0)[1]):
            try:
                Run.menu(run)
                error = None
            except AssertionError as failure:
                error = failure
        return sent, clock[0], error, run.flat()

    def test_intermediate_screens_advance_promptly(self):
        sent, elapsed, error, _ = self.run_menu([
            [(.04, frame('Shape esc back'))],
            [(.04, frame('Tend s shape'))],
            [(.04, frame('Tend enter open'))],
            [(.04, frame('4  approve'))],
        ])
        self.assertIsNone(error)
        self.assertEqual(len(sent), 4)
        self.assertLess(elapsed, .3)

    def test_incomplete_changed_frame_waits_before_next_key(self):
        sent, _, error, _ = self.run_menu([
            [(.02, b'\x1b[?2026h\x1b[2J\x1b[HShape esc back'), (.5, b'\x1b[?2026l')],
            [(.02, frame('4  approve'))],
        ])
        self.assertIsNone(error)
        self.assertGreaterEqual(sent[1] - sent[0], .5)

    def test_absent_and_late_transitions_fail_without_more_keys(self):
        for transition in ([], [(3.2, frame('4  approve'))],
                           [(.02, b'\x1b[?2026h\x1b[2J\x1b[H4  approve')]):
            with self.subTest(transition=transition):
                sent, elapsed, error, _ = self.run_menu([transition])
                self.assertIsNotNone(error)
                self.assertEqual(len(sent), 1)
                self.assertLess(elapsed, 3.1)

    def test_discard_prompt_gets_no_following_key(self):
        prompt = 'Back to menu and lose this draft? y / n'
        sent, _, error, text = self.run_menu([[ (.02, frame(prompt)) ]])
        self.assertIsNotNone(error)
        self.assertEqual(len(sent), 1)
        self.assertIn(prompt, text)
        sent, _, error, _ = self.run_menu([], initial=frame(prompt))
        self.assertIsNotNone(error)
        self.assertEqual(sent, [])

    def test_initial_incomplete_frame_never_gets_a_key(self):
        sent, elapsed, error, _ = self.run_menu([], initial=b'\x1b[?2026hShape help')
        self.assertIsNotNone(error)
        self.assertEqual(sent, [])
        self.assertLess(elapsed, 12.1)

    def test_real_unsaved_words_remain_until_discard_is_answered(self):
        with tempfile.TemporaryDirectory(prefix='tinkery-TEST-menu-draft-') as directory:
            run = Run(Path(directory), 100, 30, budget=0, agent=True)
            try:
                run.wait('1  think')
                run.send(b'1', "What's on your mind?")
                run.type('TEST keep these exact unsaved words')
                with self.assertRaisesRegex(AssertionError, 'Esc never reached the menu'):
                    run.menu()
                run.wait('Back to menu and lose this draft? y / n')
                run.send(b'n', 'TEST keep these exact unsaved words')
                self.assertEqual(run.calls(), [])
            finally:
                run.close()

    def test_four_transitions_still_require_menu(self):
        sent, elapsed, error, _ = self.run_menu([
            [(2.9, frame(f'Intermediate {i}'))] for i in range(4)
        ])
        self.assertIsNotNone(error)
        self.assertEqual(len(sent), 4)
        self.assertLess(elapsed, 12.1)


if __name__ == '__main__':
    unittest.main()
