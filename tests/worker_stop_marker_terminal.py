"""s records a cap stop only if the worker log shows the cap marker; c does nothing.

Fixture logs and the fake SDLC boundary only; no providers.
"""
from pathlib import Path
import tempfile

from worker_terminal import CAP, END, F2, HOME, Fixture, Terminal, open_review, write_logs


def stop(case):
    with tempfile.TemporaryDirectory(prefix='tinkery-TEST-stop-marker-') as directory:
        fixture = Fixture(Path(directory).resolve(), scenario='normal')
        terminal = Terminal(fixture, 80, 24, mono=True)
        try:
            open_review(terminal)
            terminal.send(HOME, 'Review worker start')
            terminal.send(END, 'F2 start worker')
            terminal.send(F2, 'Request in progress')
            terminal.wait(lambda: 'Worker: turn ended; completion still unrecorded' in terminal.flat())
            build = Path(fixture.read()['build'])
            if case == 'marker':
                write_logs(build, marker=True)
            elif case == 'no-marker':
                write_logs(build, marker=False)
            elif case == 'earlier-marker-only':
                write_logs(build, marker=True)
                write_logs(build, marker=False, name='build.resume-200.pi.log')
            elif case == 'c':
                write_logs(build, marker=True)
                assert 'c cap stop' not in terminal.flat() and 'c records' not in terminal.flat()
                terminal.send(b'?', 'Worker actions')
                assert 'c records' not in terminal.flat() and 'cap stop' not in terminal.flat()
                terminal.send(b'\x1b', 'Worker: turn ended')
                terminal.send(b'c', 'Worker: turn ended')
                assert fixture.read()['step']['state'] == 'active'
                assert not any(c['argv'][:4] == ['step', 'test-worker', 'build', 'complete'] for c in fixture.calls())
                print('PASS stop c: key does nothing and is not listed')
                return
            terminal.send(b's', 'Worker: stopped by')
            expected = 'cap' if case == 'marker' else 'operator'
            step = fixture.read()['step']
            assert step['state'] == 'stopped' and step['stopped_by'] == expected, (case, step)
            assert not (fixture.root / 'unexpected-provider').exists()
            print(f'PASS stop {case}: stopped_by == {expected}')
        finally:
            terminal.close()


if __name__ == '__main__':
    for case in ('marker', 'no-marker', 'earlier-marker-only', 'c'):
        stop(case)
