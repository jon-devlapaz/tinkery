"""Exact-commit Taste previews: local Git fixture, fake server, stub opener, no providers."""
import fcntl
import hashlib
import json
import os
from pathlib import Path
import select
import shutil
import socket
import struct
import sys
import tempfile
import termios
import time
import test_launch as subprocess
from taste_terminal import FAKE, GOAL, REPO, RUN_ID, URL, fixture, tree
from vt_screen import Screen

SERVER = '''import http.server, json, os, subprocess, sys, time
from pathlib import Path
child = subprocess.Popen([sys.executable, '-c', 'import time; time.sleep(120)'])
record = Path(os.environ['TEST_PREVIEW_STARTED'])
staged = record.with_suffix('.tmp')
staged.write_text(json.dumps(dict(pid=os.getpid(), child=child.pid, cwd=os.getcwd(), marker=Path('marker').read_text())))
staged.replace(record)
time.sleep(.6)
http.server.HTTPServer(('127.0.0.1', int(sys.argv[1])), http.server.SimpleHTTPRequestHandler).serve_forever()
'''


def alive(pid):
    status = Path(f'/proc/{pid}/stat')
    if status.exists() and status.read_text().split()[2] == 'Z':
        return False
    try:
        os.kill(pid, 0)
        return True
    except ProcessLookupError:
        return False


