"""Watch over a fake tink-sdlc checkout in TEST roots; no real checkout, providers or writes."""
import fcntl
import json
import os
from pathlib import Path
import select
import struct
import subprocess
import sys
import tempfile
import termios
import time

from vt_screen import Screen

REPO = Path(__file__).resolve().parents[1]
FAKE = '''import json, os, sys, time
from pathlib import Path
with open(os.environ['TEST_WATCH_LOG'], 'a') as log:
    log.write(' '.join(sys.argv[1:]) + '\\n')
Path(os.environ['TEST_WATCH_PID']).write_text(str(os.getpid()))
state = json.loads(Path(os.environ['TEST_WATCH_STATE']).read_text())
if state.get('slow'):
    time.sleep(30)
print(json.dumps(state['detail'] if len(sys.argv) == 4 else state['list']))
'''


def run(slug, status):
    return {'slug': slug, 'run_id': 'TEST', 'profile': 'light', 'kind': 'feature',
            'verification_status': status, 'next_action': f'TEST next for {slug}',
            'has_lock': False, 'checklist_summary': {'total': 3, 'passed': 1}}


def envelope(**value):
    return {'protocol': 'tink-sdlc', 'api_version': 1, 'workspace': '/TEST', **value}


DETAIL = envelope(run={'verification_status': 'blocked', 'next_action': 'TEST review the brief',
                       'gates': [{'stage': 3, 'status': 'pending', 'blocked': False}],
                       'checklist': [{'status': 'passed', 'proof': 'automated'},
                                     {'status': 'pending', 'proof': 'attested'}],
                       'errors': []})


def tree(root):
    return {str(p): p.read_bytes() for p in sorted(root.rglob('*')) if p.is_file()}


def journey(width, height, no_color, configured=True):
    with tempfile.TemporaryDirectory(prefix='tinkery-TEST-watch-') as directory:
        base = Path(directory).resolve()
        checkout = base / 'TEST-checkout'
        (checkout / '_system/scripts').mkdir(parents=True)
        (checkout / '_system/scripts/sdlc.py').write_text(FAKE)
        state, log, pid = base / 'state.json', base / 'calls', base / 'pid'
        binary = base / 'bin'
        binary.mkdir()
        (binary / 'python3').symlink_to(sys.executable)

        def set_state(**value):
            state.write_text(json.dumps({'detail': DETAIL, **value}))

        set_state(list=envelope(runs=[run('quiet-a', 'current'), run('waits', 'blocked')]))
        before = tree(checkout)
        master, slave = os.openpty()
        fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack('HHHH', height, width, 0, 0))
        screen = Screen(width, height)
        env = {**os.environ, 'HOME': str(base / 'HOME'), 'PATH': f'{binary}:/usr/bin:/bin',
               'TERM': 'xterm-256color', 'TINKERY_NODE': '/tinkery-TEST-no-node',
               'TEST_WATCH_LOG': str(log), 'TEST_WATCH_PID': str(pid), 'TEST_WATCH_STATE': str(state)}
        env.pop('NO_COLOR', None)
        if no_color:
            env['NO_COLOR'] = '1'
        args = [str(REPO / 'target/debug/tinkery'), '--home', '--seed-session-root', str(base / 'TEST-sessions')]
        if configured:
            args += ['--sdlc-root', str(checkout)]
        process = subprocess.Popen(args, cwd=base, env=env, stdin=slave, stdout=slave, stderr=slave)

        def wait(predicate, seconds=5):
            deadline = time.monotonic() + seconds
            while time.monotonic() < deadline:
                if select.select([master], [], [], .02)[0]:
                    screen.feed(os.read(master, 65536))
                if predicate() and not screen.synchronized:
                    return
                assert process.poll() is None, screen.text()
            raise AssertionError((width, height, no_color, screen.text()))

        def send(key, expected):
            os.write(master, key)
            wait(lambda: expected in screen.text())

        def calls():
            return log.read_text().splitlines() if log.exists() else []

        try:
            wait(lambda: '1  think' in screen.text())
            if not configured:
                send(b'5', 'Start Tinkery with --sdlc-root PATH')
                assert not log.exists(), 'nothing runs without a named checkout'
                send(b'\x1b', '1  think')
            else:
                send(b'5', 'Needs you')
                text = screen.text()
                # The waiting run leads, though tink-sdlc listed it second.
                assert text.index('waits') < text.index('quiet-a'), text
                assert 'waiting on a decision' in text and 'TEST next for waits' in text
                assert 'Quiet' in text and 'verified · 1/3 checks passed' in text
                assert 'From python3 _system/scripts/sdlc.py status --json' in text
                assert calls() == ['status --json']
                set_state(list=envelope(runs=[run('quiet-a', 'failed'), run('waits', 'blocked')]))
                send(b'r', 'quiet-a  verification failed')
                assert calls() == ['status --json'] * 2
                send(b'\r', 'Stage 3 gate: pending')
                assert 'Checklist: 1/2 passed · 1 with a check, 1 attested' in screen.text()
                assert calls()[-1] == 'status quiet-a --json', calls()
                send(b'\x1b', 'Needs you')
                send(b'?', 'What Watch does')
                send(b'?', 'Needs you')
                set_state(list={**envelope(runs=[]), 'protocol': 'other'})
                send(b'r', "Couldn't read the runs.")
                assert 'Watchreadstink-sdlcAPI1' in ''.join(screen.text().split())
                set_state(slow=True, list=envelope(runs=[]))
                send(b'r', 'Reading runs')
                wait(lambda: pid.exists() and len(calls()) == 5)
                started = time.monotonic()
                send(b'\x1b', '1  think')
                assert time.monotonic() - started < 3, 'leaving waited on the read'
                deadline = time.monotonic() + 3
                while time.monotonic() < deadline:
                    try:
                        os.kill(int(pid.read_text()), 0)
                    except ProcessLookupError:
                        break
                    time.sleep(.02)
                else:
                    raise AssertionError('leaving left the read running')
            os.write(master, b'q')
            process.wait(timeout=5)
            assert process.returncode == 0
            assert tree(checkout) == before, 'Watch changed the checkout'
            assert not (base / 'TEST-sessions').exists()
            print(json.dumps(dict(width=width, height=height, no_color=no_color, configured=configured,
                                  reads=len(calls()), provider_calls=0, checkout='unchanged TEST fixture')))
        finally:
            if process.poll() is None:
                process.kill()
                process.wait()
            os.close(master)
            os.close(slave)


if __name__ == '__main__':
    for dimensions in [(100, 30), (80, 24)]:
        for no_color in (False, True):
            journey(*dimensions, no_color)
    journey(80, 24, False, configured=False)
