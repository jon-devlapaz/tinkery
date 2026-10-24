"""Quiet Taste and unchanged detail/copy transport, using synthetic API 1 and gh only."""
import base64
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
from taste_terminal import FAKE, GOAL, HEAD, REPO, RUN_ID, URL, fixture, tree
from vt_screen import Screen

GOLDEN = REPO / 'tests/fixtures/taste-handoff-355.json'
GAP = 'TEST source gap: /TEST/receipt; command TEST-gap-command'


def run(base, width, height, mono, binary, legacy=False):
    checkout = base / 'TEST-checkout'
    scripts = checkout / '_system/scripts'
    scripts.mkdir(parents=True, exist_ok=True)
    (scripts / 'sdlc.py').write_text(FAKE)
    seed = base / 'TEST-session/seed-contract.md'
    seed.parent.mkdir(exist_ok=True)
    seed.write_text('TEST seed\n')
    (seed.parent / 'ledger.json').write_text(json.dumps({'schema_version': 1, 'goal': GOAL}))
    record = {'source': str(seed), 'path': 'runs/TEST-result/seed-contract.md',
              'sha256': hashlib.sha256(seed.read_bytes()).hexdigest()}
    state = fixture(record)
    state['detail']['run']['errors'] = [GAP]
    state['detail']['run']['checklist'][0]['check']['cwd'] = './'
    statefile, log, pid = base / 'state.json', base / 'calls', base / 'pid'
    statefile.write_text(json.dumps(state))
    log.write_text('')
    helpers = base / 'bin'
    helpers.mkdir(exist_ok=True)
    if not (helpers / 'python3').exists():
        (helpers / 'python3').symlink_to(sys.executable)
    (helpers / 'gh').write_text(f'#!{sys.executable}\n' + FAKE)
    (helpers / 'gh').chmod(0o755)
    original = tree(checkout), tree(seed.parent)
    master, slave = os.openpty()
    fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack('HHHH', height, width, 0, 0))
    screen, raw = Screen(width, height), bytearray()
    env = {**os.environ, 'HOME': str(base / 'HOME'), 'PATH': f'{helpers}:/usr/bin:/bin',
           'TERM': 'xterm-256color', 'TINKERY_NODE': '/tinkery-TEST-no-node',
           'TEST_TASTE_STATE': str(statefile), 'TEST_TASTE_CALLS': str(log), 'TEST_TASTE_PID': str(pid)}
    env.pop('NO_COLOR', None)
    if mono:
        env['NO_COLOR'] = '1'
    process = subprocess.Popen(
        [str(binary), '--operator', 'agent', '--home', '--sdlc-root', str(checkout),
         '--seed-session-root', str(base / 'TEST-sessions'), '--taste-result', f'{RUN_ID}={URL}'],
        cwd=base, env=subprocess.environment(env, home=env['HOME']), stdin=slave, stdout=slave, stderr=slave)

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
        raise AssertionError((width, height, mono, screen.text()))

    def send(key, expected):
        os.write(master, key)
        wait(lambda: expected in screen.text())

    def copied():
        return [base64.b64decode(p) for p in re.findall(
            rb'\x1b\]52;[^;]*;([A-Za-z0-9+/=]+)(?:\x07|\x1b\\)', raw)]

    def quiet():
        text = screen.text()
        for hidden in [URL, HEAD, RUN_ID, 'TEST-result', '/TEST/receipt', 'TEST-gap-command',
                       'TEST-allocation-check.py', 'argv:', 'cwd:', 'Why: Explore trade-offs.',
                       state['detail']['run']['verification']['log']]:
            assert hidden not in text, (hidden, text)
        for shown in ['TEST café', 'Allocation works: pass', 'confirm no clinical claims: pending',
                      'TEST CI: pass', 'Independent review: pending', 're-approve the plan']:
            assert shown in text, (shown, text)

    try:
        wait(lambda: '1  think' in screen.text())
        send(b'6', 'Press r to read results')
        assert not log.read_text(), 'opening performed a read'
        send(b'r', 'Results waiting on you')
        if legacy:
            send(b'\r', 'Goal (unchanged)')
        else:
            quiet()
        send(b'l', 'Guess · Next:')
        main = screen.text()
        if not legacy:
            quiet()
            assert main.count('Next:') == 1
        copies = {}
        for action in ['a', 'c']:
            before = len(copied())
            os.write(master, action.encode())
            wait(lambda: len(copied()) > before and 'Handoff copied' in screen.text())
            copies[action] = copied()[-1]
        if not legacy:
            quiet()
            send(b'\r', 'Goal (unchanged)')
        assert URL in screen.text() and HEAD in screen.text(), screen.text()
        assert screen.text().count(URL) == 1 and screen.text().count('Commit:') == 1
        detail = [screen.text()]
        for _ in range(12):
            if 'run API' in screen.text():
                break
            previous = screen.text()
            os.write(master, b'\x1b[6~')  # PgDn, including long argv/cwd evidence.
            wait(lambda: screen.text() != previous)
            detail.append(screen.text())
        assert 'run API' in screen.text(), detail
        joined = re.sub(r'\s+', '', ''.join(detail))
        for value in [GOAL, URL, HEAD, RUN_ID, 'TEST-result', GAP,
                      str(checkout / 'runs/TEST-result/04-test/output/test-log.md'),
                      'argv: ["python3","TEST-allocation-check.py"]', 'cwd: "./"',
                      state['detail']['run']['verification']['log']]:
            assert re.sub(r'\s+', '', value) in joined, (value, detail)
        for action in ['a', 'c']:
            before = len(copied())
            os.write(master, action.encode())
            wait(lambda: len(copied()) > before)
            assert copied()[-1] == copies[action], 'main/detail copying differs'
        send(b'\x1b', 'Results waiting on you')
        if not legacy:
            quiet()
        send(b'\x1b', '1  think')
        os.write(master, b'q')
        process.wait(timeout=5)
        assert process.returncode == 0
        assert (tree(checkout), tree(seed.parent)) == original
        assert len(log.read_text().splitlines()) == 3, 'navigation/copying performed another read or browser action'
        assert not (base / 'TEST-sessions').exists()
        return {'copies': copies, 'main': main, 'detail': detail}
    finally:
        if process.poll() is None:
            process.kill()
            process.wait()
        os.close(master)
        os.close(slave)


