"""Recorded #360 replay: quoted IDs apply answers and refinements without retry."""
import fcntl
import json
import os
from pathlib import Path
import select
import struct
import tempfile
import termios
import time

import test_launch as subprocess
from vt_screen import Screen

REPO = Path(__file__).resolve().parents[1]
FIXTURE = REPO / 'tests/fixtures/think-source-ref-360.json'


def prepare(root):
    """Prepare an offline replay, reusable in a real Herdr pane."""
    fixture = json.loads(FIXTURE.read_text())
    (root / 'replay.json').write_text(json.dumps(fixture))
    program = root / 'TEST-pi'
    program.write_text('''#!/usr/bin/env python3
import json, pathlib, sys
root = pathlib.Path(__file__).parent
fixture = json.loads((root / 'replay.json').read_text())
request = json.load(sys.stdin)
with (root / 'requests.jsonl').open('a') as log:
    log.write(json.dumps(request) + '\\n')
if request.get('kind') == 'decision-review':
    print(fixture['reviews'][len(request['words']) + len(request['answers']) - 1])
else:
    replies = fixture['initial_replies'] + fixture['replies']
    print(replies[len(request['sources']) - 1])
''')
    program.chmod(0o755)
    return program, fixture['sources']


def journey(width, height):
    with tempfile.TemporaryDirectory(prefix='tinkery-TEST-source-ref-') as directory:
        root = Path(directory)
        program, sources = prepare(root)
        master, slave = os.openpty()
        fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack('HHHH', height, width, 0, 0))
        screen = Screen(width, height)
        env = subprocess.environment(dict(os.environ, HOME=str(root), TERM='xterm-256color'), home=root)
        process = subprocess.Popen(
            [str(REPO / 'target/debug/tinkery'), '--operator', 'agent', '--shape-pi', '--shape-budget', '64',
             '--model', 'test/recorded-360', '--pi-command', str(program),
             '--seed-session-root', str(root / 'sessions'), '--record-root', str(root / 'records'),
             '--link-root', str(root / 'links')],
            stdin=slave, stdout=slave, stderr=slave, cwd=root, env=env)

        def wait(check):
            deadline = time.monotonic() + 10
            while time.monotonic() < deadline:
                if select.select([master], [], [], .02)[0]:
                    screen.feed(os.read(master, 65536))
                if not screen.synchronized and check(screen.text()):
                    return
                if process.poll() is not None:
                    break
            raise AssertionError(screen.text())

        def requests():
            return [json.loads(line) for line in (root / 'requests.jsonl').read_text().splitlines()]

        try:
            wait(lambda s: "What's on your mind?" in s)
            for i, words in enumerate(sources):
                os.write(master, b'\x1b[200~' + words.encode() + b'\x1b[201~\x1bOQ')
                # Reading-side state proves application, not merely stub invocation.
                expected = ['first playable scope', 'real data', 'aggregated data',
                            'Who should the player', 'What would tell you this worked?'][i]
                wait(lambda s: expected.lower() in s.lower()
                     and (root / 'requests.jsonl').exists()
                     and len(requests()) == (i + 1) * 2
                     and 'Thinking' not in s and 'Reviewing decisions' not in s)
                assert 'Reading failed' not in screen.text(), screen.text()
                shaping = [r for r in requests() if 'kind' not in r]
                assert len(shaping) == i + 1
                assert [s['text'] for s in shaping[-1]['sources']] == sources[:i + 1]
            # Inspect originals through the application's copy surface; never save a goal.
            # F1's copy row copies the provisional board and originals.
            os.write(master, b'\x1bOP')
            wait(lambda s: 'Copy the reading' in s)
            os.write(master, screen.pick('Copy the reading'))
            wait(lambda s: bool(screen.clipboards))
            for words in sources:
                assert words in screen.clipboards[-1], screen.clipboards[-1]
            assert 'MAP' in screen.clipboards[-1]
            assert len(requests()) == 10, 'No manual or automatic retry is needed'
            assert not (root / 'sessions').exists() and not (root / 'records').exists()
            os.write(master, b'\x1b')
            wait(lambda s: 'lose this draft?' in s)
            os.write(master, b'y')
            process.wait(timeout=5)
            assert process.returncode == 0
            print(json.dumps(dict(size=[width, height], sources=5, stub_calls=10,
                                  real_calls=0, retries=0, journey='recorded answer + added dump')))
        finally:
            if process.poll() is None:
                process.kill()
                process.wait()
            os.close(master)
            os.close(slave)


if __name__ == '__main__':
    for size in [(100, 30), (160, 40)]:
        journey(*size)
