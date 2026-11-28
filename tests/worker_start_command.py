"""`seed start` is F2 without the screen: the same authorization record and the same refusals,
with stub SDLC admissions and no provider."""
import json
import os
from pathlib import Path
import sys
import tempfile

import test_launch as subprocess

REPO = Path(__file__).resolve().parents[1]
ENV = subprocess.environment()
if not os.environ.get('TINKERY_TEST_SEED_FIXTURE'):
    if not (REPO / 'target/debug/tinkery').is_file() or not (REPO / 'target/debug/examples/seed_fixture').is_file():
        subprocess.run(['cargo', '+stable', 'build', '--locked', '--bin', 'tinkery',
                        '--example', 'seed_fixture'], cwd=REPO, env=ENV, check=True)
    os.environ['TINKERY_TEST_SEED_FIXTURE'] = str(REPO / 'target/debug/examples/seed_fixture')

from worker_terminal import CAP, END, F2, MODEL, Fixture, Terminal, open_review
from worker_relay_authorization_terminal import authorization

ALLOWED = ('matching', 'standing')
REFUSED = ('absent', 'seed', 'model', 'cap', 'malformed', 'standing-gray')


def command(fixture, path, operator='agent', budget=True):
    args = [str(REPO / 'target/debug/tinkery'), '--operator', operator,
            '--sdlc-root', str(fixture.project), '--seed-session-root', str(fixture.sessions),
            '--record-root', str(fixture.root / 'TEST-records'),
            '--link-root', str(fixture.root / 'TEST-links')]
    if path is not None:
        args.extend(['--worker-authorization', str(path)])
    if budget:
        args.extend(['--worker-model', MODEL, '--worker-responses', str(CAP)])
    args.extend(['seed', 'start', str(fixture.session)])
    return subprocess.run(args, cwd=fixture.project, env=fixture.env, capture_output=True,
                          text=True, timeout=120)


def record(fixture):
    """The start's authorization record with this fixture's own folder named generically."""
    build = Path(fixture.read()['run']['checkout'])
    path = build / 'runs/test-worker/worker-authorization.jsonl'
    if not path.exists():
        return None
    return path.read_text().replace(str(fixture.root), 'TEST-ROOT')


def by_screen(case):
    with tempfile.TemporaryDirectory(prefix='tinkery-TEST-start-screen-') as directory:
        fixture = Fixture(Path(directory).resolve(), gray=case == 'standing-gray')
        path = authorization(fixture, case)
        terminal = Terminal(fixture, 100, 30, operator='agent', authorization=path, mono=True)
        try:
            open_review(terminal)
            terminal.send(END, 'esc back')
            os.write(terminal.master, F2)
            terminal.wait(lambda: 'Request refused or uncertain:' in terminal.flat()
                          or 'Worker: turn ended;' in terminal.flat())
            return terminal.flat(), len(fixture.admissions()), record(fixture) if fixture.admissions() else None
        finally:
            terminal.close()


def by_command(case):
    with tempfile.TemporaryDirectory(prefix='tinkery-TEST-start-command-') as directory:
        fixture = Fixture(Path(directory).resolve(), gray=case == 'standing-gray')
        path = authorization(fixture, case)
        originals = {name: (fixture.session / name).read_bytes()
                     for name in ('seed-contract.md', 'ledger.json', 'seed-confirmation.json')}
        result = command(fixture, path)
        for name, original in originals.items():
            assert (fixture.session / name).read_bytes() == original, 'start changed goal/seed consent'
        assert not (fixture.root / 'unexpected-provider').exists()
        if case in REFUSED:
            assert result.returncode != 0, result
            assert not fixture.mutations(), f'unauthorized admission: seed start created work\n{result}'
            assert not fixture.admissions()
            return result, 0, None
        assert result.returncode == 0, result
        started = json.loads(result.stdout)
        assert started['run'] == 'test-worker' and started['step'] == 'build', started
        assert started['model'] == MODEL and started['response_cap'] == CAP, started
        assert 'Tink Contract SHA-256:' in result.stderr, 'the F2 disclosure was not shown'
        return result, len(fixture.admissions()), record(fixture)


