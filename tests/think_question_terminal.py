"""Zero-provider PTY replay: active questions stay readable without goal review."""
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

BINARY = Path(__file__).resolve().parents[1] / 'target/debug/tinkery'
QUESTION = ('What would make this loop satisfying to you: finding a workable balance for '
            'different patient populations, understanding consequences across the network, '
            'making difficult funding tradeoffs, or some combination?')
WORDS = ('TEST city healthcare system: clinics, hospitals, administration, patient populations, '
         'providers and budgeting. Keep citywide management rather than one clinic.')


def prepare(root, long_question=False):
    """Write a clearly fake Pi replay, also usable for an isolated visual dogfood."""
    question = ('Which population needs care? ' * 40 + 'Last question words?'
                if long_question else QUESTION)
    reply = dict(parts=dict(who='I', outcome='manage a city healthcare system',
                            why='Population needs and limited budgets interact',
                            must=['preserve citywide clinics, hospitals and providers'] * 12),
                 questions=[dict(id='care', text=question)])
    (root / 'reply.json').write_text(json.dumps(reply))
    program = root / 'TEST-pi'
    program.write_text('''#!/usr/bin/env python3
import json, pathlib, sys
root = pathlib.Path(__file__).parent
request = json.load(sys.stdin)
kind = request.get('kind', 'shape')
with (root / 'calls').open('a') as log:
    log.write(kind + '\\n')
print('{"decisions":[]}' if kind == 'decision-review' else (root / 'reply.json').read_text())
''')
    program.chmod(0o755)
    return program, question


def journey(width, height, long_question=False):
    with tempfile.TemporaryDirectory(prefix='tinkery-TEST-question-') as directory:
        root = Path(directory)
        program, question = prepare(root, long_question)
        master, slave = os.openpty()
        fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack('HHHH', height, width, 0, 0))
        screen = Screen(width, height)
        env = subprocess.environment(dict(os.environ, HOME=str(root), TERM='xterm-256color'), home=root)
        process = subprocess.Popen(
            [str(BINARY), '--operator', 'agent', '--shape-pi', '--model', 'test/replay',
             '--pi-command', str(program), '--seed-session-root', str(root / 'sessions'),
             '--record-root', str(root / 'records'), '--link-root', str(root / 'links')],
            stdin=slave, stdout=slave, stderr=slave, cwd=root, env=env,
        )

        def wait(check):
            deadline = time.monotonic() + 8
            while time.monotonic() < deadline:
                if select.select([master], [], [], .02)[0]:
                    screen.feed(os.read(master, 65536))
                if not screen.synchronized and check(screen.text()):
                    return
                if process.poll() is not None:
                    break
            raise AssertionError(screen.text())

        def right():
            return ' '.join(' '.join(line[width * 48 // 100 + 2:]
                                    for line in screen.text().splitlines()).split())

        try:
            wait(lambda s: "What's on your mind?" in s)
            os.write(master, b'\x1b[200~' + WORDS.encode() + b'\x1b[201~\x1bOQ')
            wait(lambda s: "One thing I'm not sure about" in s)
            if long_question and height <= 30:
                assert 'PgUp/PgDn scroll question' in screen.text(), screen.text()
            else:
                assert question in right(), screen.text()
            os.write(master, b'UNSENT ANSWER')
            wait(lambda s: 'UNSENT ANSWER' in s)
            seen = right()
            for _ in range(35):
                os.write(master, b'\x1b[6~')
                # Drain at least one complete repaint before sampling the viewport.
                until = time.monotonic() + .04
                while time.monotonic() < until:
                    if select.select([master], [], [], .01)[0]:
                        screen.feed(os.read(master, 65536))
                seen += ' ' + right()
            for words in question.split():
                assert words in seen, (words, screen.text())
            assert ('Last question words?' if long_question else 'tradeoffs, or some combination?') in right()
            assert 'UNSENT ANSWER' in screen.text()
            assert 'You are confirming' not in screen.text()
            assert (root / 'calls').read_text() == 'shape\ndecision-review\n'
            assert not (root / 'sessions').exists() and not (root / 'records').exists()
            os.write(master, b'\x1b[21~')
            wait(lambda s: 'lose this draft?' in s)
            os.write(master, b'y')
            process.wait(timeout=5)
            assert process.returncode == 0
            print(json.dumps(dict(width=width, height=height, provider='fake replay; zero real calls',
                                  long_question=long_question, journey='F2/question/PgDn/unsent input/exit without save')))
        finally:
            if process.poll() is None:
                process.kill()
                process.wait()
            os.close(master)
            os.close(slave)


if __name__ == '__main__':
    for size in [(80, 24), (100, 30), (82, 51)]:
        for long_question in [False, True]:
            journey(*size, long_question)
