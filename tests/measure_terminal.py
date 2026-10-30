"""Measure outside a repo: navigation and help never run external commands."""
import json
import os
from pathlib import Path
import select
import test_launch as subprocess
import tempfile
import time

from terminal_process import wait_output, TerminalProcess

REPO = Path(__file__).resolve().parents[1]
UNKNOWN = 'No launch repository.'
SIZES = [(80, 24), (100, 30), (160, 40), (320, 40)]


class Terminal(TerminalProcess):
    def __init__(self, width, height, no_color, gh_script=None):
        self.temporary = tempfile.TemporaryDirectory(prefix='tinkery-TEST-measure-')
        self.root = Path(self.temporary.name).resolve()
        home, binary = (self.root / name for name in ('HOME', 'bin'))
        home.mkdir()
        binary.mkdir()
        self.calls = self.root / 'calls'
        # Fail any retired reader or provider launch, even if its output is ignored.
        for name in ('python3', 'pi', 'node', 'gh'):
            command = binary / name
            command.write_text(f'#!/bin/sh\nprintf "%s\\n" "$0" >> "{self.calls}"\nexit 99\n')
            command.chmod(0o755)
        if gh_script is not None:
            (self.root / '.git').mkdir()
            (binary / 'gh').write_text(gh_script)
        self.before = self.files()
        env = {'PATH': str(binary), 'TERM': 'xterm-256color',
               'TINKERY_NODE': str(binary / 'node')}
        if gh_script is not None:
            env['GH_REPO'] = 'TEST/wrong-repository'
        if no_color:
            env['NO_COLOR'] = '1'
        super().__init__(
            [str(REPO / 'target/debug/tinkery'), '--operator', 'agent', '--home',
             '--seed-session-root', str(self.root / 'TEST-sessions')],
            cwd=self.root, env=subprocess.environment(env, home=home),
            width=width, height=height)
        self.wait(lambda: '1  think' in self.screen.text())

    def files(self):
        return {str(p.relative_to(self.root)): p.read_bytes()
                for p in self.root.rglob('*') if p.is_file()}

    def wait(self, predicate, seconds=5):
        deadline = time.monotonic() + seconds
        def consume_wait_output(chunk):
            self.raw.extend(chunk)
            self.screen.feed(chunk)
        for returncode in wait_output(self.master, self.process, consume_wait_output, deadline):
            if predicate() and not self.screen.synchronized:
                return
            if returncode is not None:
                break
        raise AssertionError(self.screen.text())

    def send(self, key, expected):
        os.write(self.master, key)
        self.wait(lambda: expected in self.screen.text())

    def finish(self, key=b'q', expected_calls=0):
        os.write(self.master, key)
        self.process.wait(timeout=5)
        assert self.process.returncode == 0
        files = self.files()
        if expected_calls:
            calls = files.pop('calls').decode().splitlines()
            assert len(calls) == expected_calls, calls
            assert all(json.loads(call)['args'][:2] == ['pr', 'list'] for call in calls)
        else:
            assert not self.calls.exists(), 'Measure launched an external command'
        assert files == self.before, 'Navigation wrote durable data'
        assert not (self.root / 'TEST-sessions').exists()

    def close(self):
        super().close()
        self.temporary.cleanup()


def journey(width, height, no_color):
    terminal = Terminal(width, height, no_color)
    try:
        terminal.send(b'7', UNKNOWN)
        text = terminal.screen.text()
        assert 'esc back   ? help' in text, text
        assert 'unavailable' not in text and 'Reading' not in text, text
        terminal.send(b'?', 'q goes back to the menu')
        assert 'q quits' not in terminal.screen.text()
        terminal.send(b'?', UNKNOWN)
        terminal.send(b'\x1bOP', "Measure — this repo's pull requests")
        terminal.send(b'\x1b', UNKNOWN)
        terminal.send(b'\x1b', '1  think')
        terminal.send(b'7', UNKNOWN)
        terminal.send(b'?', "Measure — this repo's pull requests")
        terminal.send(b'q', '1  think')
        assert '? help   q quit' in terminal.screen.text()
        terminal.finish()
        print(json.dumps(dict(width=width, height=height, no_color=no_color,
                              state='unknown', provider_calls=0, command_calls=0)))
    finally:
        terminal.close()


if __name__ == '__main__':
    for dimensions in SIZES:
        for no_color in (False, True):
            journey(*dimensions, no_color)
