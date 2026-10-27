"""Practice-only PTY journey: Ctrl-T and the header label flip light and dark on every screen,
typed Think words stay exact, every launch starts light, and NO_COLOR keeps the monochrome screen."""
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
CHARCOAL = b'48;2;38;35;32'
PAPER = b'48;2;255;252;240'
CTRL_T = b'\x14'
PLACES = ['think', 'tend', 'shape', 'approve', 'watch', 'taste', 'measure']


def journey(width, height, no_color=False):
    with tempfile.TemporaryDirectory(prefix='tinkery-TEST-theme-') as directory:
        root = Path(directory)
        master, slave = os.openpty()
        fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack('HHHH', height, width, 0, 0))
        screen = Screen(width, height)
        raw = bytearray()
        env = {**os.environ, 'TERM': 'xterm-256color'}
        env.pop('NO_COLOR', None)
        if no_color:
            env['NO_COLOR'] = '1'
        process = subprocess.Popen(
            [str(BINARY), '--operator', 'agent', '--home'],
            stdin=slave, stdout=slave, stderr=slave, cwd=root,
            env=subprocess.environment(env, home=root),
        )

        def read():
            data = os.read(master, 65536)
            raw.extend(data)
            screen.feed(data)

        def header():
            return screen.text().splitlines()[1].rstrip()

        def wait(check):
            deadline = time.monotonic() + 6
            while time.monotonic() < deadline:
                if select.select([master], [], [], .02)[0]:
                    read()
                if not screen.synchronized and check(screen.text()):
                    return
                if process.poll() is not None:
                    break
            raise AssertionError(screen.text())

        def settle(seconds=.3):
            deadline = time.monotonic() + seconds
            while time.monotonic() < deadline:
                if select.select([master], [], [], .02)[0]:
                    read()

        def send(keys, text):
            os.write(master, keys)
            wait(lambda s: text in s)

        def flip(keys, label):
            raw.clear()
            os.write(master, keys)
            wait(lambda _: header().endswith(f'{label}  ?'))
            settle()
            painted = CHARCOAL if label == 'light' else PAPER
            assert painted in raw, f'{label} label without its paper'

        def click_label():
            line = header()
            column = max(line.rfind('dark'), line.rfind('light')) + 2
            return f'\x1b[<0;{column};2M\x1b[<0;{column};2m'.encode()

        try:
            wait(lambda s: '1  think' in s)
            if no_color:
                settle()
                before = screen.text()
                assert 'dark' not in header() and 'light' not in header(), header()
                assert b'48;2;' not in raw and b'38;2;' not in raw, 'NO_COLOR painted colour'
                os.write(master, CTRL_T)
                os.write(master, f'\x1b[<0;{width - 8};2M\x1b[<0;{width - 8};2m'.encode())
                settle()
                assert screen.text() == before, 'the hidden switch changed the screen'
                send(b'1', "What's on your mind?")
                send(b'TEST typed', 'TEST typed')
                os.write(master, CTRL_T)
                send(b' text', 'TEST typed text')
                assert b'48;2;' not in raw and b'38;2;' not in raw, 'NO_COLOR painted colour'
            else:
                wait(lambda _: header().endswith('dark  ?'))  # every launch starts light
                flip(CTRL_T, 'light')
                flip(click_label(), 'dark')
                flip(CTRL_T, 'light')
                for number, place in enumerate(PLACES[1:], start=2):
                    os.write(master, str(number).encode())
                    wait(lambda s: place in s.splitlines()[1])
                    assert header().endswith('light  ?'), (place, header())
                    flip(CTRL_T, 'dark')
                    flip(click_label(), 'light')
                    send(b'\x1b' if place in ('measure', 'shape') else b'\x1b[21~', '1  think')
                send(b'1', "What's on your mind?")
                assert header().endswith('light  ?'), header()
                send(b'TEST tilt', 'TEST tilt')
                flip(CTRL_T, 'dark')
                send(b' t T', 'TEST tilt t T')
                flip(click_label(), 'light')
                send(b'?', 'TEST tilt t T?')
                assert 'TEST tilt t T?' in screen.text() and 'TEST tilt t T?t' not in screen.text()
            send(b'\x1b', 'lose this draft?')
            send(b'y', '1  think')
            os.write(master, b'q')
            process.wait(timeout=5)
            assert process.returncode == 0
            assert not (root / '.local/share/tinkery').exists(), 'the theme choice was saved'
            print(json.dumps(dict(width=width, height=height, provider='none', no_color=no_color,
                                  journey='theme switch: key, click, every place, typed words')))
        finally:
            if process.poll() is None:
                process.kill()
                process.wait()
            os.close(master)
            os.close(slave)


for size in [(80, 24), (100, 30)]:
    journey(*size)
    journey(*size)  # A second launch starts light again.
journey(80, 24, no_color=True)
