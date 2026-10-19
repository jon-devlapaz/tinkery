"""Approve's read-only handoff over official Seed Me TEST fixtures; no provider or real approval."""
import base64
import fcntl
import hashlib
import json
import os
import re
import select
import shlex
import struct
import subprocess
import tempfile
import termios
import time
from pathlib import Path

from shape_terminal import SKILL, make, seed, files
from vt_screen import Screen

REPO = Path(__file__).resolve().parents[1]


def journey(width, height, case, no_color):
    with tempfile.TemporaryDirectory(prefix="tinkery-TEST-approve-") as directory:
        root = Path(directory).resolve()
        sessions = root / "TEST sessions' folder"
        expected = []
        if case in ('ready', 'started', 'edited', 'moved', 'runs-unreadable'):
            for name, date in [('TEST oldest café', '2026-01-01T00:00:00Z'),
                               ('TEST newest', '2026-02-01T00:00:00Z')]:
                session = make(sessions, name, goal=name + '\nTEST original goal.', question=False)
                seed(session, confirm=True)
                ledger = session / 'ledger.json'
                value = json.loads(ledger.read_text())
                value['updated_at'] = date
                ledger.write_text(json.dumps(value))
                expected.append(session)
            draft = make(sessions, 'draft', goal='TEST excluded draft', question=False)
            seed(draft)
            stale = make(sessions, 'stale', goal='TEST excluded stale', question=False)
            seed(stale, confirm=True)
            path = stale / 'seed-contract.md'
            path.write_text(path.read_text() + '\nTEST changed seed\n')
            make(sessions, 'question', goal='TEST excluded question')
        elif case == 'incomplete':
            broken = sessions / 'broken'
            broken.mkdir(parents=True)
            (broken / 'ledger.json').write_text('not JSON')
        elif case == 'unreadable':
            sessions.write_text('TEST not a directory')
        checkout = root / 'TEST-checkout'
        configured = case in ('started', 'edited', 'moved', 'runs-unreadable')
        if configured:
            script = checkout / '_system/scripts/sdlc.py'
            script.parent.mkdir(parents=True)
            original = expected[0] / 'seed-contract.md'
            record = {'path': 'runs/TEST-other/seed-contract.md', 'source': str(original.resolve()),
                      'sha256': hashlib.sha256(original.read_bytes()).hexdigest()}
            report = {'protocol': 'tink-sdlc', 'api_version': 1, 'runs': [
                {'slug': 'TEST-unrelated-name', 'seed_contract': record}]}
            script.write_text('print(' + repr(json.dumps(report)) + ')\n'
                              if case != 'runs-unreadable' else 'raise SystemExit("TEST unreadable runs")\n')
            if case == 'edited':
                original.write_text(original.read_text() + '\nTEST edit after intake\n')
            elif case == 'moved':
                moved = sessions / 'TEST moved session'
                expected[0].rename(moved)
                expected[0] = moved
        host = root / 'pi'
        host.write_text('#!/bin/sh\ntouch TEST-provider-was-run\nexit 86\n')
        host.chmod(0o755)
        before = files(root)
        master, slave = os.openpty()
        fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack('HHHH', height, width, 0, 0))
        env = {**os.environ, 'HOME': str(root), 'TERM': 'xterm-256color',
               'TINKERY_NODE': '/tinkery-TEST-no-node', 'PATH': str(root) + os.pathsep + os.environ['PATH']}
        if no_color:
            env['NO_COLOR'] = '1'
        else:
            env.pop('NO_COLOR', None)
        proc = subprocess.Popen([str(REPO / 'target/debug/tinkery'), '--operator', 'agent', '--home',
            '--seed-me', str(SKILL), '--seed-session-root', str(sessions),
            '--record-root', str(root / 'TEST-records'), '--link-root', str(root / 'TEST-links'),
            *(['--sdlc-root', str(checkout)] if configured else [])], stdin=slave, stdout=slave, stderr=slave, cwd=root, env=env)
        screen = Screen(width, height)
        output = bytearray()

        def flat():
            return ' '.join(screen.text().split())

        def wait(predicate):
            until = time.monotonic() + 15
            while time.monotonic() < until:
                if select.select([master], [], [], .02)[0]:
                    data = os.read(master, 65536)
                    output.extend(data)
                    screen.feed(data)
                if predicate() and not screen.synchronized:
                    return
                if proc.poll() is not None:
                    break
            raise AssertionError(screen.text())

        def send(keys, text):
            os.write(master, keys)
            wait(lambda: text in flat())

        def copies():
            return [base64.b64decode(data).decode() for data in
                    re.findall(rb'\x1b\]52;[^;]*;([^\x07\x1b]*)(?:\x07|\x1b\\)', output)]

        try:
            wait(lambda: '4  approve' in screen.text())
            os.write(master, b'4')
            if case in ('started', 'edited'):
                wait(lambda: 'Seed 1 of 1' in flat())
                assert 'TEST newest' in flat() and 'TEST oldest café' not in flat(), screen.text()
                if case == 'edited':
                    assert 'unknown start' in flat(), screen.text()
                else:
                    assert 'unknown start' not in flat(), screen.text()
            elif case in ('ready', 'moved', 'runs-unreadable'):
                wait(lambda: 'Seed 1 of 2' in flat())
                if case != 'moved':
                    assert "Can't tell which seeds have started; none are hidden." in flat(), screen.text()
                assert 'TEST oldest café' in flat() and 'TEST newest' not in flat(), screen.text()
                assert 'TEST original goal.' in flat(), screen.text()
                acceptance = (expected[0] / 'seed-contract.md').read_text().split('## Acceptance criteria\n', 1)[1].split('\n## ', 1)[0]
                for line in acceptance.splitlines():
                    if line.strip():
                        assert ' '.join(line.split()) in flat(), (line, screen.text())
                assert not copies()
                send(b'c', 'Copied.')
                wait(lambda: len(copies()) == 1)
                commands = copies()[0].splitlines()
                assert commands[0] == 'python3 _system/scripts/sdlc.py new <run>'
                assert shlex.split(commands[1]) == ['python3', '_system/scripts/sdlc.py', 'stage', '<run>', '1',
                    '--seed-contract', str((expected[0] / 'seed-contract.md').resolve()),
                    '--seed-me-helper', str((SKILL.parent / 'scripts/session.py').resolve())]
                send(b'\x1b[F', '--seed-me-helper')
                assert 'esc back' in flat()
                send(b'?', 'Right now')
                os.write(master, b'cnp')
                send(b'\x1b', '--seed-me-helper')
                assert len(copies()) == 1, 'help leaked a copy key'
                send(b'n', 'TEST newest')
                send(b'p', 'TEST oldest café')
                send(b'\x1b', '4 approve')
                send(b'4', 'Seed 1 of 2')
            else:
                message = {'empty': 'No verified seeds could be listed.',
                           'incomplete': 'No verified seeds could be listed.',
                           'unreadable': 'Ready seeds unavailable.'}[case]
                wait(lambda: message in flat())
                if case == 'incomplete':
                    assert 'may be incomplete' in flat()
                    send(b'?', 'broken')
                    send(b'\x1b', message)
                os.write(master, b'c')
                send(b'\x1b', '4 approve')
                assert not copies()
            assert files(root) == before, 'Approve changed TEST files'
            assert not (root / 'TEST-provider-was-run').exists()
            print(json.dumps(dict(width=width, height=height, no_color=no_color,
                journey='approve/' + case, provider_calls=0, session_writes=0)))
        finally:
            proc.kill()
            proc.wait()
            os.close(master)
            os.close(slave)


if __name__ == '__main__':
    for size in [(80, 24), (100, 30)]:
        for no_color in [False, True]:
            for case in ['ready', 'empty', 'incomplete', 'unreadable', 'started', 'edited', 'moved', 'runs-unreadable']:
                journey(*size, case, no_color)
