"""Repo-first Tend and cross-repo needs; synthetic TEST records, no provider calls."""
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

from vt_screen import Screen

REPO = Path(__file__).resolve().parents[1]
SKILL = Path(os.environ['SEED_ME_TEST_SKILL']).resolve()
KEYS = '↑↓←→ move   enter open   l link   esc back'
LABELS = ['TEST A goal', 'TEST B goal', 'TEST unknown goal', 'TEST legacy goal', 'TEST outside goal']


class Terminal:
    def __init__(self, root, cwd, width, height, no_color):
        self.master, self.slave = os.openpty()
        fcntl.ioctl(self.slave, termios.TIOCSWINSZ, struct.pack('HHHH', height, width, 0, 0))
        self.screen = Screen(width, height)
        env = {**os.environ, 'TERM': 'xterm-256color', 'TINKERY_NODE': '/tinkery-TEST-no-node'}
        env.pop('NO_COLOR', None)
        if no_color:
            env['NO_COLOR'] = '1'
        self.process = subprocess.Popen([str(REPO / 'target/debug/tinkery'), '--home',
            '--shape-pi', '--model', 'test/model', '--seed-me', str(SKILL),
            '--pi-command', str(root / 'TEST-never-pi'),
            '--seed-session-root', str(root / 'TEST-sessions'),
            '--record-root', str(root / 'TEST-records'), '--link-root', str(root / 'TEST-links')],
            cwd=cwd, stdin=self.slave, stdout=self.slave, stderr=self.slave, env=env)

    def wait(self, predicate):
        deadline = time.monotonic() + 10
        while time.monotonic() < deadline:
            if select.select([self.master], [], [], .02)[0]:
                self.screen.feed(os.read(self.master, 65536))
            if not self.screen.synchronized and predicate(self.screen.text()):
                return self.screen.text()
            if self.process.poll() is not None:
                break
        raise AssertionError(self.screen.text())

    def wait_exit(self):
        deadline = time.monotonic() + 10
        while True:
            returncode = self.process.poll()
            if returncode is not None:
                return returncode
            remaining = deadline - time.monotonic()
            if remaining <= 0:
                raise AssertionError('Process did not exit within 10 seconds:\n' + self.screen.text())
            if select.select([self.master], [], [], min(.02, remaining))[0]:
                self.screen.feed(os.read(self.master, 65536))

    def send(self, key, text):
        os.write(self.master, key)
        return self.wait(lambda screen: text in screen)

    def selected(self, label):
        return self.wait(lambda screen: screen.splitlines()[-3].startswith('  ' + label))

    def close(self):
        if self.process.poll() is None:
            self.process.kill()
            self.process.wait()
        os.close(self.master)
        os.close(self.slave)


def assert_incomplete(screen, unreadable):
    text = ' '.join(line.strip() for line in screen.splitlines())
    records = 'record' if unreadable == 1 else 'records'
    assert 'No active goals' not in text, screen
    assert text.count(f"{unreadable} saved {records} couldn't be read") == 1, screen
    assert text.count('1 ended not shown') == 1, screen


