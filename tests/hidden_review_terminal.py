"""Visible review is required for confirmation; real PTY, TEST roots, stub providers."""
import fcntl
import json
import os
import select
import signal
import struct
import tempfile
import termios
import time
from pathlib import Path
from agent_operation_terminal import ACTOR, LONG_WHY
from shape_answer_terminal import Run, F1, F2, helper
from vt_screen import Screen

GOAL = f'I keep refusals calm.\nWhy: {LONG_WHY}.\nDone when: Refusals never show private text.'


def pump(run, seconds=.2):
    end = time.monotonic() + seconds
    while time.monotonic() < end or (run.screen.synchronized and time.monotonic() < end + 3):
        if select.select([run.master], [], [], .02)[0]:
            run.screen.feed(os.read(run.master, 65536))
    assert not run.screen.synchronized


def resize_with_input(run, width, height, data=b'', input_first=False):
    # Pause the child so resize and input are both pending before it can handle either.
    pump(run)
    os.kill(run.process.pid, signal.SIGSTOP)
    _, status = os.waitpid(run.process.pid, os.WUNTRACED)
    assert os.WIFSTOPPED(status)
    try:
        run.width = width
        run.screen = Screen(width, height)
        if data and input_first:
            os.write(run.master, data)
        fcntl.ioctl(run.slave, termios.TIOCSWINSZ, struct.pack('HHHH', height, width, 0, 0))
        if data and not input_first:
            os.write(run.master, data)
    finally:
        os.kill(run.process.pid, signal.SIGCONT)
    pump(run, .4)


def no_save(run):
    assert not run.records.exists() and not run.sessions.exists(), 'hidden review wrote a save'


def input_is(run, word):
    pump(run)
    assert any(line.strip() == word for line in run.screen.text().splitlines()), run.screen.text()


def journey(width, height, read=False, supported=False):
    with tempfile.TemporaryDirectory(prefix='tinkery-TEST-hidden-review-') as directory:
        actor = ACTOR
        run = Run(Path(directory), 100, 30, 4, agent=True, why=LONG_WHY)
        try:
            run.wait('1  think')
            run.send(b'1', "What's on your mind?")
            run.type('Refusals feel noisy.')
            run.send(F2, 'May refusals show private text?')
            run.type('never')
            run.send(F2, "Ready for your review")
            run.choose('Review the goal', 'You are confirming')
            if read:
                run.send(b'\x1b[6~' * 200, 'Done when: Refusals never show')
            word = 'confirm' if read else 'conf'
            os.write(run.master, word.encode())
            input_is(run, word)
            no_save(run)
            resize_with_input(run, width, height, b'\r')
            if supported:
                # A smaller supported view has more unread content: recalculate before Enter.
                run.wait('Read to the end first:')
                no_save(run)
            else:
                run.wait('Need 80×24.')
                no_save(run)
                os.write(run.master, b'\x1b[6~' * 200 + b'\x7fx' + F2
                         + b'\x1b[200~hidden paste\x1b[201~\r')
                pump(run, .5)
                no_save(run)
                assert 'Need 80×24.' in run.screen.text()
                # Input queued on restoration must survive the repaint, exactly once.
                resize_with_input(run, 100, 30, b'\r' if read else b'irm\r')
                if not read:
                    run.wait('Read to the end first:')
                    no_save(run)
                    input_is(run, 'confirm')
            if supported or not read:
                run.send(b'\x1b[6~' * 200, 'Done when: Refusals never show')
                run.send(b'\r', 'Saved.')
            else:
                run.wait('Saved.')
            written = list(run.records.iterdir())
            assert len(written) == len(list(run.sessions.iterdir())) == 1
            record = json.loads(written[0].read_text())
            ledger = helper.load(run.sessions / record['seed_me_session'])
            assert record['goal'] == ledger['goal'] == GOAL
            assert record['goal_confirmation'] == actor
            assert record['turns'][0]['text'] == 'Refusals feel noisy.'
            assert any(turn.get('answer') == 'never' for turn in record['turns'])
            assert not run.calls()
            print(f'confirmation resize 100×30 → {width}×{height}, read={read}: '
                  'queued input preserved; hidden/unread save refused; exact goal and actor saved')
        finally:
            run.close()


