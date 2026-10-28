"""Repo PR lists through fake gh: explicit reads, evidence, errors and cancellation."""
import json
import os
import sys
import time
import signal
import threading

from measure_terminal import SIZES, Terminal


def pr(number, state='OPEN', title=None, checks=None):
    return dict(number=number, title=title or f'TEST change {number}', state=state,
                url=f'https://github.com/TEST/repo/pull/{number}', author={'login': 'TEST-author'},
                createdAt='2026-01-01T00:00:00Z', updatedAt='2026-01-02T00:00:00Z',
                closedAt=None if state == 'OPEN' else '2026-01-02T00:00:00Z',
                mergedAt='2026-01-02T00:00:00Z' if state == 'MERGED' else None,
                headRefOid='abc123', statusCheckRollup=checks)


def fake(payload, mode='ok'):
    return f'''#!{sys.executable}
import json, os, pathlib, sys, time
with pathlib.Path('calls').open('a') as out:
    out.write(json.dumps(dict(args=sys.argv[1:], cwd=os.getcwd(), repo=os.getenv('GH_REPO'), pid=os.getpid())) + '\\n')
assert os.environ['GH_PROMPT_DISABLED'] == '1'
assert sys.stdin.read() == ''
mode = {mode!r}
if mode.startswith('held-') or mode == 'descendant':
    import subprocess
    child = subprocess.Popen([sys.executable, '-c', "import os,pathlib,time; p=pathlib.Path('helper-pid'); p.with_suffix('.tmp').write_text(str(os.getpid())); p.with_suffix('.tmp').replace(p); time.sleep(60)"],
                             start_new_session=mode.startswith('held-'), stdin=subprocess.DEVNULL,
                             stdout=None if mode == 'held-stdout' else subprocess.DEVNULL,
                             stderr=None if mode == 'held-stderr' else subprocess.DEVNULL)
    while not pathlib.Path('helper-pid').exists():
        time.sleep(.01)
if mode == 'slow':
    time.sleep(60)
if mode == 'oversized':
    print('x' * (4 * 1024 * 1024 + 1))
    sys.exit(0)
if mode == 'failure' or (mode == 'refresh-failure' and len(pathlib.Path('calls').read_text().splitlines()) > 1):
    print('TEST-private-error-never-display', file=sys.stderr)
    sys.exit(1)
if mode == 'malformed':
    print('{{')
else:
    print({json.dumps(payload)!r})
'''


def calls(terminal):
    return [json.loads(line) for line in terminal.calls.read_text().splitlines()]


def wait_for_call(terminal):
    """Wait for the first complete fake-gh record, then parse it strictly."""
    terminal.wait(lambda: terminal.calls.exists() and terminal.calls.read_bytes().endswith(b'\n'))
    return calls(terminal)[0]


def journey(width, height, no_color):
    payload = [pr(1, checks=[{'name': 'unit', 'conclusion': 'SUCCESS'}]),
               pr(2, 'MERGED', checks=[{'name': 'journey', 'conclusion': 'FAILURE'}]),
               pr(3, 'CLOSED')]
    t = Terminal(width, height, no_color, fake(payload))
    try:
        t.send(b'7', 'Nothing has been read yet')
        t.send(b'\r', 'Nothing has been read yet')
        t.send(b'?', 'q goes back to the menu')
        t.send(b'r', 'q goes back to the menu')
        t.send(b'\x1b', 'Nothing has been read yet')
        assert not t.calls.exists(), 'Navigation/help started gh'
        t.send(b'r', '1–3 of 3 PRs')
        text = t.screen.text()
        assert 'open' in text and 'merged' in text and 'closed' in text, text
        call = calls(t)[0]
        assert call['repo'] is None
        assert call['cwd'] == str(t.root)
        assert call['args'][2:4] == ['--state', 'all']
        assert 'updated:>=' in call['args'][5] and 'sort:updated-desc' in call['args'][5]
        assert call['args'][6:8] == ['--limit', '101']
        t.send(b'\r', 'unit: SUCCESS')
        assert 'Reported PR-head checks' in t.screen.text()
        assert 'abc123' in t.screen.text()
        t.send(b'\x1b', '1–3 of 3 PRs')
        t.send(b'\x1b[B', '> #2')
        t.send(b'\r', 'journey: FAILURE')
        assert '#2 · merged' in t.screen.text()
        t.send(b'\x1b', '> #2')
        t.send(b'\x1b[B', '> #3')
        t.send(b'\r', 'Checks unavailable or not recorded')
        assert '#3 · closed-unmerged' in t.screen.text()
        t.send(b'\x1b', '> #3')
        assert len(calls(t)) == 1, 'Details started another read'
        t.send(b'r', '1–3 of 3 PRs')
        t.wait(lambda: len(calls(t)) == 2)
        t.send(b'q', '1  think')
        t.send(b'7', 'Nothing has been read yet')
        t.send(b'q', '1  think')
        t.finish(expected_calls=2)
        print(json.dumps(dict(journey='prs', width=width, height=height, no_color=no_color,
                              gh_calls=2, provider_calls=0)))
    finally:
        t.close()


