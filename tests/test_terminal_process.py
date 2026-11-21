"""The shared terminal owner releases only its fake process and descriptors on failure."""
import errno
import fcntl
import importlib.util
import io
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
import terminal_process
from terminal_process import wait_output, TerminalProcess


class DeadlineTests(unittest.TestCase):
    def test_default_preserves_the_checked_five_second_wait(self):
        with mock.patch.dict(os.environ, {}, clear=True), \
             mock.patch('terminal_process.time.monotonic', return_value=100):
            self.assertEqual(terminal_process.wait_deadline(), 105)

    def test_one_scale_applies_to_default_and_explicit_waits(self):
        with mock.patch.dict(os.environ, TINKERY_TEST_WAIT_SCALE='3'), \
             mock.patch('terminal_process.time.monotonic', return_value=100):
            self.assertEqual(terminal_process.wait_deadline(), 115)
            self.assertEqual(terminal_process.wait_deadline(20), 160)

    def test_feature_wait_helpers_have_no_local_unscaled_deadlines(self):
        import ast
        root = Path(__file__).resolve().parent
        for path in root.glob('*.py'):
            if path.name in ('terminal_process.py', 'test_terminal_process.py', 'test_python_tests.py'):
                continue
            tree = ast.parse(path.read_text())
            parents = {child: node for node in ast.walk(tree) for child in ast.iter_child_nodes(node)}
            for node in ast.walk(tree):
                if not isinstance(node, ast.BinOp) or not isinstance(node.op, ast.Add):
                    continue
                if ast.unparse(node.left) != 'time.monotonic()':
                    continue
                if isinstance(node.right, ast.Constant) and node.right.value < 1:
                    continue
                owner = parents.get(node)
                while owner is not None and not isinstance(owner, ast.FunctionDef):
                    owner = parents.get(owner)
                self.assertIn(owner.name if owner else None, ('pump', 'settle'),
                              f'{path.name}:{node.lineno} bypasses wait_deadline')

    def test_invalid_scales_fail_instead_of_skipping_the_wait(self):
        for value in ('0', '-1', 'nan', 'inf', 'invalid'):
            with self.subTest(value=value), mock.patch.dict(os.environ, TINKERY_TEST_WAIT_SCALE=value):
                with self.assertRaises(ValueError):
                    terminal_process.wait_deadline()


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
            def consume_wait_output(chunk):
                terminal.raw.extend(chunk)
                terminal.screen.feed(chunk)
            for _ in wait_output(terminal.master, terminal.process, consume_wait_output, deadline):
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


