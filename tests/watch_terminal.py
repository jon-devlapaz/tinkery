"""Watch over a fake tink-sdlc checkout in TEST roots; no real checkout, providers or writes."""
import fcntl
import hashlib
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
if len(sys.argv) == 4:
    slug = sys.argv[2]
    row = next(r for r in state['list']['runs'] if r['slug'] == slug)
    detail = state.get('details', {}).get(slug, state['detail'])
    detail['run']['verification_status'] = row['verification_status']
    detail['run']['next_action'] = row['next_action']
    detail['run']['seed_contract'] = row.get('seed_contract')
    if row['verification_status'] != 'blocked':
        detail['run']['gates'][0]['status'] = 'approved'
        if slug not in state.get('details', {}):
            detail['run']['checklist'][1].update(automatic=True, proof='automated')
    print(json.dumps(detail))
else:
    print(json.dumps(state['list']))
'''


def run(slug, status):
    return {'slug': slug, 'run_id': 'TEST', 'profile': 'light', 'kind': 'feature',
            'verification_status': status, 'next_action': f'TEST next for {slug}',
            'has_lock': False, 'checklist_summary': {'total': 3, 'passed': 1}}


def envelope(**value):
    return {'protocol': 'tink-sdlc', 'api_version': 1, 'workspace': '/TEST', **value}


DETAIL = envelope(run={'verification_status': 'blocked', 'next_action': 'TEST review the brief',
                       'gates': [{'stage': 3, 'status': 'pending', 'blocked': False}],
                       'checklist': [{'id': 'TEST-auto', 'verify': 'TEST executable check', 'status': 'passed',
                                      'proof': 'automated', 'automatic': True, 'needs_recheck': False, 'mark': None},
                                     {'id': 'TEST-manual', 'verify': 'TEST operator observation', 'status': 'pending',
                                      'proof': 'attested', 'automatic': False, 'needs_recheck': False, 'mark': None}],
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
        args = [str(REPO / 'target/debug/tinkery'), '--home', '--operator', 'agent', '--seed-session-root', str(base / 'TEST-sessions')]
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
                assert 'Quiet' in text and 'verified · 1/2 checks passed' in text
                assert 'From python3 _system/scripts/sdlc.py status --json' in text
                assert calls() == ['status --json', 'status quiet-a --json', 'status waits --json']
                set_state(list=envelope(runs=[run('quiet-a', 'failed'), run('waits', 'blocked')]))
                send(b'r', 'quiet-a  verification failed')
                assert calls() == ['status --json', 'status quiet-a --json', 'status waits --json'] * 2
                send(b'\r', 'Stage 3 gate: approved')
                assert 'Checklist: 1/2 passed · 2 with a check, 0 attested' in screen.text()
                assert calls()[-1] == 'status quiet-a --json', calls()
                send(b'\x1b', 'Needs you')
                send(b'?', 'What Watch does')
                send(b'?', 'Needs you')

                # Summary counts cannot identify a manual observation. Detail must decide order.
                long_detail = json.loads(json.dumps(DETAIL))
                long_detail['run']['checklist'][0]['mark'] = {'evidence': 'TEST evidence ' * 500 + 'TEST-LAST-EVIDENCE'}
                long_detail['run']['errors'] = ['TEST-FIRST-ERROR ' + 'TEST explanation ' * 500 + 'TEST-LAST-ERROR']
                set_state(list=envelope(runs=[run('quiet-a', 'current'), run('observe', 'current')]),
                          details={'observe': long_detail})
                send(b'r', '1 attestations owed')
                text = screen.text()
                assert text.index('observe') < text.index('quiet-a'), text
                before_detail = len(calls())
                send(b'\r', 'TEST executable check')
                assert 'TEST-LAST-ERROR' not in screen.text()
                send(b'\x1b[F', 'TEST-LAST-ERROR')
                send(b'\x1b[5~', 'TEST explanation')
                send(b'\x1b[H', 'Stage 3 gate: approved')
                send(b'\x1b[6~', 'TEST evidence')
                assert len(calls()) == before_detail + 1, 'scrolling launched a read'
                send(b'\x1b', 'Needs you')
                for status in ['blocked', 'failed', 'interrupted', 'running', 'not-run', 'stale', 'invalid']:
                    row = run('TEST-state', status)
                    if status == 'invalid':
                        row['error'] = 'TEST broken run'
                    set_state(list=envelope(runs=[row]))
                    words = {'blocked': 'waiting on a decision', 'failed': 'verification failed',
                             'interrupted': 'verification interrupted', 'running': 'running',
                             'not-run': 'not verified yet', 'stale': 'verification out of date',
                             'invalid': "can't be read"}
                    send(b'r', words[status])
                # Only the source path AND the recorded bytes lend a goal to a run.
                seed = base / 'TEST-session/seed-contract.md'
                seed.parent.mkdir()
                seed.write_bytes(b'TEST confirmed original seed\n')
                goal = ' TEST café  \nTEST exact goal tail  '
                (seed.parent / 'ledger.json').write_text(json.dumps({'schema_version': 1, 'goal': goal}))
                original = seed.read_bytes()
                record = {'path': 'runs/unrelated/seed-contract.md', 'source': str(seed.resolve()),
                          'sha256': hashlib.sha256(original).hexdigest()}
                row = {**run('TEST-unrelated-slug', 'blocked'), 'seed_contract': record}
                for seed_case in ('matched', 'edited', 'moved', 'missing'):
                    seed.write_bytes(original)
                    if seed_case == 'edited':
                        seed.write_bytes(original + b'TEST edited\n')
                    elif seed_case == 'moved':
                        seed.rename(base / 'TEST-moved-seed.md')
                    elif seed_case == 'missing':
                        seed.unlink()
                    set_state(list=envelope(runs=[row]))
                    expected = 'TEST exact goal tail' if seed_case == 'matched' else 'Goal unknown:'
                    send(b'r', expected)
                    assert ('TEST exact goal tail' in screen.text()) == (seed_case == 'matched')
                    send(b'\r', 'Stage 3 gate: pending')
                    assert expected in screen.text(), screen.text()
                    assert ('TEST exact goal tail' in screen.text()) == (seed_case == 'matched')
                    assert not any(word in screen.text() for word in ('seed_contract', 'sha256', 'API'))
                    send(b'\x1b', 'Needs you')
                set_state(list={**envelope(runs=[]), 'protocol': 'other'})
                send(b'r', "Couldn't read the runs.")
                assert "isn't supported" in screen.text()
                slow_call_count = len(calls()) + 1
                set_state(slow=True, list=envelope(runs=[]))
                send(b'r', 'Reading runs')
                wait(lambda: pid.exists() and len(calls()) == slow_call_count)
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