def cases():
    for payload, mode, expected in [
        ([], 'ok', 'No PRs found in this window'),
        ([pr(n) for n in range(1, 102)], 'ok', '1–7 of 100 PRs · more on GitHub'),
        ([], 'failure', 'GitHub read failed'),
        ([], 'malformed', 'unreadable PR list'),
        ([], 'oversized', 'incomplete or exceeded 4 MiB'),
    ]:
        t = Terminal(80, 24, True, fake(payload, mode))
        try:
            t.send(b'7', 'Nothing has been read yet')
            t.send(b'r', expected)
            assert 'TEST-private-error-never-display' not in t.screen.text()
            if mode != 'ok':
                assert 'No PRs found' not in t.screen.text()
            if payload:
                for _ in range(101):
                    os.write(t.master, b'\x1b[B')
                t.wait(lambda: '> #100' in t.screen.text())
            t.send(b'q', '1  think')
            t.finish(expected_calls=1)
        finally:
            t.close()
    t = Terminal(80, 24, True, fake([], 'slow'))
    try:
        t.send(b'7', 'Nothing has been read yet')
        t.send(b'r', 'Reading GitHub')
        pid = wait_for_call(t)['pid']
        t.resize(100, 30)
        t.wait(lambda: 'Reading GitHub' in t.screen.text())
        t.send(b'?', 'q goes back to the menu')
        start = time.monotonic()
        t.send(b'q', '1  think')
        assert time.monotonic() - start < 2
        try:
            os.kill(pid, 0)
        except ProcessLookupError:
            pass
        else:
            raise AssertionError('gh survived leaving Measure')
        t.send(b'7', 'Nothing has been read yet')
        t.send(b'q', '1  think')
        t.finish(expected_calls=1)
    finally:
        t.close()
    t = Terminal(80, 24, True, fake([]))
    try:
        (t.root / 'bin/gh').unlink()
        t.before = t.files()
        t.send(b'7', 'Nothing has been read yet')
        t.send(b'r', "Couldn't start gh")
        t.send(b'q', '1  think')
        t.finish()
    finally:
        t.close()
    t = Terminal(80, 24, True, fake([pr(1, title='TEST stale row')], 'refresh-failure'))
    try:
        t.send(b'7', 'Nothing has been read yet')
        t.send(b'r', 'TEST stale row')
        t.send(b'r', 'GitHub read failed')
        assert 'TEST stale row' not in t.screen.text()
        assert 'No PRs found' not in t.screen.text()
        t.send(b'q', '1  think')
        t.finish(expected_calls=2)
    finally:
        t.close()
    print('8 error, empty, partial, missing-gh, refresh and cancellation journeys passed')


