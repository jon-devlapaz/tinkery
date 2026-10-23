"""Taste over synthetic API 1, explicit handoff links and fake gh; no network or providers."""
import base64
import copy
import fcntl
import hashlib
import json
import os
from pathlib import Path
import re
import select
import struct
import sys
import tempfile
import termios
import time
import test_launch as subprocess
from vt_screen import Screen

REPO = Path(__file__).resolve().parents[1]
HEAD = 'a' * 40
NEXT = 'b' * 40
URL = 'https://github.com/TEST/clinic-tycoon/pull/1'
GOAL = (' TEST café  \nWhy: Explore trade-offs.\n'
        'Done when: Allocation works; assumptions are inspectable.\nMust not: Claim clinical validation.  ')
FAKE = '''import json, os, sys, time
from pathlib import Path
with open(os.environ['TEST_TASTE_CALLS'], 'a') as log:
    log.write(json.dumps(sys.argv[1:]) + '\\n')
Path(os.environ['TEST_TASTE_PID']).write_text(str(os.getpid()))
state = json.loads(Path(os.environ['TEST_TASTE_STATE']).read_text())
if state.get('slow'):
    time.sleep(30)
if '--web' in sys.argv:
    sys.exit(0)
if '--json' in sys.argv and 'pr' in sys.argv:
    if state.get('gh_fail'):
        sys.exit(1)
    print(json.dumps(state['pr']))
elif len(sys.argv) == 4:
    print(json.dumps(state['detail']))
else:
    print(json.dumps(state['list']))
'''


def envelope(**value):
    return {'protocol': 'tink-sdlc', 'api_version': 1, **value}


def fixture(record):
    row = {'slug': 'TEST-result', 'run_id': '00000000-0000-4000-8000-000000000114',
           'verification_status': 'current', 'seed_contract': record}
    item = {'id': 'seed-1', 'verify': 'Allocation works', 'status': 'passed',
            'automatic': True, 'proof': 'automated', 'needs_recheck': False, 'mark': None}
    detail = {**row, 'gates': [{'stage': 3, 'status': 'stale'}], 'errors': [],
              'verification': {'result': 'passed', 'passed': True,
                               'candidate': {'head': HEAD, 'tree': 'TEST-tree'}},
              'checklist': [item, {**item, 'id': 'seed-2', 'verify': 'confirm no clinical claims',
                                  'automatic': False, 'proof': 'attested', 'status': 'pending'}]}
    pr = {'url': URL, 'headRefOid': HEAD, 'state': 'OPEN', 'statusCheckRollup': [
        {'__typename': 'CheckRun', 'name': 'TEST CI', 'status': 'COMPLETED', 'conclusion': 'SUCCESS'}]}
    return {'list': envelope(runs=[row]), 'detail': envelope(run=detail), 'pr': pr}


def tree(root):
    return {str(p.relative_to(root)): p.read_bytes() for p in root.rglob('*') if p.is_file()}


