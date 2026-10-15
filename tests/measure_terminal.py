"""Measure failure/recovery with fake commands and TEST roots; no real ledger or providers."""
import fcntl
import json
import os
from pathlib import Path
import select
import signal
import struct
import subprocess
import sys
import tempfile
import termios
import time

from vt_screen import Screen

REPO = Path(__file__).resolve().parents[1]
EXPLANATIONS = {
    'missing-python': "Tinkery couldn’t find what it needs to read the ledger.",
    'missing-module': "Tinkery couldn’t load what it needs to read the ledger.",
    'missing-directory': "The configured ledger directory couldn’t be opened.",
    'failed': "The ledger reader exited without completing the report.",
    'empty': "The ledger reader exited without completing the report.",
    'launch': "The ledger reader couldn’t be started.",
}
MARKERS = [f'M{i:03}café' for i in range(160)]
ERROR = ('FIRST-DETAIL\n' + ' '.join(MARKERS) + '\n'
         + ''.join(f'line-{i:03}\n' for i in range(45))
         + '\x1b[2J\r\t\x07CONTROL-END\nLAST-DETAIL\n')
SUCCESS = 'Existing report\nWork: unknown\nAttention: unknown\n'


def journey(width, height, kind, no_color):
    with tempfile.TemporaryDirectory(prefix='tinkery-TEST-measure-') as directory:
        root = Path(directory).resolve()
        home = root / 'HOME'
        home.mkdir()
        source = root / ('TEST-ledger-' + 'café-' * 24)
        source.mkdir()
        (source / 'unchanged').write_bytes(b'synthetic ledger boundary')
        binary = root / 'bin'
        binary.mkdir()
        calls = root / 'calls'
        command = binary / 'python3'
        stderr = (f'{sys.executable}: No module named tink_substrate\n' if kind == 'missing-module'
                  else '' if kind == 'empty' else ERROR)
        command.write_text(f'#!{sys.executable}\nimport sys\nfrom pathlib import Path\n'
                           f'with Path({str(calls)!r}).open("a") as f: f.write("read\\n")\n'
                           f'sys.stdout.write({SUCCESS!r} if {kind!r} == "success" else "")\n'
                           f'sys.stderr.write({stderr!r})\n'
                           f'sys.exit(0 if {kind!r} == "success" else 7)\n')
        command.chmod(0o644 if kind == 'launch' else 0o755)
        if kind == 'missing-python':
            command.rename(binary / 'not-python')
        selected_source = source / 'missing' if kind == 'missing-directory' else source
        before = {str(p): p.read_bytes() for p in source.rglob('*') if p.is_file()}
        master, slave = os.openpty()
        fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack('HHHH', height, width, 0, 0))
        screen = Screen(width, height)
        raw = bytearray()
        env = {**os.environ, 'HOME': str(home), 'PATH': str(binary),
               'TERM': 'xterm-256color', 'TINK_SUBSTRATE_DIR': str(selected_source),
               'TINKERY_NODE': '/tinkery-TEST-no-node'}
        env.pop('NO_COLOR', None)
        if no_color:
            env['NO_COLOR'] = '1'
        process = subprocess.Popen([str(REPO / 'target/debug/tinkery'), '--home',
                                    '--seed-session-root', str(root / 'TEST-sessions')],
                                   cwd=root, env=env, stdin=slave, stdout=slave, stderr=slave)

        def wait(predicate):
            deadline = time.monotonic() + 5
            while time.monotonic() < deadline:
                if select.select([master], [], [], .02)[0]:
                    try:
                        data = os.read(master, 65536)
                        raw.extend(data)
                        screen.feed(data)
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

        def compact():
            return ''.join(screen.text().split())

        try:
            wait(lambda: '1  think' in screen.text())
            if kind == 'success':
                send(b'7', 'Existing report')
                assert SUCCESS.strip() in '\n'.join(line.strip() for line in screen.text().splitlines())
                assert 'Ledger report unavailable' not in screen.text()
                send(b'q', '1  think')
            else:
                send(b'7', 'Ledger report unavailable')
                assert EXPLANATIONS[kind] in screen.text(), screen.text()
                assert 'Recorded work and attention are unknown.' in screen.text()
                assert 'esc back   ? help' in screen.text()
                assert str(root) not in screen.text() and 'FIRST-DETAIL' not in screen.text()
                summary = screen.text()
                send(b'?', 'Right now')
                assert 'h how it works' in screen.text() and 'q quits' not in screen.text()
                send(b'h', 'How it works')
                assert 'esc back   h help' in screen.text() and 'q menu' in screen.text()
                seen = compact()
                # Every screenful overlaps the previous one: no skipped diagnostic rows.
                for _ in range(120):
                    old = screen.text()
                    frames = raw.count(b'\x1b[?2026l')
                    os.write(master, b'\x1b[6~')
                    # Two completed frames exclude a queued pre-key frame, even at the clamp.
                    wait(lambda: raw.count(b'\x1b[?2026l') >= frames + 2)
                    seen += compact()
                    assert 'esc back   h help' in screen.text() and 'q menu' in screen.text()
                    if screen.text() == old:
                        break
                else:
                    raise AssertionError('Diagnostics did not clamp')
                assert ''.join(str(selected_source).split()) in seen
                assert 'python3-B-mtink_substrateledgerreport' in seen
                if kind in ('failed', 'empty', 'missing-module'):
                    assert 'Exitstatus:7' in seen
                if kind == 'empty':
                    assert 'Noerrordetailswerereturned.' in seen
                elif kind == 'failed':
                    assert all(marker in seen for marker in MARKERS)
                    assert all(f'line-{i:03}' in seen for i in range(45))
                    assert 'FIRST-DETAIL' in seen and 'LAST-DETAIL' in seen
                    assert '\\u{1b}[2J\\r\\t\\u{7}CONTROL-END' in seen
                    # Resize while scrolled: details remain navigable and controls visible.
                    resized_width = 100 if width == 80 else 80
                    fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack('HHHH', 24, resized_width, 0, 0))
                    screen = Screen(resized_width, 24)
                    os.kill(process.pid, signal.SIGWINCH)
                    wait(lambda: 'q menu' in screen.text())
                    send(b'\x1b[5~', 'esc back   h help')
                send(b'\x1b', 'Right now')
                send(b'\x1b', 'Ledger report unavailable')
                assert 'esc back   ? help' in screen.text()
                if kind != 'failed':
                    assert screen.text() == summary
                send(b'\x1b', '1  think')
                send(b'7', 'Ledger report unavailable')
                send(b'\x1bOP', 'Right now')
                send(b'h', 'How it works')
                send(b'q', '1  think')
                assert process.poll() is None
                send(b'7', 'Ledger report unavailable')
                send(b'?', 'Right now')
                send(b'?', 'Ledger report unavailable')
                send(b'q', '1  think')
            os.write(master, b'q')
            process.wait(timeout=5)
            assert process.returncode == 0
            expected_reads = 1 if kind == 'success' else 3
            if kind not in ('missing-python', 'missing-directory', 'launch'):
                assert calls.read_text() == 'read\n' * expected_reads, 'Help unexpectedly reread the ledger'
            else:
                assert not calls.exists()
            assert {str(p): p.read_bytes() for p in source.rglob('*') if p.is_file()} == before
            assert not (root / 'TEST-sessions').exists()
            print(json.dumps(dict(width=width, height=height, failure=kind, no_color=no_color,
                                  provider_calls=0, source='unchanged synthetic TEST fixture')))
        finally:
            if process.poll() is None:
                process.kill()
                process.wait()
            os.close(master)
            os.close(slave)


if __name__ == '__main__':
    for dimensions in [(80, 24), (100, 30), (160, 40), (320, 40)]:
        for no_color in (False, True):
            for kind in [*EXPLANATIONS, 'success']:
                journey(*dimensions, kind, no_color)