def same_as_screen(case):
    shown, screen_admissions, screen_record = by_screen(case)
    result, admissions, start_record = by_command(case)
    assert admissions == screen_admissions, (case, admissions, screen_admissions)
    if case in REFUSED:
        # The refusal before its retry words; the screen says F2, the command names itself.
        reason = ' '.join(result.stderr.split()).split('Error: ', 1)[1].strip('"')
        reason = reason.split(' Nothing was started')[0]
        assert reason and reason in shown, f'{case}: command refused with {reason!r}; screen showed:\n{shown}'
    else:
        assert start_record == screen_record, f'{case}:\n{start_record}\n!=\n{screen_record}'
    print(f'PASS seed start {case} matches F2')


def needs_allowance_and_checkout():
    with tempfile.TemporaryDirectory(prefix='tinkery-TEST-start-command-') as directory:
        fixture = Fixture(Path(directory).resolve())
        path = authorization(fixture, 'matching')
        result = command(fixture, path, budget=False)
        assert result.returncode != 0 and 'No worker allowance' in result.stderr, result
        assert not fixture.mutations()
        undeclared = subprocess.run(
            [str(REPO / 'target/debug/tinkery'), '--sdlc-root', str(fixture.project),
             'seed', 'start', str(fixture.session)],
            cwd=fixture.project, env=fixture.env, capture_output=True, text=True, timeout=60)
        assert undeclared.returncode != 0 and '--operator human|agent' in undeclared.stderr, undeclared
        rootless = subprocess.run(
            [str(REPO / 'target/debug/tinkery'), '--operator', 'agent',
             '--seed-session-root', str(fixture.sessions), 'seed', 'start', str(fixture.session)],
            cwd=fixture.project, env=fixture.env, capture_output=True, text=True, timeout=60)
        assert rootless.returncode != 0 and 'needs --sdlc-root' in rootless.stderr, rootless
        # --snapshot promises no model requests, so it never starts a worker.
        snapshot = subprocess.run(
            [str(REPO / 'target/debug/tinkery'), '--operator', 'agent', '--snapshot',
             '--sdlc-root', str(fixture.project), '--seed-session-root', str(fixture.sessions),
             '--worker-authorization', str(path), '--worker-model', MODEL,
             '--worker-responses', str(CAP), 'seed', 'start', str(fixture.session)],
            cwd=fixture.project, env=fixture.env, capture_output=True, text=True, timeout=60)
        assert snapshot.returncode != 0 and 'drop --snapshot' in snapshot.stderr, snapshot
        assert not fixture.mutations() and not fixture.admissions(), 'a --snapshot start created work'
        # A second start of the same Tink is refused, as Approve refuses it.
        first = command(fixture, path)
        assert first.returncode == 0, first
        again = command(fixture, path)
        assert again.returncode != 0 and 'already started' in again.stderr, again
        assert len(fixture.admissions()) == 1, 'one Tink admitted two workers'
    print('PASS seed start needs an allowance, a declaration and a checkout, and starts once')


def a_human_types_the_start():
    """A human's typed `seed start` is their own start: no relayed words and no screen."""
    with tempfile.TemporaryDirectory(prefix='tinkery-TEST-start-human-') as directory:
        fixture = Fixture(Path(directory).resolve())
        result = command(fixture, None, operator='human')
        assert result.returncode == 0, result
        started = json.loads(result.stdout)
        assert started['run'] == 'test-worker' and started['model'] == MODEL, started
        assert 'Tink Contract SHA-256:' in result.stderr, 'the F2 disclosure was not shown'
        assert len(fixture.admissions()) == 1, fixture.admissions()
    # The same command from an agent without relayed words starts nothing.
    with tempfile.TemporaryDirectory(prefix='tinkery-TEST-start-human-') as directory:
        fixture = Fixture(Path(directory).resolve())
        agent = command(fixture, None)
        assert agent.returncode != 0 and not fixture.mutations() and not fixture.admissions(), agent
    print('PASS seed start by a human needs only the typed command; an agent needs relayed words')


if __name__ == '__main__':
    needs_allowance_and_checkout()
    a_human_types_the_start()
    for case in (*ALLOWED, *REFUSED):
        same_as_screen(case)