if __name__ == '__main__':
    if '--record-baseline' in sys.argv:
        with tempfile.TemporaryDirectory(prefix='tinkery-TEST-quiet-355-') as directory:
            base = Path(directory).resolve()
            result = run(base, 100, 30, False, REPO / 'target/TEST-taste-before-355', legacy=True)
            GOLDEN.write_text(json.dumps({k: v.decode().replace(str(base), '{ROOT}')
                                          for k, v in result['copies'].items()}, ensure_ascii=False, indent=2) + '\n')
            (REPO / 'target/TEST-taste-before-355.txt').write_text(result['main'])
    else:
        for width, height in [(100, 30), (80, 24)]:
            for mono in [False, True]:
                with tempfile.TemporaryDirectory(prefix='tinkery-TEST-quiet-355-') as directory:
                    base = Path(directory).resolve()
                    baseline = os.environ.get('TINKERY_TASTE_BASELINE')
                    before = run(base, width, height, mono, Path(baseline), legacy=True) if baseline else None
                    result = run(base, width, height, mono, REPO / 'target/debug/tinkery')
                    for action, expected in json.loads(GOLDEN.read_text()).items():
                        assert result['copies'][action] == expected.replace('{ROOT}', str(base)).encode(), action
                    if before:
                        assert result['copies'] == before['copies'], 'parent/new payload bytes changed'
                        assert result['detail'] == before['detail'], 'detail content or ordering changed'
                    if (width, height, mono) == (100, 30, False):
                        (REPO / 'target/TEST-taste-after-355.txt').write_text(result['main'])
                    print(json.dumps(dict(journey='quiet Taste/main/detail/identical copies', width=width,
                                          height=height, no_color=mono, parent_comparison=bool(before),
                                          provider_calls=0, source_reads=3)))