def journey(width, height, mono, wrong_head=False):
    with tempfile.TemporaryDirectory(prefix='tinkery-TEST-preview-journey-') as directory:
        base = Path(directory).resolve()
        env = {**os.environ, 'HOME': str(base / 'HOME'), 'TERM': 'xterm-256color',
               'TINKERY_NODE': '/tinkery-TEST-no-node'}
        env.pop('NO_COLOR', None)
        if mono:
            env['NO_COLOR'] = '1'
        env = subprocess.environment(env, home=env['HOME'])
        repo = base / 'TEST-result-repo'
        repo.mkdir()
        git = shutil.which('git')

        def run_git(*args):
            return subprocess.check_output([git, *args], cwd=repo, env=env).decode().strip()

        run_git('init', '-q')
        run_git('config', 'user.name', 'TEST')
        run_git('config', 'user.email', 'TEST@example.invalid')
        with socket.socket() as sock:
            sock.bind(('127.0.0.1', 0))
            port = sock.getsockname()[1]
        spec = {'command': f'exec python3 preview.py {port}', 'url': f'http://127.0.0.1:{port}/'}
        (repo / 'tinkery.preview').write_text(json.dumps(spec) + '\n')
        (repo / 'preview.py').write_text(SERVER)
        (repo / 'marker').write_text('exact result')
        run_git('add', '.')
        run_git('commit', '-qm', 'TEST exact result')
        head = run_git('rev-parse', 'HEAD')
        (repo / 'marker').write_text('wrong result')
        run_git('commit', '-qam', 'TEST newer result not selected')
        other = run_git('rev-parse', 'HEAD')
        checkout = base / 'TEST-status'
        scripts = checkout / '_system/scripts'
        scripts.mkdir(parents=True)
        (scripts / 'sdlc.py').write_text(FAKE)
        seed = base / 'TEST-seed/seed-contract.md'
        seed.parent.mkdir()
        seed.write_text('TEST original')
        (seed.parent / 'ledger.json').write_text(json.dumps({'schema_version': 1, 'goal': GOAL}))
        state = fixture({'source': str(seed), 'path': 'runs/TEST-result/seed-contract.md',
                         'sha256': hashlib.sha256(seed.read_bytes()).hexdigest()})
        state['pr']['headRefOid'] = head
        statefile = base / 'state.json'
        statefile.write_text(json.dumps(state))
        helpers = base / 'bin'
        helpers.mkdir()
        (helpers / 'python3').symlink_to(sys.executable)
        (helpers / 'gh').write_text(f'#!{sys.executable}\n' + f'''import json, os, sys
from pathlib import Path
if sys.argv[1] == 'api':
    assert sys.argv[-1].endswith('tinkery.preview?ref={head}')
    if json.loads(Path(os.environ['TEST_TASTE_STATE']).read_text()).get('no_preview'):
        print('HTTP 404', file=sys.stderr)
        sys.exit(1)
    print({json.dumps(spec)!r})
    sys.exit(0)
''' + FAKE)
        (helpers / 'git').write_text(f'#!{sys.executable}\n' + f'''import os, sys
args = sys.argv[1:]
if 'fetch' in args:
    assert 'credential.helper=!gh auth git-credential' in args
    assert args[-2] == 'https://github.com/TEST/clinic-tycoon.git'
    assert args[-1] == {head!r}
    args[-2:] = [{str(repo)!r}, {other if wrong_head else head!r}]
os.execv({git!r}, [{git!r}, *args])
''')
        opened = base / 'opened'
        opener = f'#!{sys.executable}\nfrom pathlib import Path\nimport sys\nPath({str(opened)!r}).write_text(sys.argv[1])\n'
        for name in ['open', 'xdg-open']:
            (helpers / name).write_text(opener)
        for path in helpers.iterdir():
            path.chmod(0o755)
        started = base / 'started'
        env.update(PATH=f'{helpers}:/usr/bin:/bin', TEST_TASTE_STATE=str(statefile),
                   TEST_TASTE_CALLS=str(base / 'calls'), TEST_TASTE_PID=str(base / 'pid'),
                   TEST_PREVIEW_STARTED=str(started))
        master, slave = os.openpty()
        fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack('HHHH', height, width, 0, 0))
        screen = Screen(width, height)
        process = subprocess.Popen([str(REPO / 'target/debug/tinkery'), '--operator', 'agent', '--home',
                                    '--sdlc-root', str(checkout), '--seed-session-root', str(base / 'TEST-sessions'),
                                    '--taste-result', f'{RUN_ID}={URL}'], cwd=base, env=env,
                                   stdin=slave, stdout=slave, stderr=slave)
        original = tree(checkout)
        repo_original = tree(repo)

        def wait(predicate, seconds=10):
            until = time.monotonic() + seconds
            while time.monotonic() < until:
                if select.select([master], [], [], .02)[0]:
                    screen.feed(os.read(master, 65536))
                if predicate() and not screen.synchronized:
                    return
                assert process.poll() is None, screen.text()
            raise AssertionError(screen.text())

        def send(key, expected):
            os.write(master, key)
            wait(lambda: expected in screen.text())

        def stopped(record):
            wait(lambda: not alive(record['pid']) and not alive(record['child']))
            assert not Path(record['cwd']).exists(), 'temporary checkout survived cancellation'

        try:
            wait(lambda: '1  think' in screen.text())
            send(b'6', 'Press r to read results')
            assert not started.exists() and not opened.exists()
            send(b'r', '›  TEST café')
            send(b'\r', 'v run')
            text = screen.text().replace('\n', '').replace(' ', '')
            assert head in text and spec['command'].replace(' ', '') in text, screen.text()
            assert not started.exists() and not opened.exists(), 'read/navigation executed command'
            send(b'?', 'What Taste reads')
            os.write(master, b'v')
            time.sleep(.1)
            assert not started.exists(), 'help leaked preview execution'
            os.write(master, b'\x1b')
            wait(lambda: 'What Taste reads' not in screen.text() and 'v run "' in screen.text())
            os.write(master, b'v')
            if wrong_head:
                wait(lambda: 'Checkout commit disagrees' in screen.text())
                assert not started.exists() and not opened.exists(), 'wrong commit executed'
                send(b'\x1b', 'Preview stopped and reaped')
            else:
                wait(started.exists)
                record = json.loads(started.read_text())
                assert record['marker'] == 'exact result'
                assert record['cwd'] != str(repo) and record['cwd'] != str(checkout)
                assert not opened.exists(), 'opened browser before URL answered'
                wait(lambda: opened.exists() and 'sent http' in screen.text())
                assert opened.read_text() == spec['url']
                send(b'\x1b', 'Preview stopped and reaped')
                stopped(record)
                started.unlink()
                opened.unlink()
                os.write(master, b'v')
                wait(started.exists)
                record = json.loads(started.read_text())
                send(b'\x1b[21~', '1  think')  # F10 leaves and stops, including startup.
                stopped(record)
                send(b'6', 'Press r to read results')
                state['no_preview'] = True
                statefile.write_text(json.dumps(state))
                send(b'r', '›  TEST café')
                send(b'\r', 'Checks')
                send(b'v', 'No preview declared')
                assert 'tinkery.preview' in screen.text() and 'command' in screen.text(), screen.text()
                assert not opened.exists(), 'undeclared preview opened a browser'
            assert tree(checkout) == original and tree(repo) == repo_original
            print(json.dumps(dict(journey='Taste exact-head preview' if not wrong_head else 'Taste wrong-head refusal',
                                  width=width, height=height, no_color=mono, provider_calls=0,
                                  opener='stub', esc_reaped=not wrong_head, leave_reaped=not wrong_head)))
        finally:
            if process.poll() is None:
                os.write(master, b'\x1b[21~')
                wait(lambda: '1  think' in screen.text())
                os.write(master, b'q')
                until = time.monotonic() + 5
                while process.poll() is None and time.monotonic() < until:
                    if select.select([master], [], [], .02)[0]:
                        os.read(master, 65536)
                process.wait(timeout=1)
            os.close(master)
            os.close(slave)


if __name__ == '__main__':
    for width, height in [(100, 30), (80, 24)]:
        for mono in [False, True]:
            journey(width, height, mono)
            journey(width, height, mono, wrong_head=True)