def journey(width, height, no_color, place, unreadable=1):
    incomplete = place.startswith('incomplete')
    outside = place in ('outside', 'incomplete-outside')
    with tempfile.TemporaryDirectory(prefix='tinkery-TEST-repo-tend-') as directory:
        root = Path(directory).resolve()
        for name in ['TEST-repo-a', 'TEST-repo-b', 'TEST-repo-empty']:
            (root / name / '.git').mkdir(parents=True)
        (root / 'TEST-records').mkdir()
        for index, (name, label) in enumerate(zip(['a', 'b', 'unknown', 'legacy', 'outside'], LABELS)):
            if incomplete and name not in (('a', 'b') if place == 'incomplete' else ('a',)):
                continue
            path = root / 'TEST-sessions' / name
            path.mkdir(parents=True)
            (path / 'ledger.json').write_text(json.dumps(dict(schema_version=1, session_id=name,
                status='completed' if incomplete and name == 'a' else 'active', goal=label, updated_at=f'2026-10-17T12:00:0{5-index}Z')))
            if name == 'unknown':
                continue
            record = dict(schema=2 if name == 'legacy' else 3, tinkery='TEST', origin='simulated',
                model=None, thinking=None, prompt_hash=None, started='2026-10-17T12:00:00Z',
                confirmed='2026-10-17T12:00:00Z', seed_me_session=name,
                seed_me_session_path=str(path), seed_me_session_id=name, turns=[], checks=[], goal=label)
            if name != 'legacy':
                record['repo'] = str(root / ('TEST-repo-a' if name == 'a' else 'TEST-repo-b')) if name != 'outside' else None
            (root / 'TEST-records' / f'{name}.json').write_text(json.dumps(record))
        if incomplete:
            for index in range(unreadable):
                broken = root / 'TEST-sessions' / f'TEST-unreadable-{index}'
                broken.mkdir()
                (broken / 'ledger.json').write_text('not JSON')
        if place == 'broken':
            (root / 'TEST-records' / 'TEST-broken.json').write_text('not JSON')
        host = root / 'TEST-never-pi'
        host.write_text('#!/bin/sh\ntouch "' + str(root / 'TEST-provider-ran') + '"\nexit 86\n')
        host.chmod(0o755)
        before = {str(p): p.read_bytes() for folder in ['TEST-sessions', 'TEST-records']
                  for p in (root / folder).rglob('*') if p.is_file()}
        cwd = root / {'inside': 'TEST-repo-a', 'broken': 'TEST-repo-a', 'empty': 'TEST-repo-empty', 'incomplete': 'TEST-repo-a', 'incomplete-all': 'TEST-repo-a'}.get(place, '.')
        terminal = Terminal(root, cwd, width, height, no_color)
        try:
            terminal.wait(lambda screen: '1  think' in screen)
            terminal.send(b'2', 'this repo · a all' if not outside else 'esc back')
            if incomplete:
                screen = terminal.wait(lambda screen: 'saved record' in screen)
                assert_incomplete(screen, unreadable)
                if not outside:
                    assert 'Press a for all goals.' in ' '.join(line.strip() for line in screen.splitlines()), screen
                    screen = terminal.send(b'a', 'all · a this repo')
                    if place == 'incomplete':
                        terminal.selected(LABELS[1])
                    else:
                        assert_incomplete(screen, unreadable)
                    screen = terminal.send(b'a', 'this repo · a all')
                    assert_incomplete(screen, unreadable)
            elif place in ('empty', 'broken'):
                empty_message = f'{5 if place == "broken" else 2} goals have no readable repo record.'
                screen = terminal.wait(lambda screen: empty_message in screen)
                assert 'No active goals' not in screen, screen
                terminal.send(b'a', 'all · a this repo')
            elif place == 'inside':
                screen = terminal.selected(LABELS[0])
                assert all(label not in screen for label in LABELS[1:]), screen
                os.write(terminal.master, b'j')
                terminal.selected(LABELS[0])
                # The scope row must not shift pointer hits away from the painted label.
                row, line = next((row, line) for row, line in enumerate(screen.splitlines()[4:-4], 4)
                                 if LABELS[0] in line)
                column = line.index(LABELS[0]) + 2
                click = (f'\x1b[<0;{column+1};{row+1}M\x1b[<0;{column+1};{row+1}m').encode()
                os.write(terminal.master, click * 2)
                terminal.wait(lambda screen: 'r history' in screen)
                terminal.send(b'\x1b', 'this repo · a all')
                terminal.send(b'a', 'all · a this repo')
            else:
                assert 'this repo' not in terminal.screen.text()
                os.write(terminal.master, b'a')
            if incomplete:
                assert not (root / 'TEST-provider-ran').exists()
                assert not (root / 'TEST-links').exists()
                assert {str(p): p.read_bytes() for folder in ['TEST-sessions', 'TEST-records']
                        for p in (root / folder).rglob('*') if p.is_file()} == before
                terminal.send(b'\x1b', '1  think')
                os.write(terminal.master, b'q')
                terminal.wait_exit()
                assert terminal.process.returncode == 0
                print(json.dumps(dict(width=width, height=height, no_color=no_color,
                    journey='repo-scope/' + place, unreadable=unreadable, provider_calls=0, fixtures='temporary TEST roots')))
                return
            terminal.selected(LABELS[0])
            # Navigate every goal rather than requiring all labels to fit at once.
            for label in LABELS[1:]:
                os.write(terminal.master, b'j')
                terminal.selected(label)
            os.write(terminal.master, b'j')
            terminal.selected(LABELS[-1])
            for label in reversed(LABELS[:-1]):
                os.write(terminal.master, b'k')
                terminal.selected(label)
            if place == 'inside':
                terminal.send(b'l', 'enter link')
                terminal.send(b'j', 'TEST A goal needs TEST B goal')
                os.write(terminal.master, b'a')  # Linking keeps its exact source/target.
                terminal.send(b'\r', KEYS)
                terminal.wait(lambda screen: 'needs TEST B goal · not ready' in screen)
                links = root / 'TEST-links' / 'needs.json'
                expected = dict(version=1, needs=[dict(goal='a', needs='b')])
                assert json.loads(links.read_text()) == expected
                saved_links = links.read_bytes()
                terminal.send(b'a', 'this repo · a all')
                screen = terminal.selected(LABELS[0])
                assert 'needs TEST B goal · not ready' in screen, screen
                assert LABELS[1] not in '\n'.join(screen.splitlines()[4:-4]), screen
                assert all(label not in screen for label in LABELS[2:]), screen
                # Detail and help do not toggle; Shape's s remains a separate action.
                terminal.send(b'\r', 'r history')
                os.write(terminal.master, b'a')
                terminal.send(b'\x1b', 'this repo · a all')
                terminal.send(b'\x1bOP', 'Right now')
                os.write(terminal.master, b'a')
                terminal.send(b'\x1b', 'this repo · a all')
                assert links.read_bytes() == saved_links
            elif place in ('empty', 'broken'):
                terminal.send(b'a', empty_message)
            assert not (root / 'TEST-provider-ran').exists()
            assert {str(p): p.read_bytes() for folder in ['TEST-sessions', 'TEST-records']
                    for p in (root / folder).rglob('*') if p.is_file()} == before
            if place != 'inside':
                assert not (root / 'TEST-links').exists()
            terminal.send(b'\x1b', '1  think')
            os.write(terminal.master, b'q')
            terminal.wait_exit()
            assert terminal.process.returncode == 0
            print(json.dumps(dict(width=width, height=height, no_color=no_color,
                journey='repo-scope/' + place, provider_calls=0, fixtures='temporary TEST roots')))
        finally:
            terminal.close()


if __name__ == '__main__':
    for size in [(80, 24), (100, 30)]:
        for no_color in [False, True]:
            for place in ['incomplete-outside', 'incomplete-all']:
                for unreadable in [1, 2]:
                    journey(*size, no_color, place, unreadable)
            for place in ['incomplete', 'inside', 'outside', 'empty', 'broken']:
                journey(*size, no_color, place)
