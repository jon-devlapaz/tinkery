"""Zero-provider PTY replay: active questions stay readable without goal review."""
from terminal_process import wait_output
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


def prepare(root, long_question=False, suggestion=False):
    """Write a clearly fake Pi replay, also usable for an isolated visual dogfood."""
    question = ('Which population needs care? ' * 40 + 'Last question words?'
                if long_question else QUESTION)
    reply = dict(parts=dict(who='I', outcome='manage a city healthcare system',
                            why='Population needs and limited budgets interact',
                            must=['preserve citywide clinics, hospitals and providers'] * 12),
                 questions=[dict(id='care', text=question)])
    if suggestion:
        reply['questions'][0]['suggestion'] = dict(answer='Start with clinics.', reason='Your words include clinics; starting there is a proposal.')
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


def journey(width, height, long_question=False, suggestion=False, mono=False):
    with tempfile.TemporaryDirectory(prefix='tinkery-TEST-question-') as directory:
        root = Path(directory)
        program, question = prepare(root, long_question, suggestion)
        master, slave = os.openpty()
        fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack('HHHH', height, width, 0, 0))
        screen = Screen(width, height)
        env = subprocess.environment(dict(os.environ, HOME=str(root), TERM='xterm-256color'), home=root)
        if mono:
            env['NO_COLOR'] = '1'
        process = subprocess.Popen(
            [str(BINARY), '--operator', 'agent', '--shape-pi', '--shape-budget', '64', '--model', 'test/replay',
             '--pi-command', str(program), '--seed-session-root', str(root / 'sessions'),
             '--record-root', str(root / 'records'), '--link-root', str(root / 'links')],
            stdin=slave, stdout=slave, stderr=slave, cwd=root, env=env,
        )

        def wait(check):
            deadline = time.monotonic() + 8
            for returncode in wait_output(master, process, screen.feed, deadline):
                if not screen.synchronized and check(screen.text()):
                    return
                if returncode is not None:
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
            elif 'PgUp/PgDn scroll question' not in screen.text():
                assert question in right(), screen.text()
            if suggestion:
                # F1's row copies the suggestion into the draft; no send.
                os.write(master, b'\x1bOP')
                wait(lambda s: 'Use the suggested answer' in s)
                os.write(master, screen.pick('Use the suggested answer'))
                wait(lambda s: 'Start with clinics.' in s and 'Close help' not in s)
                assert (root / 'calls').read_text() == 'shape\ndecision-review\n'
                os.write(master, b'\x15')  # Clear the copied draft, not the question.
                os.write(master, b'\x1bOP')
                wait(lambda s: 'Use the suggested answer' in s)
                os.write(master, b'\x1b')
                wait(lambda s: 'Close help' not in s)
            os.write(master, b'UNSENT ANSWER')
            wait(lambda s: 'UNSENT ANSWER' in s)
            if suggestion:
                # A typed draft is kept: F1 no longer offers to replace it.
                os.write(master, b'\x1bOP')
                wait(lambda s: 'Close help' in s)
                assert 'Use the suggested' not in screen.text(), screen.text()
                os.write(master, b'\x1b')
                wait(lambda s: 'Close help' not in s)
                assert 'UNSENT ANSWER' in screen.text()
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
            assert ('Last question words?' if long_question else 'tradeoffs, or some combination?') in seen
            if suggestion:
                assert 'Suggested answer: Start with clinics.' in seen
                assert 'starting there is a proposal.' in seen
            assert 'UNSENT ANSWER' in screen.text()
            assert 'You are confirming' not in screen.text()
            assert (root / 'calls').read_text() == 'shape\ndecision-review\n'
            assert not (root / 'sessions').exists() and not (root / 'records').exists()
            os.write(master, b'\x1b')
            wait(lambda s: 'lose this draft?' in s)
            os.write(master, b'y')
            process.wait(timeout=5)
            assert process.returncode == 0
            print(json.dumps(dict(width=width, height=height, provider='fake replay; zero real calls',
                                  long_question=long_question, suggestion=suggestion, no_color=mono, journey='F2/question/PgDn/unsent input/exit without save')))
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

    for size in [(80, 24), (100, 30), (160, 40)]:
        for mono in [False, True]:
            journey(*size, suggestion=True, mono=mono)
