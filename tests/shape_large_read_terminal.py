"""A seed above 65,535 rows stays unread until its actual tail appears; real PTYs, zero calls."""
import fcntl
import json
import os
from pathlib import Path
import select
import struct
import tempfile
import termios
import time

from shape_answer_terminal import F3, Run, files, helper
from shape_terminal import make, seed
from vt_screen import Screen


def page_down(run, count, deadline=None):
    """Drain output while sending all keys, including partial PTY writes."""
    pending = memoryview(b'\x1b[6~' * count)
    os.set_blocking(run.master, False)
    deadline = time.monotonic() + 60 if deadline is None else deadline
    try:
        while pending:
            assert time.monotonic() < deadline, run.screen.text()
            readable, writable, _ = select.select([run.master], [run.master], [], .02)
            if readable:
                run.screen.feed(os.read(run.master, 65536))
            if writable:
                try:
                    sent = os.write(run.master, pending[:4096])
                    pending = pending[sent:]
                except BlockingIOError:
                    pass
    finally:
        os.set_blocking(run.master, True)


def read_to_end(run):
    """Acknowledge each small key batch before sending another; stop at the actual end."""
    started = time.monotonic()
    deadline = started + 60

    def wait(text, phase, batch):
        wait_started = time.monotonic()
        wait_seconds = min(20, max(0, deadline - wait_started))
        try:
            run.wait(text, timeout=wait_seconds)
        except AssertionError as error:
            raise AssertionError(f'Batch {batch + 1}, {phase}: '
                                 f'{time.monotonic() - started:.1f}s elapsed, '
                                 f'{time.monotonic() - wait_started:.1f}s waited '
                                 f'of {wait_seconds:.1f}s wait budget.\n{error}') from error

    for batch in range(128):
        assert time.monotonic() < deadline, f'Large seed never reached its end:\n{run.screen.text()}'
        # Keep the keys plus F5 below Crossterm's 1 KiB input read.
        page_down(run, 128, deadline)
        # F5 can appear only after these PgDn keys have all been consumed.
        os.write(run.master, b'\x1b[15~')
        wait('Your goal, as you confirmed it', 'opening F5', batch)
        os.write(run.master, b'\x1b[15~')
        wait('pgup/dn read', 'closing F5', batch)
        if any(line.strip() == 'End of seed' for line in run.screen.text().splitlines()):
            print(f'Large seed reached its tail in {batch + 1} batches, '
                  f'{time.monotonic() - started:.1f}s', flush=True)
            return
    raise AssertionError(f'Large seed exceeded the bounded reading batches:\n{run.screen.text()}')


def journey(width, height):
    with tempfile.TemporaryDirectory(prefix='tinkery-TEST-large-seed-') as directory:
        root = Path(directory).resolve()
        session = make(root / 'TEST-sessions', 'draft', goal='TEST exact large goal', question=False)
        seed(session)
        path = session / 'seed-contract.md'
        original = path.read_text().replace('TEST draft.', 'TEST row\n' * 65_536)
        original += '\nTEST actual seed tail 中 e\u0301 👩‍💻\n'
        path.write_text(original)
        run = Run(root, width, height, budget=0, agent=True)
        try:
            run.open_shape()
            run.wait('Seed draft r1')
            words = 'TEST confirm the complete large seed'
            run.type(words)
            before = files(session)
            page_down(run, 10)
            run.send(F3, "PgDn to 'End of seed' first. Nothing was confirmed.")
            assert files(session) == before and all(line.strip() != 'End of seed' for line in run.screen.text().splitlines())
            # A resize must not turn the old row-count overflow into permission.
            new_width, new_height = (100, 30) if width == 80 else (80, 24)
            run.width = new_width
            run.screen = Screen(new_width, new_height)
            fcntl.ioctl(run.slave, termios.TIOCSWINSZ, struct.pack('HHHH', new_height, new_width, 0, 0))
            run.wait('pgup/dn read to end')
            run.send(F3, "PgDn to 'End of seed' first. Nothing was confirmed.")
            assert files(session) == before
            read_to_end(run)
            run.wait('TEST actual seed tail', timeout=60)
            assert all(word in run.screen.text() for word in ['中', 'e\u0301', '👩‍💻'])
            run.wait('End of seed')
            run.wait('F3 confirm seed r1')
            assert files(session) == before and run.calls() == []
            run.send(F3, 'Seed ready:')
            assert helper.status(session)['handoff']['verified']
            receipt = json.loads((session / 'seed-confirmation.json').read_text())
            assert receipt['source'] == words
            confirmed = path.read_text()
            without_note = '\n'.join(line for line in confirmed.split('\n') if not line.startswith('Confirmed by:'))
            assert without_note.replace('status: confirmed for intake', 'status: draft', 1) == original
            assert run.calls() == [], 'Reading or confirming called a model'
            print(f'Large Shape seed passed {width}x{height} -> {new_width}x{new_height}: '
                  f'{len(original.encode())} bytes, >65,535 rows, actual tail and exact bytes, zero calls')
        finally:
            run.close()


if __name__ == '__main__':
    for size in [(80, 24), (100, 30)]:
        journey(*size)
