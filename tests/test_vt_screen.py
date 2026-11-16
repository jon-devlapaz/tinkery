"""Terminal waits distinguish complete expected views from stale or partial transport."""
import unittest
from vt_screen import Screen


class FrameTests(unittest.TestCase):
    def test_history_keeps_transient_frames_after_the_action_starts(self):
        screen = Screen(100, 30)
        def frame(text):
            return ('\x1b[?2026h\x1b[2J\x1b[1;1H' + text + '\x1b[?2026l').encode()
        screen.feed(frame('old saving notice'))
        before = screen.frames
        screen.feed(frame('transient saving notice') + frame('final refusal'))
        def shown_after(text):
            return any(number > before and text in view
                       for number, view in screen.frame_history)
        self.assertTrue(shown_after('transient saving notice'))
        self.assertTrue(shown_after('final refusal'))
        self.assertNotIn('transient saving notice', screen.text())
        self.assertFalse(shown_after('old saving notice'))
        self.assertEqual([number for number, _ in screen.frame_history], [1, 2, 3])
        screen.feed(frame('unfinished notice')[:-1])
        self.assertFalse(shown_after('unfinished notice'))
        screen.feed(b'l')
        self.assertTrue(shown_after('unfinished notice'))
        for _ in range(40):
            screen.feed(frame('later frame'))
        self.assertEqual(len(screen.frame_history), 32)
        self.assertFalse(shown_after('transient saving notice'))
        self.assertEqual(screen.frame_history[-1][0], screen.frames)

    def test_only_a_complete_expected_view_can_finish_an_action(self):
        for expected in ('┌ Right now ', 'PgUp/PgDn scroll reading', '1  Copy the reading',
                         '↑↓←→ move   enter open   l link   esc back'):
            with self.subTest(expected=expected):
                screen = Screen(100, 30)
                def frame(text):
                    return ('\x1b[?2026h\x1b[2J\x1b[1;1H' + text + '\x1b[?2026l').encode()
                screen.feed(frame(expected))
                before = screen.frames
                def finished():
                    return screen.ready(before) and expected in screen.text()
                self.assertFalse(finished(), 'old expected text is not an action response')
                # A queued repaint can complete without showing the requested view.
                screen.feed(frame('previous view'))
                self.assertTrue(screen.ready(before))
                self.assertFalse(finished(), 'completion alone does not identify the new view')
                # Split both synchronized-update escapes and delay all remaining bytes.
                data = frame(expected)
                start = data.index(b'h') + 1
                for chunk in (data[:start-1], data[start-1:start], data[start:-1]):
                    screen.feed(chunk)
                    self.assertFalse(finished())
                screen.feed(data[-1:])
                self.assertTrue(finished())
                # A new partial start makes the displayed view unsafe again.
                screen.feed(b'\x1b[?2026')
                self.assertFalse(finished())
                screen.feed(b'h')
                self.assertFalse(finished(), 'old cells remain visible inside an open update')


if __name__ == '__main__':
    unittest.main()
