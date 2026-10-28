"""The shared terminal owner releases only its fake process and descriptors on failure."""
import fcntl
import os
from pathlib import Path
import select
import struct
import sys
import tempfile
import termios
import time
import unittest
from unittest import mock

import test_launch
from terminal_process import TerminalProcess


class OwnershipTests(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory(prefix='tinkery-TEST-terminal-owner-')
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        self.env = test_launch.environment(home=self.root / 'HOME')

    def launch(self):
        terminal = TerminalProcess([sys.executable, '-u', '-c',
                                "import time; print('\\x1b[?2026h\\x1b[1;1HREADY\\x1b[?2026l', flush=True); time.sleep(30)"],
                               cwd=self.root, env=self.env, width=80, height=24)
        self.addCleanup(self.cleanup, terminal)
        return terminal

    def close_descriptor(self, descriptor):
        try:
            os.close(descriptor)
        except OSError:
            pass

    def cleanup(self, terminal):
        # Observe owner failures before this safety net reaps only our fake.
        if terminal.process.poll() is None:
            terminal.process.kill()
        terminal.process.wait()
        for descriptor in (terminal.master, terminal.slave):
            self.close_descriptor(descriptor)

    def assert_closed(self, terminal):
        self.assertIsNotNone(terminal.process.poll())
        for descriptor in (terminal.master, terminal.slave):
            with self.assertRaises(OSError):
                os.fstat(descriptor)

    def test_output_screen_resize_and_normal_close(self):
        terminal = self.launch()
        try:
            deadline = time.monotonic() + 5
            while time.monotonic() < deadline:
                if select.select([terminal.master], [], [], .02)[0]:
                    terminal.feed()
                if terminal.screen.ready() and 'READY' in terminal.screen.text():
                    break
            else:
                self.fail('fake complete output missing')
            self.assertIn('READY', terminal.screen.text())
            self.assertTrue(terminal.screen.ready())
            self.assertIn(b'READY', terminal.raw)
            terminal.resize(100, 30)
            self.assertEqual((terminal.screen.width, terminal.screen.height), (100, 30))
            self.assertEqual(struct.unpack('HHHH', fcntl.ioctl(terminal.slave, termios.TIOCGWINSZ,
                                                             bytes(8)))[:2], (30, 100))
        finally:
            terminal.close()
        self.assert_closed(terminal)

    def test_feature_exception_still_reaps_process_and_closes_descriptors(self):
        terminal = self.launch()
        with self.assertRaisesRegex(RuntimeError, 'TEST feature failed'):
            try:
                raise RuntimeError('TEST feature failed')
            finally:
                terminal.close()
        self.assert_closed(terminal)

    def test_launch_failure_closes_both_descriptors(self):
        descriptors = []
        openpty = os.openpty
        def allocate():
            pair = openpty()
            descriptors.extend(pair)
            for descriptor in pair:
                self.addCleanup(self.close_descriptor, descriptor)
            return pair
        with mock.patch('terminal_process.os.openpty', side_effect=allocate):
            with self.assertRaises(FileNotFoundError):
                TerminalProcess([str(self.root / 'TEST-missing')], cwd=self.root, env=self.env,
                                width=80, height=24)
        self.assertEqual(len(descriptors), 2)
        for descriptor in descriptors:
            with self.assertRaises(OSError):
                os.fstat(descriptor)


if __name__ == '__main__':
    unittest.main()
