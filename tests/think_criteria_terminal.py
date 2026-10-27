"""Fake-Pi PTY regressions: Why and copy distinguish experience from success criteria."""
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
from think_question_terminal import prepare
from vt_screen import Screen

BINARY = Path(__file__).resolve().parents[1] / 'target/debug/tinkery'
WORDS = ('TEST city healthcare game with the approachable feel of RollerCoaster Tycoon. '
         'Keep clinics, hospitals, providers and budgeting.')
CRITERIA = 'I can allocate funds across providers.'


def prepare_criteria(root, criteria):
    """Create a fake replay with experience always supplied and optional Done when."""
    program, _ = prepare(root)
    reply = dict(parts=dict(who='I', outcome='manage citywide healthcare with the approachable '
                            'feel of RollerCoaster Tycoon', why='Needs and budgets interact',
                            done_when=criteria))
    (root / 'reply.json').write_text(json.dumps(reply))
    return program


def journey(width, height, criteria, no_color):
    with tempfile.TemporaryDirectory(prefix='tinkery-TEST-criteria-') as directory:
        root = Path(directory)
        program = prepare_criteria(root, criteria)
        master, slave = os.openpty()
        fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack('HHHH', height, width, 0, 0))
        screen = Screen(width, height)
        env = subprocess.environment(dict(os.environ, HOME=str(root), TERM='xterm-256color'), home=root)
        args = [str(BINARY), '--operator', 'agent', '--shape-pi', '--shape-budget', '64', '--model', 'test/replay',
                '--pi-command', str(program), '--seed-session-root', str(root / 'sessions'),
                '--record-root', str(root / 'records'), '--link-root', str(root / 'links')]
        process = subprocess.Popen(args + (['--no-color'] if no_color else []),
                                   stdin=slave, stdout=slave, stderr=slave, cwd=root, env=env)

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

        try:
            wait(lambda s: "What's on your mind?" in s)
            os.write(master, b'\x1b[200~' + WORDS.encode() + b'\x1b[201~\x1bOQ')
            wait(lambda s: ("Ready for your review." if criteria else "Still missing: Done when.")
                 in ' '.join(s.split()))
            os.write(master, b'UNSENT ANSWER\x1bOP')
            wait(lambda s: 'Why: attempts and checks' in s)
            os.write(master, screen.pick('Why: attempts and checks'))
            wait(lambda s: 'Success criteria' in s and 'F2 send' in s)
            right = ' '.join(' '.join(line[width * 48 // 100 + 2:]
                                     for line in screen.text().splitlines()).split())
            assert f'Success criteria {criteria or "Not yet captured"}' in right, screen.text()
            assert 'Desired experience' not in right and 'Not supplied' not in right
            os.write(master, b'\x1b')
            wait(lambda s: 'Success criteria' not in s and 'UNSENT ANSWER' in s)
            os.write(master, b'\x1bOP')
            wait(lambda s: 'Copy the reading' in s)
            os.write(master, screen.pick('Copy the reading'))
            wait(lambda s: bool(screen.clipboards))
            copied = screen.clipboards[-1]
            assert f'## Success criteria / proposed\n\n{criteria or "Not yet captured"}' in copied
            assert WORDS in copied and 'RollerCoaster Tycoon' in copied
            assert 'Desired experience' not in copied and 'Not supplied' not in copied
            assert 'UNSENT ANSWER' in screen.text()
            assert (root / 'calls').read_text() == 'shape\ndecision-review\n'
            assert not (root / 'sessions').exists() and not (root / 'records').exists()
            os.write(master, b'\x1b')
            wait(lambda s: 'lose this draft?' in s)
            os.write(master, b'y')
            process.wait(timeout=5)
            assert process.returncode == 0
            print(json.dumps(dict(width=width, height=height, criteria_present=criteria is not None,
                                  no_color=no_color, provider='fake; zero real calls',
                                  journey='F2/F1 Why/Esc/F1 copy captured OSC52/unsent answer/no save')))
        finally:
            if process.poll() is None:
                process.kill()
                process.wait()
            os.close(master)
            os.close(slave)


if __name__ == '__main__':
    for size in [(80, 24), (100, 30)]:
        for criteria in [None, CRITERIA]:
            for no_color in [False, True]:
                journey(*size, criteria, no_color)
