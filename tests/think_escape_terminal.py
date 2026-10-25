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
ORIGINAL = 'TEST original café 👩‍💻\r\nKeep these exact words.'
DRAFT = 'TEST unsent answer 中\r\nStill local.'
LOSS = 'Back to menu and lose this draft? y / n'


def journey(width, height):
    with tempfile.TemporaryDirectory(prefix='tinkery-TEST-escape-') as directory:
        root = Path(directory)
        master, slave = os.openpty()
        fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack('HHHH', height, width, 0, 0))
        screen = Screen(width, height)
        process = subprocess.Popen(
            [str(BINARY), '--operator', 'agent', '--home'],
            stdin=slave, stdout=slave, stderr=slave, cwd=root,
            env=subprocess.environment({**os.environ, 'TERM': 'xterm-256color'}, home=root),
        )

        def pump(seconds=.2):
            deadline = time.monotonic() + seconds
            while time.monotonic() < deadline:
                if select.select([master], [], [], .02)[0]:
                    screen.feed(os.read(master, 65536))

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

        def copy(expected):
            before = len(screen.clipboards)
            os.write(master, b'\x03')
            wait(lambda _: len(screen.clipboards) > before)
            assert screen.clipboards[before:] == [expected], screen.clipboards

        def escape_without_leaving():
            before = list(screen.clipboards)
            os.write(master, b'\x1b')
            pump()
            assert LOSS not in screen.text() and process.poll() is None, screen.text()
            assert screen.clipboards == before, 'Escape copied text'

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
            send(b'\x1b[200~' + ORIGINAL.encode() + b'\x1b[201~', 'Keep these exact words.')
            send(b'\x1bOQ', 'which part matters')
            send(b'\x1b[200~' + DRAFT.encode() + b'\x1b[201~', 'Still local.')
            os.write(master, b'\t\x01')
            copy(ORIGINAL)
            escape_without_leaving()
            before = list(screen.clipboards)
            os.write(master, b'\x03')
            pump()
            assert screen.clipboards == before, 'Escape left a stale selection'
            os.write(master, b'\x01')
            copy(ORIGINAL)  # First Escape cleared the selection but kept original focus.
            escape_without_leaving()
            escape_without_leaving()
            os.write(master, b'\x01')
            copy(DRAFT)  # Next Escape returned to writing without losing unsent bytes.
            send(b'\x1b', LOSS)
            os.write(master, b'\x1b')
            wait(lambda s: LOSS not in s)
            copy(DRAFT)
            # With no original selection, one Escape returns directly to writing.
            os.write(master, b'\t')
            pump()
            escape_without_leaving()
            os.write(master, b'\x01')
            copy(DRAFT)
            send(b'\x1b', LOSS)
            os.write(master, b'n')
            wait(lambda s: LOSS not in s)
            copy(DRAFT)
            os.write(master, b'\t\x01')
            copy(ORIGINAL)
            escape_without_leaving()
            escape_without_leaving()
            send(b'\x1b', LOSS)
            send(b'y', '1  think')
            send(b'1', "What's on your mind?")
            assert 'Still local.' not in screen.text() and 'TEST original' not in screen.text()
            send(b'\x1b', '1  think')
            os.write(master, b'q')
            process.wait(timeout=5)
            assert process.returncode == 0
            assert not (root / '.local/share/tinkery/sessions').exists(), 'Escape saved a goal'
            assert not (root / '.local/share/tinkery/think').exists(), 'Escape wrote a record'
            print(json.dumps(dict(width=width, height=height, provider='none',
                                  journey='original selection/clear/writing/guard/cancel/discard/reopen',
                                  original_bytes=len(ORIGINAL.encode()), draft_bytes=len(DRAFT.encode()))))
        finally:
            if process.poll() is None:
                process.kill()
                process.wait()
            os.close(master)
            os.close(slave)


for size in [(80, 24), (100, 30), (160, 40), (320, 40)]:
    journey(*size)