def stale_mouse(input_first):
    with tempfile.TemporaryDirectory(prefix='tinkery-TEST-resized-mouse-') as directory:
        run = Run(Path(directory), 100, 30, 4, agent=True)
        try:
            run.wait('1  think')
            run.send(b'1', "What's on your mind?")
            run.type('Refusals feel noisy.')
            run.send(F2, 'May refusals show private text?')
            run.type('never')
            run.send(F2, "Ready for your review")
            run.choose('Review the goal', 'You are confirming')
            os.write(run.master, b'confirm')
            input_is(run, 'confirm')
            assert run.screen.text().splitlines()[27][50:54] == 'Back'
            # This old-layout click is outside Back, but lands on Back after resize.
            click = b'\x1b[<0;41;22M'
            # Keep queued mouse protection through the 64-event batch boundary too.
            resize_with_input(run, 80, 24, b'\x01' * 80 + click, input_first=input_first)
            run.wait('type confirm')
            input_is(run, 'confirm')
            assert run.screen.text().splitlines()[21][40:44] == 'Back'
            no_save(run)
            # Once the old queue drained, a deliberate click at the new Back works.
            run.send(click, "Ready for your review")
            assert 'type confirm' not in run.screen.text()
            no_save(run)
            assert not run.calls()
            print(f'resized mouse, input_first={input_first}: stale click ignored across batches; '
                  'review/input retained; fresh Back click works')
        finally:
            run.close()


def guidance(width, height, long):
    """Fresh, fitting and resized review guidance follows the same guarded read bound."""
    with tempfile.TemporaryDirectory(prefix='tinkery-TEST-review-guidance-') as directory:
        root = Path(directory)
        why = LONG_WHY if long else 'the work is stuck'
        goal = f'I keep refusals calm.\nWhy: {why}.\nDone when: Refusals never show private text.'
        run = Run(root, width, height, 4, agent=True, why=why)
        try:
            run.wait('1  think')
            run.send(b'1', "What's on your mind?")
            run.type('Refusals feel noisy.')
            run.send(F2, 'May refusals show private text?')
            run.type('never')
            run.send(F2, 'Ready for your review')
            calls = (root / 'think-calls.jsonl').read_bytes()
            assert len(calls.splitlines()) == 4
            run.choose('Review the goal', 'You are confirming')
            prompt = 'PgDn: read to the end first.'
            if long:
                run.wait(prompt)
                assert 'type confirm' not in run.flat(), run.screen.text()
                assert 'PgDn read to end' in run.flat()
                run.send(F1, 'Esc closes help')
                assert 'Type confirm' not in run.flat()
                run.send(b'\x1b', prompt)
                run.send(b'confirm\r', 'Read to the end first:')
                no_save(run)
                input_is(run, 'confirm')
                run.send(b'\x1b[6~' * 200, 'type confirm')
                assert prompt not in run.flat()
                run.send(b'\x1b[5~', prompt)
                run.send(b'\x1b[6~' * 200, 'type confirm')
                # Width/height changes reuse the newly measured bound before Enter runs.
                if width == 100:
                    resize_with_input(run, 80, 24)
                    run.wait(prompt)
                    input_is(run, 'confirm')
                    run.send(b'\r', 'Read to the end first:')
                    no_save(run)
                    run.send(b'\x1b[6~' * 200, 'type confirm')
                    resize_with_input(run, 100, 30)
                    run.wait('type confirm')
                    assert prompt not in run.flat()
                # Correcting the typed word cannot move an already-read end (#316).
                run.send(b'\x7f\x1b[200~m\x1b[201~', 'confirm')
                run.send(b'\r', 'Saved.')
            else:
                run.wait('type confirm')
                assert prompt not in run.flat()
                run.send(F1, 'Type confirm, then Enter')
                run.send(b'\x1b', 'type confirm')
                run.send(b'confirm\r', 'Saved.')
            written = list(run.records.iterdir())
            assert len(written) == len(list(run.sessions.iterdir())) == 1
            record = json.loads(written[0].read_text())
            ledger = helper.load(run.sessions / record['seed_me_session'])
            assert record['goal'] == ledger['goal'] == goal
            assert record['turns'][0]['text'] == 'Refusals feel noisy.'
            assert record['goal_confirmation'] == ACTOR
            assert (root / 'think-calls.jsonl').read_bytes() == calls
            assert not run.calls()
            print(f'review guidance {width}×{height}, long={long}, mono={bool(os.environ.get("NO_COLOR"))}: '
                  'cue matches read gate; exact goal and input retained; no extra requests')
        finally:
            run.close()


if __name__ == '__main__':
    for mono in [False, True]:
        if mono:
            os.environ['NO_COLOR'] = '1'
        else:
            os.environ.pop('NO_COLOR', None)
        for size in [(80, 24), (100, 30)]:
            for long in [False, True]:
                guidance(*size, long)
    os.environ.pop('NO_COLOR', None)
    journey(40, 12, read=False)
    journey(40, 12, read=True)
    # One run per just-too-small edge, width and height.
    journey(79, 24, read=False)
    journey(80, 23, read=False)
    journey(80, 24, read=True, supported=True)
    os.environ['NO_COLOR'] = '1'
    for input_first in [False, True]:
        stale_mouse(input_first)
