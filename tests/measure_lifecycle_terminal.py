"""Measure waiting, cancellation and shutdown with synthetic TEST subprocess groups only."""
import fcntl
import json
import os
from pathlib import Path
import select
import signal
import struct
import test_launch as subprocess
import sys
import tempfile
import termios
import time

from vt_screen import Screen

REPO = Path(__file__).resolve().parents[1]


def journey(width, height, kind, no_color):
    with tempfile.TemporaryDirectory(prefix='tinkery-TEST-measure-lifecycle-') as directory:
        root = Path(directory)
        home, source, binary = (root / name for name in ('HOME', 'TEST-ledger', 'bin'))
        for path in (home, source, binary):
            path.mkdir()
        fixture = source / 'unchanged'
        fixture.write_bytes(b'synthetic ledger')
        calls, ready, release, pids = (root / name for name in ('calls', 'ready', 'release', 'pids'))
        command = binary / 'python3'
        success = kind in ('success', 'success-helpers')
        with_helper = kind in ('cancel', 'success-helpers')
        command.write_text(
            f'#!{sys.executable}\nimport os, signal, subprocess, sys, time\nfrom pathlib import Path\n'
            f'with Path({str(calls)!r}).open("a") as f: f.write("read\\n")\n'
            f'with Path({str(pids)!r}).open("a") as f: f.write(str(os.getpid()) + "\\n")\n'
            + (f'signal.signal(signal.SIGTERM, signal.SIG_IGN)\n'
               f'child = subprocess.Popen([sys.executable, "-c", "import time; time.sleep(60)"], '
               + ('stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL' if success else '')
               + ')\n'
               f'assert os.getpid() == os.getpgrp() == os.getpgid(child.pid)\n'
               f'with Path({str(pids)!r}).open("a") as f: f.write(str(child.pid) + "\\n")\n'
               if with_helper else '')
            + f'Path({str(ready)!r}).write_text("ready")\n'
            f'while not Path({str(release)!r}).exists(): time.sleep(.01)\n'
            f'print("Existing report" if {success!r} else "", flush=True)\n'
            f'print("synthetic failure", file=sys.stderr)\n'
            f'sys.exit(0 if {success!r} else 7)\n')
        command.chmod(0o755)
        master, slave = os.openpty()
        fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack('HHHH', height, width, 0, 0))
        screen = Screen(width, height)
        env = {**os.environ, 'HOME': str(home), 'PATH': str(binary), 'TERM': 'xterm-256color',
               'TINK_SUBSTRATE_DIR': str(source), 'TINKERY_NODE': '/tinkery-TEST-no-node'}
        env.pop('NO_COLOR', None)
        if no_color:
            env['NO_COLOR'] = '1'
        process = subprocess.Popen([str(REPO / 'target/debug/tinkery'), '--home',
                                    '--seed-session-root', str(root / 'TEST-sessions')],
                                   cwd=root, env=subprocess.environment(env, home=(env)['HOME']), stdin=slave, stdout=slave, stderr=slave)

        def wait(predicate):
            deadline = time.monotonic() + 8
            while time.monotonic() < deadline:
                if select.select([master], [], [], .02)[0]:
                    try:
                        screen.feed(os.read(master, 65536))
                    except OSError:
                        if process.poll() is None:
                            raise
                if predicate() and not screen.synchronized:
                    return
                if process.poll() is not None:
                    break
            raise AssertionError((kind, width, height, screen.text()))

        def send(key, expected):
            os.write(master, key)
            wait(lambda: expected in screen.text())

        def assert_owned_stopped():
            for index, pid in enumerate(map(int, pids.read_text().splitlines())):
                state = subprocess.run(['ps', '-o', 'stat=', '-p', str(pid)], capture_output=True, text=True, env=subprocess.environment()).stdout.strip()
                is_helper = with_helper and index % 2 == 1
                assert not state or (is_helper and state.startswith('Z')), (pid, state)

        completed = False
        reads = 1
        try:
            wait(lambda: '1  think' in screen.text())
            assert not calls.exists(), 'menu must not read the ledger'
            send(b'7', 'Reading ledger…')
            wait(ready.exists)
            assert 'esc back   ? help' in screen.text()
            assert 'Recorded work and attention are unknown.' in screen.text()
            send(b'?', 'Right now')
            assert calls.read_text() == 'read\n'
            send(b'\x1b', 'Reading ledger…')
            # Reflow must keep controls and input live while the child is blocked.
            resized = 100 if width == 80 else 80
            fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack('HHHH', 24, resized, 0, 0))
            screen = Screen(resized, 24)
            os.kill(process.pid, signal.SIGWINCH)
            wait(lambda: 'Reading ledger…' in screen.text())
            if kind == 'cancel':
                send(b'\x1b', '1  think')
                send(b'7', 'Finishing the previous read…')
                send(b'?', 'Right now')
                assert calls.read_text() == 'read\n', 'reopen during cleanup queued a read'
                # Both application quit routes while Measure owns a stopping process.
                os.write(master, b'\x03' if no_color else b'\x1b[21~')
            else:
                release.touch()
                wait(lambda: ('Existing report' if success else 'Ledger report unavailable') in screen.text())
                if with_helper:
                    assert_owned_stopped()
                    send(b'q', '1  think')
                    send(b'7', 'Existing report')
                    reads = 2
                    assert calls.read_text() == 'read\n' * reads
                    assert_owned_stopped()
                if kind == 'failure':
                    send(b'?', 'Right now')
                    send(b'h', 'How it works')
                    assert 'synthetic failure' in screen.text()
                send(b'q', '1  think')
                os.write(master, b'q')
            process.wait(timeout=8)
            assert process.returncode == 0
            assert calls.read_text() == 'read\n' * reads
            assert_owned_stopped()
            assert fixture.read_bytes() == b'synthetic ledger'
            assert sorted(p.name for p in source.iterdir()) == ['unchanged']
            assert not (root / 'TEST-sessions').exists()
            completed = True
            print(json.dumps(dict(width=width, height=height, kind=kind, no_color=no_color,
                                  reads=reads, provider_calls=0, source='unchanged TEST fixture')))
        finally:
            if process.poll() is None:
                process.kill()
                process.wait()
            if not completed and pids.exists():
                for pid in map(int, pids.read_text().splitlines()):
                    try:
                        os.kill(pid, signal.SIGKILL)
                    except ProcessLookupError:
                        pass
            os.close(master)
            os.close(slave)


if __name__ == '__main__':
    for dimensions in [(80, 24), (100, 30), (160, 40), (320, 40)]:
        for no_color in (False, True):
            for kind in ('success', 'failure', 'cancel', 'success-helpers'):
                journey(*dimensions, kind, no_color)
