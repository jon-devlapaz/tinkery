"""Practice-only PTY regression: Escape returns to menu without implicit saving."""
import fcntl
import json
import os
from pathlib import Path
import select
import struct
import test_launch as subprocess
import tempfile
import termios
import time

from vt_screen import Screen

BINARY = Path(__file__).resolve().parents[1] / 'target/debug/tinkery'


def journey(width, height):
    with tempfile.TemporaryDirectory(prefix='tinkery-TEST-escape-') as directory:
        root = Path(directory)
        master, slave = os.openpty()
        fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack('HHHH', height, width, 0, 0))
        screen = Screen(width, height)
        process = subprocess.Popen(
            [str(BINARY), '--home'],
            stdin=slave, stdout=slave, stderr=slave, cwd=root,
            env=subprocess.environment({**os.environ, 'HOME': str(root), 'TERM': 'xterm-256color'}, home=({**os.environ, 'HOME': str(root), 'TERM': 'xterm-256color'})['HOME']),
        )

        def wait(check):
            deadline = time.monotonic() + 6
            while time.monotonic() < deadline:
                if select.select([master], [], [], .02)[0]:
                    screen.feed(os.read(master, 65536))
                if not screen.synchronized and check(screen.text()):
                    return
                if process.poll() is not None:
                    break
            raise AssertionError(screen.text())

        def send(keys, text):
            os.write(master, keys)
            wait(lambda s: text in s)

        try:
            wait(lambda s: '1  think' in s)
            send(b'1', "What's on your mind?")
            send(b'\x1b', '1  think')
            assert process.poll() is None, 'Escape quit instead of returning to menu'
            send(b'1', "What's on your mind?")
            send(b'Keep my words.', 'Keep my words.')
            send(b'\x1b', 'Back to menu and lose this draft? y / n')
            os.write(master, b'\x1b')
            wait(lambda s: 'lose this draft?' not in s and 'Keep my words.' in s)
            send(b'\x1b', 'Back to menu and lose this draft? y / n')
            send(b'y', '1  think')
            send(b'1', "What's on your mind?")
            assert 'Keep my words.' not in screen.text(), 'New visit retained discarded words'
            send(b'\x1b', '1  think')
            os.write(master, b'q')
            process.wait(timeout=5)
            assert process.returncode == 0
            assert not (root / '.local/share/tinkery/sessions').exists(), 'Escape saved a goal'
            assert not (root / '.local/share/tinkery/think').exists(), 'Escape wrote a record'
            print(json.dumps(dict(width=width, height=height, provider='none',
                                  journey='home/think/Escape/reopen/unsaved guard/decline/discard/reopen')))
        finally:
            if process.poll() is None:
                process.kill()
                process.wait()
            os.close(master)
            os.close(slave)


for size in [(80, 24), (100, 30), (160, 40), (320, 40)]:
    journey(*size)