class ExitDrainTests(unittest.TestCase):
    def launch(self, script):
        temporary = tempfile.TemporaryDirectory(prefix='tinkery-TEST-exit-drain-')
        self.addCleanup(temporary.cleanup)
        root = Path(temporary.name)
        terminal = TerminalProcess([sys.executable, '-u', '-c', script], cwd=root,
                                   env=test_launch.environment(home=root / 'HOME'),
                                   width=80, height=24)
        self.addCleanup(terminal.close)
        return terminal

    def wait_for(self, terminal, predicate, seconds):
        for _ in wait_output(terminal.master, terminal.process, terminal.screen.feed,
                             time.monotonic() + seconds):
            if predicate():
                return
        raise AssertionError('TEST predicate deadline or process exit')

    def test_delayed_reader_small_reads_keep_complete_final_output(self):
        # Screen deliberately does not emulate autowrap; place the final text explicitly.
        payload = '\x1b[?2026h\x1b[1;1H' + 'x' * 200 + '\x1b[2;1HTEST FINAL\x1b[?2026l'
        terminal = self.launch(f'print({payload!r}, flush=True)')
        self.assertEqual(terminal.process.wait(timeout=5), 0)
        read = os.read
        with mock.patch('terminal_process.os.read',
                        side_effect=lambda fd, size: read(fd, min(size, 7))) as reads:
            self.wait_for(terminal, lambda: terminal.screen.ready() and
                          'TEST FINAL' in terminal.screen.text(), 2)
        self.assertGreater(reads.call_count, 1)
        self.assertIn('TEST FINAL', terminal.screen.text())
        self.assertTrue(terminal.screen.ready())

    def test_eof_and_exited_pty_eio_preserve_nonzero_exit(self):
        process = mock.Mock()
        process.poll.return_value = 7
        for result in (b'', OSError(errno.EIO, 'TEST closed PTY')):
            with self.subTest(result=result), \
                 mock.patch('terminal_process.select.select', return_value=([123], [], [])), \
                 mock.patch('terminal_process.os.read', side_effect=[result, result]):
                self.assertEqual(list(wait_output(123, process, self.fail,
                                                  time.monotonic() + 1)), [7])

    def test_live_eof_paces_predicate_checks_until_deadline(self):
        reader, writer = os.pipe()
        self.addCleanup(os.close, reader)
        os.close(writer)
        process = mock.Mock()
        process.poll.return_value = None
        started = time.monotonic()
        with mock.patch('terminal_process.sys.stderr', io.StringIO()):
            checks = sum(1 for _ in wait_output(reader, process, self.fail,
                                               started + .06, interval=.01))
        self.assertGreaterEqual(time.monotonic() - started, .05)
        self.assertLess(time.monotonic() - started, 1)
        self.assertGreater(checks, 0)
        self.assertLessEqual(checks, 20, 'EOF caused unpaced predicate polling')

    def test_unexpected_read_errors_are_not_hidden(self):
        for code, returncode in ((errno.EIO, None), (errno.EBADF, 7)):
            process = mock.Mock()
            process.poll.return_value = returncode
            with self.subTest(code=code), \
                 mock.patch('terminal_process.select.select', return_value=([123], [], [])), \
                 mock.patch('terminal_process.os.read', side_effect=OSError(code, 'TEST read error')):
                with self.assertRaises(OSError) as error:
                    list(wait_output(123, process, self.fail, time.monotonic() + 1))
                self.assertEqual(error.exception.errno, code)

    def test_never_satisfied_predicate_has_a_deadline(self):
        terminal = self.launch("import time; print('TEST running', flush=True); time.sleep(30)")
        started = time.monotonic()
        with mock.patch('terminal_process.sys.stderr', io.StringIO()), \
             self.assertRaisesRegex(AssertionError, 'predicate deadline'):
            self.wait_for(terminal, lambda: False, .12)
        self.assertGreaterEqual(time.monotonic() - started, .1)
        self.assertLess(time.monotonic() - started, 2)

    def test_deadline_reports_raw_tail_and_split_frame_count(self):
        process = mock.Mock()
        process.poll.return_value = None
        chunks = iter([b'x' * 5000 + b'\x1b[?2026', b'l\x1b[?2026hTEST unfinished'])
        received = []
        output = io.StringIO()
        with mock.patch('terminal_process.select.select', return_value=([123], [], [])), \
             mock.patch('terminal_process.os.read', side_effect=lambda *_: next(chunks, b'')), \
             mock.patch('terminal_process.sys.stderr', output):
            list(wait_output(123, process, received.append, time.monotonic() + .03, interval=.005))
        self.assertEqual(len(received), 2)
        report = output.getvalue()
        self.assertIn('completed_frames_in_wait=1', report)
        self.assertIn(repr(b'\x1b[?2026l\x1b[?2026hTEST unfinished')[2:-1], report)
        self.assertLess(len(report), 4300)

    def test_exhausted_settle_does_not_print_deadline_diagnostics(self):
        process = mock.Mock()
        process.poll.return_value = None
        output = io.StringIO()
        with mock.patch('terminal_process.select.select', return_value=([], [], [])), \
             mock.patch('terminal_process.sys.stderr', output):
            checks = list(wait_output(123, process, self.fail, time.monotonic() + .03,
                                      interval=.005, report=False))
        self.assertTrue(checks)
        self.assertEqual(output.getvalue(), '')

    def test_satisfied_wait_does_not_print_deadline_diagnostics(self):
        process = mock.Mock()
        process.poll.return_value = None
        output = io.StringIO()
        with mock.patch('terminal_process.select.select', return_value=([123], [], [])), \
             mock.patch('terminal_process.os.read', return_value=b'TEST ready'), \
             mock.patch('terminal_process.sys.stderr', output):
            for _ in wait_output(123, process, lambda _: None, time.monotonic() + .03):
                break
        self.assertEqual(output.getvalue(), '')

    def test_exit_drain_cannot_extend_a_deadline(self):
        process = mock.Mock()
        process.poll.return_value = 0
        with mock.patch('terminal_process.select.select', return_value=([123], [], [])), \
             mock.patch('terminal_process.os.read', return_value=b'TEST endless output'):
            started = time.monotonic()
            list(wait_output(123, process, lambda chunk: None, started + .03))
        self.assertLess(time.monotonic() - started, 1)


if __name__ == '__main__':
    unittest.main()
