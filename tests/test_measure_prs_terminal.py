"""Controlled fake-gh writes must finish a record before its PID is read."""
import json
from pathlib import Path
import select
import sys
import tempfile
import test_launch as subprocess
import unittest
from unittest import mock

from measure_prs_terminal import wait_for_call
from measure_terminal import Terminal
from vt_screen import Screen


class CallReadinessTests(unittest.TestCase):
    def terminal(self, path, process):
        terminal = Terminal.__new__(Terminal)
        terminal.calls = path
        terminal.master = -1
        terminal.screen = Screen(80, 24)
        terminal.process = process
        return terminal

    def test_waits_for_buffered_and_unterminated_records(self):
        for mode in ('buffered', 'unterminated'):
            with self.subTest(mode=mode), tempfile.TemporaryDirectory(prefix='tinkery-TEST-measure-call-') as directory:
                root = Path(directory)
                home = root / 'HOME'
                home.mkdir()
                script = root / 'fake-gh.py'
                script.write_text('''import json, os, pathlib, sys
with pathlib.Path('calls').open('a') as out:
    out.write(json.dumps(dict(pid=os.getpid())))
    if sys.argv[1] == 'buffered':
        out.write('\\n')
    else:
        out.flush()
    print('held', flush=True)
    assert sys.stdin.readline() == 'release\\n'
    if sys.argv[1] == 'unterminated':
        out.write('\\n')
''')
                with subprocess.Popen(
                    [sys.executable, str(script), mode], cwd=root,
                    env=subprocess.environment({}, home=home), stdin=subprocess.PIPE,
                    stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True,
                ) as writer:
                    try:
                        self.assertTrue(select.select([writer.stdout], [], [], 5)[0])
                        self.assertEqual(writer.stdout.readline(), 'held\n')
                        terminal = self.terminal(root / 'calls', writer)
                        if mode == 'buffered':
                            self.assertEqual(terminal.calls.read_bytes(), b'')
                        else:
                            self.assertFalse(terminal.calls.read_bytes().endswith(b'\n'))
                            self.assertEqual(json.loads(terminal.calls.read_text())['pid'], writer.pid)
                        polls = 0

                        def poll(*_):
                            nonlocal polls
                            if _[-1] == 0:
                                return [], [], []
                            polls += 1
                            if polls == 2:
                                writer.stdin.write('release\n')
                                writer.stdin.flush()
                                self.assertEqual(writer.wait(timeout=5), 0)
                            return [], [], []

                        with mock.patch('measure_terminal.select.select', side_effect=poll):
                            record = wait_for_call(terminal)
                        self.assertEqual(polls, 2, 'Accepted the record before the writer finished')
                        self.assertEqual(record['pid'], writer.pid)
                    finally:
                        if writer.poll() is None:
                            writer.kill()
                            writer.wait(timeout=5)

    def test_complete_malformed_record_still_fails(self):
        with tempfile.TemporaryDirectory(prefix='tinkery-TEST-measure-call-') as directory:
            path = Path(directory) / 'calls'
            path.write_text('not JSON\n')
            terminal = self.terminal(path, mock.Mock())
            with mock.patch('measure_terminal.select.select', return_value=([], [], [])):
                with self.assertRaises(json.JSONDecodeError):
                    wait_for_call(terminal)


if __name__ == '__main__':
    unittest.main()