def held_pipe_journey(stream, timeout=False):
    t = Terminal(100, 30, False, fake([], 'held-' + stream))
    release = None
    helper = None
    try:
        t.send(b'7', 'Nothing has been read yet')
        started = time.monotonic()
        t.send(b'r', 'Reading GitHub')
        direct = wait_for_call(t)['pid']
        ready = t.root / 'helper-pid'
        t.wait(ready.exists)
        helper = int(ready.read_text())
        def child_reaped():
            try:
                os.kill(direct, 0)
                return False
            except ProcessLookupError:
                return True
        t.wait(child_reaped)
        # Only the fixture owns this detached writer. Release a reverted blocking join safely.
        def stop_helper():
            try:
                os.kill(helper, signal.SIGKILL)
            except ProcessLookupError:
                pass
        release = threading.Timer(32 if timeout else 3, stop_helper)
        release.start()
        if timeout:
            t.wait(lambda: 'GitHub read exceeded 30 seconds' in t.screen.text(), seconds=34)
            assert time.monotonic() - started < 31.5, 'deadline waited for the detached writer'
            t.send(b'\x1b', '1  think')
        else:
            started = time.monotonic()
            t.send(b'\x1b', '1  think')
            assert time.monotonic() - started < 2, 'Escape waited for the detached writer'
        try:
            os.kill(direct, 0)
        except ProcessLookupError:
            pass
        else:
            raise AssertionError('direct gh child was not reaped')
        os.kill(helper, 0)  # Product cancellation must not kill an unrelated process group.
        stop_helper()
        release.cancel()
        release.join()
        ready.unlink()
        t.finish(expected_calls=1)
        print(json.dumps(dict(journey='held pipe', stream=stream, timeout=timeout, provider_calls=0)))
    finally:
        if release:
            release.cancel()
            release.join()
        if helper:
            try:
                os.kill(helper, signal.SIGKILL)
            except ProcessLookupError:
                pass
        t.close()


def ordinary_descendant_journey():
    helper = None
    t = Terminal(80, 24, True, fake([], 'descendant'))
    try:
        t.send(b'7', 'Nothing has been read yet')
        t.send(b'r', 'No PRs found in this window')
        helper = int((t.root / 'helper-pid').read_text())
        # Same-group descendants remain the product's responsibility.
        import test_launch as processes
        state = processes.run(['/bin/ps', '-o', 'stat=', '-p', str(helper)],
                              capture_output=True, text=True,
                              env=processes.environment(home=t.root / 'HOME')).stdout.strip()
        assert not state or state.startswith('Z'), 'ordinary descendant survived the read'
        (t.root / 'helper-pid').unlink()
        t.send(b'\x1b', '1  think')
        t.finish(expected_calls=1)
    finally:
        if helper:
            try:
                os.kill(helper, signal.SIGKILL)
            except ProcessLookupError:
                pass
        t.close()


def layout_journey(width, height, no_color):
    title = 'Wrap these exact Unicode words 漢字 e\u0301. ' * 70 + 'LAST-TITLE-WORD'
    payload = [pr(n, title=title if n == 1 else f'TEST row {n}') for n in range(1, 102)]
    t = Terminal(width, height, no_color, fake(payload))
    try:
        t.send(b'7', 'Nothing has been read yet')
        t.send(b'r', '1–7 of 100 PRs · more on GitHub')
        text = t.screen.text()
        assert 'Pull requests ·' not in text and 'tinkery › measure' not in text
        assert 'measure ·' in text
        assert 'GitHub · read at' not in text
        assert 'TEST row 7' in text and 'TEST row 8' not in text
        assert 'pg↑↓ title' in text
        for _ in range(100):
            os.write(t.master, b'\x1b[6~')
        t.wait(lambda: 'LAST-TITLE-WORD' in t.screen.text())
        t.send(b'?', 'GitHub read at:')
        assert 'Window: updated since' in t.screen.text()
        t.send(b'\x1b', 'LAST-TITLE-WORD')
        t.send(b'\x1b[B', '> #2')
        assert 'pg↑↓ title' not in t.screen.text()
        t.send(b'\r', '#2 · open')
        t.send(b'\x1b', '> #2')
        t.resize(80 if width != 80 else 100, 24)
        t.wait(lambda: '> #2' in t.screen.text())
        for _ in range(110):
            os.write(t.master, b'\x1b[B')
        t.wait(lambda: '> #100' in t.screen.text())
        assert '94–100 of 100 PRs · more on GitHub' in t.screen.text()
        assert len(calls(t)) == 1, 'Layout/navigation performed another read'
        t.send(b'q', '1  think')
        t.finish(expected_calls=1)
        print(json.dumps(dict(journey='quiet list/full selected title/help/resize/all rows',
                              width=width, height=height, no_color=no_color, gh_calls=1)))
    finally:
        t.close()


if __name__ == '__main__':
    for dimensions in SIZES:
        for no_color in (False, True):
            journey(*dimensions, no_color)
    cases()
    for stream in ('stdout', 'stderr'):
        held_pipe_journey(stream)
        held_pipe_journey(stream, timeout=True)
    ordinary_descendant_journey()
    for dimensions in SIZES + [(70, 49)]:
        for no_color in (False, True):
            layout_journey(*dimensions, no_color)
