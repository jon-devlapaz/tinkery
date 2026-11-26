"""Journey waits must observe the destination and the end of history under delayed input."""
import os
import threading
import unittest
from unittest import mock
from vt_screen import Screen

import reading_scroll_terminal as reading
import shape_answer_terminal as shape
import shape_terminal as shape_reading
import test_shape_confirmation_wait as confirmation
import tend_thought_terminal as thoughts
import think_history_terminal as history
import think_delivery_terminal as delivery


class JourneyWaitTests(unittest.TestCase):
    def test_launch_refusal_can_finish_without_a_terminal_frame(self):
        term = mock.Mock()
        term.screen = Screen(80, 24)
        term.screen.feed(b'Error: TEST refused before terminal setup')
        with mock.patch.object(history, 'wait_output', return_value=iter([1])):
            self.assertIn('TEST refused', history.Terminal.wait(term, lambda text: 'TEST refused' in text))

    def test_waits_reject_a_partial_next_frame_escape(self):
        for module in (history, thoughts.flow):
            with self.subTest(module=module.__name__):
                term = mock.Mock()
                term.screen = Screen(80, 24)
                term.screen.feed(b'\x1b[?2026hTEST target\x1b[?2026l\x1b[?202')
                finished = []

                def frames(*args):
                    yield None
                    term.screen.feed(b'6h\x1b[2J\x1b[1;1HTEST target new\x1b[?2026l')
                    finished.append(True)
                    yield None

                with mock.patch.object(module, 'wait_output', frames):
                    if module is history:
                        history.Terminal.wait(term, lambda text: 'TEST target' in text)
                    else:
                        module.wait(term, 'TEST target')
                self.assertEqual(finished, [True])

    def test_confirmation_delay_does_not_stop_a_partial_terminal_frame(self):
        original_type = shape.Run.type

        def begin_frame_before_type(run, words):
            if words == 'confirm':
                os.write(run.slave, b'\x1b[?2026h')
            return original_type(run, words)

        with mock.patch.object(shape.Run, 'type', begin_frame_before_type):
            confirmation.ConfirmationWaitTests(
                'test_confirmation_hint_waits_for_typed_words_before_asserting'
            ).test_confirmation_hint_waits_for_typed_words_before_asserting()

    def test_thought_reader_waits_for_its_title_not_garden_words(self):
        original_send = thoughts.flow.send
        write = os.write

        def delayed_reader(term, keys, text):
            if keys != b'v' or '2 thoughts could not be read.' not in term.screen.text():
                return original_send(term, keys, text)
            delayed = []

            def hold_input(fd, data):
                if fd == term.master and data == b'v':
                    timer = threading.Timer(.15, write, args=(fd, data))
                    delayed.append(timer)
                    timer.start()
                    return len(data)
                return write(fd, data)

            try:
                with mock.patch('os.write', side_effect=hold_input):
                    return original_send(term, keys, text)
            finally:
                for timer in delayed:
                    timer.join()

        with mock.patch.object(thoughts.flow, 'send', side_effect=delayed_reader):
            thoughts.unreadable((80, 24))

    def test_a_late_page_down_still_shows_the_last_exclusion(self):
        write = os.write
        delayed = None

        def delay_one_page(fd, data):
            nonlocal delayed
            if delayed is None and data == b'\x1b[6~':
                delayed = threading.Timer(.5, write, args=(fd, data))
                delayed.start()
                return len(data)
            return write(fd, data)

        try:
            with mock.patch('os.write', side_effect=delay_one_page):
                reading.journey(100, 30, False)
            self.assertIsNotNone(delayed)
        finally:
            if delayed is not None:
                delayed.cancel()
                delayed.join()

    def test_enter_waits_for_tend_when_home_is_still_visible(self):
        write = os.write
        for terminal, journey, args in (
            (shape, delivery.journey, (80, 24, True)),
            (shape_reading, shape_reading.journey, (100, 30, 'question')),
        ):
            with self.subTest(journey=journey.__module__):
                pending = None
                delivered = False
                wait_output = terminal.wait_output

                def queue_navigation(fd, data):
                    nonlocal pending
                    self.assertIsNone(pending, 'A second key arrived before Tend opened.')
                    if data == b'2':
                        pending = (fd, data)
                        return len(data)
                    return write(fd, data)

                def observe_previous_screen(*args, **kwargs):
                    nonlocal pending, delivered
                    for status in wait_output(*args, **kwargs):
                        yield status
                        if pending is not None:
                            write(*pending)
                            pending = None
                            delivered = True

                with mock.patch.object(os, 'write', queue_navigation), \
                     mock.patch.object(terminal, 'wait_output', observe_previous_screen):
                    journey(*args)
                self.assertTrue(delivered)

    def test_delivery_reads_to_its_known_end_after_a_delayed_down_key(self):
        write = os.write
        close = delivery.Run.close
        delayed = []

        def hold_input(fd, data):
            if data == b'\x1b[B' and not delayed:
                timer = threading.Timer(.75, write, args=(fd, data))
                delayed.append(timer)
                timer.start()
                return len(data)
            return write(fd, data)

        def close_after_delivery(run):
            for timer in delayed:
                timer.join()
            return close(run)

        with mock.patch('os.write', side_effect=hold_input), \
             mock.patch.object(delivery.Run, 'close', close_after_delivery):
            delivery.journey(80, 24, True)
        self.assertEqual(len(delayed), 1)

    def test_history_reads_to_its_known_end_after_a_delayed_down_key(self):
        write = os.write
        close = history.Terminal.close
        delayed = []
        downs = 0

        def hold_input(fd, data):
            nonlocal downs
            if data == b'\x1b[B':
                downs += 1
                if downs == 2:
                    timer = threading.Timer(.75, write, args=(fd, data))
                    delayed.append(timer)
                    timer.start()
                    return len(data)
            return write(fd, data)

        def close_after_delivery(term):
            for timer in delayed:
                timer.join()
            return close(term)

        with mock.patch('os.write', side_effect=hold_input), \
             mock.patch.object(history.Terminal, 'close', close_after_delivery):
            history.journey(80, 24, True)
        self.assertEqual(len(delayed), 1)


if __name__ == '__main__':
    unittest.main()
