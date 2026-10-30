"""The shared terminal owner releases only its fake process and descriptors on failure."""
import errno
import fcntl
import importlib.util
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
from terminal_process import wait_output, TerminalProcess


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
        with self.assertRaisesRegex(AssertionError, 'predicate deadline'):
            self.wait_for(terminal, lambda: False, .12)
        self.assertGreaterEqual(time.monotonic() - started, .1)
        self.assertLess(time.monotonic() - started, 2)

    def test_exit_drain_cannot_extend_a_deadline(self):
        process = mock.Mock()
        process.poll.return_value = 0
        with mock.patch('terminal_process.select.select', return_value=([123], [], [])), \
             mock.patch('terminal_process.os.read', return_value=b'TEST endless output'):
            started = time.monotonic()
            list(wait_output(123, process, lambda chunk: None, started + .03))
        self.assertLess(time.monotonic() - started, 1)

    def test_inventoried_journeys_delegate_waiting(self):
        import ast
        root = Path(__file__).resolve().parents[1]
        inventory = {
            'tests/agent_operation_terminal.py': ['type_confirmation'],
            'tests/approve_terminal.py': ['wait'],
            'tests/brain_dump_terminal.py': ['await_text', 'absent'],
            'tests/dogfood_terminal.py': ['await_condition'],
            'tests/goal_confirmation_terminal.py': ['await_text', 'absent', 'journey'],
            'tests/goal_history_terminal.py': ['drain'],
            'tests/help_exit_terminal.py': ['await_condition'],
            'tests/measure_terminal.py': ['wait'],
            'tests/reading_scroll_terminal.py': ['wait_for'],
            'tests/selection_terminal.py': ['await_text'],
            'tests/shape_answer_terminal.py': ['wait', 'menu', 'read_until', 'exit_hint'],
            'tests/shape_incomplete_read_terminal.py': ['wait'],
            'tests/shape_retry_terminal.py': ['wait_calls', 'wait_either'],
            'tests/shape_sdlc_handoff.py': ['wait'],
            'tests/shape_terminal.py': ['wait'],
            'tests/taste_preview_terminal.py': ['wait'],
            'tests/taste_quiet_terminal.py': ['wait'],
            'tests/taste_terminal.py': ['wait'],
            'tests/tend_repo_terminal.py': ['wait', 'wait_exit'],
            'tests/tend_terminal.py': ['wait'],
            'tests/test_session_roots.py': ['run_ordinary'],
            'tests/test_terminal_process.py': ['test_output_screen_resize_and_normal_close'],
            'tests/theme_terminal.py': ['wait'],
            'tests/think_criteria_terminal.py': ['wait'],
            'tests/think_delivery_terminal.py': ['copy_board', 'read_right', 'journey'],
            'tests/think_escape_terminal.py': ['wait'],
            'tests/think_history_terminal.py': ['wait'],
            'tests/think_question_terminal.py': ['wait'],
            'tests/think_source_ref_terminal.py': ['wait'],
            'tests/voice_terminal.py': ['wait', 'text'],
            'tests/watch_terminal.py': ['wait'],
        }
        spec = importlib.util.spec_from_file_location('wait_inventory', root / 'scripts/migrate-terminal-waits.py')
        audit = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(audit)
        shared, local, unknown = audit.inventory(root)
        self.assertTrue(shared)
        self.assertTrue(local)
        self.assertEqual(unknown, [], 'A local PTY wait has not delegated or justified its differing semantics')
        for path, names in inventory.items():
            tree = ast.parse((root / path).read_text())
            for name in names:
                functions = [n for n in ast.walk(tree) if isinstance(n, ast.FunctionDef) and n.name == name]
                self.assertTrue(functions, (path, name))
                for function in functions:
                    if path == 'tests/voice_terminal.py' and name == 'text' and not any(isinstance(n, ast.For) for n in ast.walk(function)):
                        continue
                    self.assertTrue(any(isinstance(n, ast.Call) and isinstance(n.func, ast.Name)
                                        and n.func.id == 'wait_output' for n in ast.walk(function)),
                                    (path, name))


if __name__ == '__main__':
    unittest.main()
