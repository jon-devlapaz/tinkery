"""Visible review is required for confirmation; real PTY, TEST roots, stub providers."""
import copy
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
from agent_operation_terminal import ACTOR
from shape_answer_terminal import Run, F2, F3, helper
from vt_screen import Screen

GOAL = 'I keep refusals calm.\nWhy: the work is stuck.\nDone when: Refusals never show private text.'


def pump(run, seconds=.2):
    end = time.monotonic() + seconds
    while time.monotonic() < end or (run.screen.synchronized and time.monotonic() < end + 3):
        if select.select([run.master], [], [], .02)[0]:
            run.screen.feed(os.read(run.master, 65536))
    assert not run.screen.synchronized


def resize_with_input(run, width, height, data=b''):
    # Pause the child so resize and input are both pending before it can handle either.
    pump(run)
    os.kill(run.process.pid, signal.SIGSTOP)
    _, status = os.waitpid(run.process.pid, os.WUNTRACED)
    assert os.WIFSTOPPED(status)
    try:
        run.width = width
        run.screen = Screen(width, height)
        fcntl.ioctl(run.slave, termios.TIOCSWINSZ, struct.pack('HHHH', height, width, 0, 0))
        if data:
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
        actor = copy.deepcopy(ACTOR)
        actor['source'] += ' TEST permission covers this temporary goal only.' * 40
        run = Run(Path(directory), 100, 30, 0, authorization=actor)
        try:
            run.wait('1  think')
            run.send(b'1', "What's on your mind?")
            run.type('Refusals feel noisy.')
            run.send(F2, 'May refusals show private text?')
            run.type('never')
            run.send(F2, "Nothing I'm unsure about")
            run.send(F3, 'type confirm')
            if read:
                run.send(b'\x1b[6~' * 200, 'blast radius: small')
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
                os.write(run.master, b'\x1b[6~' * 200 + b'\x7fx' + F3
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
                run.send(b'\x1b[6~' * 200, 'blast radius: small')
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


if __name__ == '__main__':
    for size in [(40, 12), (79, 24), (80, 23)]:
        for read in [False, True]:
            journey(*size, read=read)
    journey(80, 24, read=True, supported=True)
    os.environ['NO_COLOR'] = '1'
    journey(80, 24, read=True, supported=True)
