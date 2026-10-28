"""Terminal waits distinguish complete expected views from stale or partial transport."""
import unittest
from vt_screen import Screen


class FrameTests(unittest.TestCase):
    def test_only_a_complete_expected_view_can_finish_an_action(self):
        for expected in ('┌ Right now ', '1  Copy the reading',
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
