"""#176: real Shape list discovery in a PTY, with TEST roots and fail-if-called helpers/providers."""
import fcntl
import json
import os
from pathlib import Path
import select
import struct
import subprocess
import tempfile
import termios
import time
import unittest

from vt_screen import Screen

REPO = Path(__file__).resolve().parents[1]
EMPTY = "No goals are waiting for a seed."
OFFERED = "TEST eligible goal"
HIDDEN = "TEST ended goal"


def files(root):
    return {str(p.relative_to(root)): p.read_bytes() for p in root.rglob('*') if p.is_file()}


class ShapeIncompleteReadJourney(unittest.TestCase):
    def journey(self, populated, unreadable, width, height, no_color):
        parent = REPO / 'target' / 'tmp'
        parent.mkdir(parents=True, exist_ok=True)
        with tempfile.TemporaryDirectory(prefix='tinkery-TEST-176-pty-', dir=parent) as directory:
            root = Path(directory).resolve()
            sessions = root / 'TEST-sessions'
            sessions.mkdir()
            if unreadable:
                broken = sessions / 'TEST-unreadable'
                broken.mkdir()
                (broken / 'ledger.json').write_text('not JSON: TEST unreadable ledger')
            if populated:
                for name, goal, status in [('TEST-eligible', OFFERED, 'active'),
                                           ('TEST-ended', HIDDEN, 'completed')]:
                    session = sessions / name
                    session.mkdir()
                    (session / 'ledger.json').write_text(json.dumps({
                        'session_id': name, 'status': status, 'goal': goal,
                        'updated_at': '2026-10-17T00:00:00Z'}))
            skill = root / 'TEST-skill' / 'SKILL.md'
            skill.parent.mkdir()
            skill.write_bytes((REPO / '.ci-seed-me/skills/seed-me/SKILL.md').read_bytes())
            helper = skill.parent / 'scripts' / 'session.py'
            helper.parent.mkdir()
            helper.write_text("from pathlib import Path\nPath(__file__).parents[2].joinpath('helper-was-run').write_text('unexpected')\nraise SystemExit(86)\n")
            provider = root / 'never-run-pi'
            provider.write_text('#!/bin/sh\necho unexpected > provider-was-run\nexit 86\n')
            provider.chmod(0o755)
            home = root / 'TEST-home'
            home.mkdir()
            before = files(sessions)
            baseline_files = files(root)
            env = dict(os.environ, HOME=str(home), TERM='xterm-256color',
                       TMPDIR=str(root), PYTHONDONTWRITEBYTECODE='1',
                       TINKERY_NODE='/tinkery-TEST-no-node')
            env.pop('NO_COLOR', None)
            if no_color:
                env['NO_COLOR'] = '1'
            master, slave = os.openpty()
            fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack('HHHH', height, width, 0, 0))
            process = subprocess.Popen([
                str(REPO / 'target/debug/tinkery'), '--home', '--shape-pi',
                '--model', 'test/model', '--shape-budget', '0', '--seed-me', str(skill),
                '--seed-session-root', str(sessions), '--record-root', str(root / 'TEST-records'),
                '--link-root', str(root / 'TEST-links'), '--pi-command', str(provider),
            ], cwd=root, env=env, stdin=slave, stdout=slave, stderr=slave)
            screen = Screen(width, height)

            def flat():
                return ' '.join(screen.text().split())

            def wait(predicate):
                deadline = time.monotonic() + 8
                while time.monotonic() < deadline:
                    if select.select([master], [], [], .02)[0]:
                        screen.feed(os.read(master, 65536))
                    if predicate() and not screen.synchronized:
                        return
                    self.assertIsNone(process.poll(), screen.text())
                self.fail('TEST discovery/navigation timeout:\n' + screen.text())

            def shape():
                os.write(master, b'3')
                wait(lambda: 'tinkery › shape' in flat() and 'Reading your sessions' not in flat())
                return flat(), screen.text()

            try:
                wait(lambda: '3 shape' in flat())
                actual, captured = shape()
                os.write(master, b'\x1b')
                wait(lambda: '3 shape' in flat())
                again, _ = shape()
                self.assertEqual(actual, again, 'returning changed settled Shape output')
                self.assertEqual(OFFERED in actual, populated, captured)
                self.assertNotIn(HIDDEN, actual, captured)
                if unreadable:
                    self.assertRegex(actual, r'\b1 (?:record|session)s?\b',
                                     '#176 missing incomplete-read notice/count:\n' + captured)
                    self.assertIn("couldn't be read", actual, captured)
                    self.assertNotIn(EMPTY, actual, captured)
                    self.assertIn('F1 shows why.', actual, captured)
                    self.assertNotIn('or Tend shows why', actual, captured)
                    os.write(master, b'\x1bOP')
                    wait(lambda: 'Unreadable session records' in flat())
                    pages = []
                    for _ in range(12):
                        pages.append(screen.text())
                        os.write(master, b'\x1b[6~')
                        until = time.monotonic() + .08
                        while time.monotonic() < until:
                            if select.select([master], [], [], .02)[0]:
                                screen.feed(os.read(master, 65536))
                    help_text = '\n'.join(pages)
                    compact = ''.join(help_text.split())
                    self.assertIn(str(sessions / 'TEST-unreadable' / 'ledger.json'), compact, help_text)
                    self.assertIn('Invalid JSON', help_text, help_text)
                    self.assertNotIn('Tend shows the discovery details', help_text)
                    os.write(master, b'\x1b')
                    wait(lambda: 'tinkery › shape' in flat() and 'Unreadable session records' not in flat())
                else:
                    self.assertIn(EMPTY, actual, captured)
                print(f'TEST #176 PTY populated={populated} unreadable={unreadable} '
                      f'{width}x{height} NO_COLOR={no_color}: passed')
            finally:
                if process.poll() is None:
                    process.kill()
                process.wait(timeout=5)
                os.close(master)
                os.close(slave)
                self.assertEqual(files(sessions), before, 'Shape changed TEST session files')
                self.assertFalse((root / 'helper-was-run').exists(), 'Seed Me helper was invoked')
                self.assertFalse((root / 'provider-was-run').exists(), 'provider was invoked')
                self.assertFalse((root / 'TEST-records').exists(), 'record root was created')
                self.assertFalse((root / 'TEST-links').exists(), 'link root was created')
                self.assertEqual(files(root), baseline_files, 'read-only navigation wrote TEST files')

    def test_unreadable_empty_list(self):
        self.cases(populated=False, unreadable=True)

    def test_unreadable_populated_list(self):
        self.cases(populated=True, unreadable=True)

    def test_complete_empty_control(self):
        self.cases(populated=False, unreadable=False)

    def cases(self, populated, unreadable):
        for width, height in [(100, 30), (80, 24)]:
            for no_color in [False, True]:
                with self.subTest(width=width, height=height, no_color=no_color):
                    self.journey(populated, unreadable, width, height, no_color)


if __name__ == '__main__':
    unittest.main()