def journey(width, height, no_color):
    with tempfile.TemporaryDirectory(prefix='tinkery-TEST-taste-') as directory:
        base = Path(directory).resolve()
        checkout = base / 'TEST-checkout'
        scripts = checkout / '_system/scripts'
        scripts.mkdir(parents=True)
        (scripts / 'sdlc.py').write_text(FAKE)
        handoff = checkout / 'runs/TEST-result/handoff.md'
        handoff.parent.mkdir(parents=True)
        handoff.write_text(f'PR: {URL}\n')
        seed = base / 'TEST-session/seed-contract.md'
        seed.parent.mkdir()
        seed.write_text('TEST seed\n')
        (seed.parent / 'ledger.json').write_text(json.dumps({'schema_version': 1, 'goal': GOAL}))
        record = {'source': str(seed), 'path': 'runs/TEST-result/seed-contract.md',
                  'sha256': hashlib.sha256(seed.read_bytes()).hexdigest()}
        state = fixture(record)
        statefile, log, pid = base / 'state.json', base / 'calls', base / 'pid'
        statefile.write_text(json.dumps(state))
        binary = base / 'bin'
        binary.mkdir()
        (binary / 'python3').symlink_to(sys.executable)
        (binary / 'gh').write_text(f'#!{sys.executable}\n' + FAKE)
        (binary / 'gh').chmod(0o755)
        master, slave = os.openpty()
        fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack('HHHH', height, width, 0, 0))
        screen = Screen(width, height)
        raw = bytearray()
        env = {**os.environ, 'HOME': str(base / 'HOME'), 'PATH': f'{binary}:/usr/bin:/bin',
               'TERM': 'xterm-256color', 'TINKERY_NODE': '/tinkery-TEST-no-node',
               'TEST_TASTE_STATE': str(statefile), 'TEST_TASTE_CALLS': str(log), 'TEST_TASTE_PID': str(pid)}
        env.pop('NO_COLOR', None)
        if no_color:
            env['NO_COLOR'] = '1'
        args = [str(REPO / 'target/debug/tinkery'), '--operator', 'agent', '--home',
                '--sdlc-root', str(checkout), '--seed-session-root', str(base / 'TEST-sessions')]
        process = subprocess.Popen(args, cwd=base, env=subprocess.environment(env, home=env['HOME']),
                                   stdin=slave, stdout=slave, stderr=slave)
        original = tree(checkout)

        def wait(predicate, seconds=8):
            deadline = time.monotonic() + seconds
            while time.monotonic() < deadline:
                if select.select([master], [], [], .02)[0]:
                    chunk = os.read(master, 65536)
                    raw.extend(chunk)
                    screen.feed(chunk)
                if predicate() and not screen.synchronized:
                    return
                assert process.poll() is None, screen.text()
            raise AssertionError((width, height, no_color, screen.text()))

        def send(key, expected):
            os.write(master, key)
            wait(lambda: expected in screen.text())

        def calls():
            return [json.loads(line) for line in log.read_text().splitlines()] if log.exists() else []

        def update():
            statefile.write_text(json.dumps(state))

        def copied():
            payloads = re.findall(rb'\x1b\]52;[^;]*;([A-Za-z0-9+/=]+)(?:\x07|\x1b\\)', raw)
            return [base64.b64decode(p).decode() for p in payloads]

        try:
            wait(lambda: '1  think' in screen.text())
            send(b'6', 'Press r to read results')
            assert not calls(), 'opening Taste must disclose reads, not perform them'
            send(b'?', 'What Taste reads')
            os.write(master, b'oacr')
            time.sleep(.1)
            assert not calls(), 'help leaked actions'
            send(b'\x1b', 'Press r to read results')
            send(b'r', 'Results waiting on you')
            assert len(calls()) == 3, calls()
            assert all('--web' not in c for c in calls())
            send(b'\r', 'Goal (unchanged)')
            assert 'TEST café' in screen.text(), screen.text()
            assert HEAD in screen.text(), screen.text()
            # Reads and scrolling never launch anything; the whole goal is retained in the copy.
            read_count = len(calls())
            send(b'\x1b[F', 'API 1 verification receipt')
            send(b'a', 'Handoff copied; decide externally.')
            wait(lambda: bool(copied()))
            assert GOAL in copied()[-1] and HEAD in copied()[-1]
            assert 'Acceptance review requested' in copied()[-1]
            assert 're-approve the plan' in copied()[-1] and 'confirm no clinical claims' in copied()[-1]
            assert 'No verdict recorded' in copied()[-1]
            count = len(copied())
            os.write(master, b'c')
            wait(lambda: len(copied()) > count)
            assert 'Correction requested' in copied()[-1]
            assert len(calls()) == read_count
            send(b'o', 'PR sent to browser')
            assert calls()[-1] == ['pr', 'view', URL, '--web']
            # New commit, still-green old local receipt: the local condition must be stale.
            state['pr']['headRefOid'] = NEXT
            state['pr']['statusCheckRollup'] = []
            update()
            send(b'r', NEXT)
            assert 'Allocation works: stale' in screen.text(), screen.text()
            assert 'Allocation works: pass' not in screen.text()
            assert HEAD not in screen.text(), 'old revision repeated in compact view'
            send(b'\x1b[F', 'Earlier verification is history')
            count = len(copied())
            os.write(master, b'c')
            wait(lambda: len(copied()) > count)
            assert NEXT in copied()[-1] and 'Allocation works: stale' in copied()[-1]
            # Matching failure and missing evidence remain separate from success.
            state['pr']['headRefOid'] = HEAD
            state['detail']['run']['verification']['result'] = 'failed'
            state['detail']['run']['verification']['passed'] = False
            state['detail']['run']['verification_status'] = 'failed'
            state['detail']['run']['checklist'][0]['status'] = 'failed'
            update()
            send(b'r', 'Allocation works: fail')
            assert 'pending' in screen.text()
            # Unknown original refuses copying, without falling back to the old goal.
            seed.write_text('TEST changed original\n')
            send(b'r', 'Goal unknown')
            count = len(copied())
            send(b'a', 'Goal/run identity unknown')
            assert len(copied()) == count
            seed.write_text('TEST seed\n')
            # Missing/conflicting handoff and failed remote read remove the result head.
            for content, expected in [('', 'PR link missing'),
                                      (f'{URL}\nhttps://github.com/TEST/clinic-tycoon/pull/2\n', 'Conflicting PR links')]:
                handoff.write_text(content)
                send(b'r', 'Commit: unknown')
                send(b'\r', 'Goal (unchanged)')
                send(b'\x1b[F', expected)
            handoff.write_text(original['runs/TEST-result/handoff.md'].decode())
            state['gh_fail'] = True
            update()
            send(b'r', 'Commit: unknown')
            send(b'\r', 'Goal (unchanged)')
            send(b'\x1b[F', "Couldn't read GitHub")
            del state['gh_fail']
            update()
            send(b'\x1b', 'Results waiting on you')
            send(b'\x1b', '1  think')
            before_reopen = len(calls())
            send(b'6', 'Press r to read results')
            assert len(calls()) == before_reopen
            # Cancellation must stop an in-flight helper, with no late reopening.
            state['slow'] = True
            update()
            os.write(master, b'r')
            wait(lambda: len(calls()) > before_reopen)
            send(b'\x1b', '1  think')
            deadline = time.monotonic() + 3
            while time.monotonic() < deadline:
                try:
                    os.kill(int(pid.read_text()), 0)
                except ProcessLookupError:
                    break
                time.sleep(.02)
            else:
                raise AssertionError('Taste left its read process running')
            os.write(master, b'q')
            process.wait(timeout=5)
            assert process.returncode == 0
            assert tree(checkout) == original, 'Taste mutated the named checkout'
            assert not (base / 'TEST-sessions').exists()
            assert not any(any(x in c for x in ['verify', 'mark', 'decide', 'stage', 'merge', 'review']) for c in calls())
            print(json.dumps(dict(journey='Taste', width=width, height=height, no_color=no_color,
                                  provider_calls=0, navigation='explicit o only', verdicts=0)))
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
