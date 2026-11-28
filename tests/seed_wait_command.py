"""`tinkery seed wait` (#1059): one read-only command that blocks until the driving agent is
needed, then prints one JSON result. A fake SDLC and a stub Pi; no provider, no model calls."""
import json
import os
from pathlib import Path
import tempfile
import threading
import time

import test_launch as subprocess

REPO = Path(__file__).resolve().parents[1]
ENV = subprocess.environment()
if not os.environ.get('TINKERY_TEST_SEED_FIXTURE'):
    if not (REPO / 'target/debug/tinkery').is_file() or not (REPO / 'target/debug/examples/seed_fixture').is_file():
        subprocess.run(['cargo', '+stable', 'build', '--locked', '--bin', 'tinkery',
                        '--example', 'seed_fixture'], cwd=REPO, env=ENV, check=True)
    os.environ['TINKERY_TEST_SEED_FIXTURE'] = str(REPO / 'target/debug/examples/seed_fixture')

from worker_terminal import Fixture
from worker_relay_authorization_terminal import authorization
from worker_start_command import command as start
from seed_command import QUESTION, Run


def wait(fixture, seconds=30):
    result = subprocess.run(
        [str(REPO / 'target/debug/tinkery'), '--sdlc-root', str(fixture.project),
         '--wait-seconds', str(seconds), 'seed', 'wait', str(fixture.session)],
        cwd=fixture.project, env=fixture.env, capture_output=True, text=True, timeout=seconds + 30)
    assert result.returncode in (0, 2), result
    return result.returncode, json.loads(result.stdout)


def later(change, seconds=3):
    """Change the fake SDLC's state once a wait has begun, as a worker or verify would."""
    timer = threading.Timer(seconds, change)
    timer.start()
    return timer


def files(folder):
    return {str(p): p.read_bytes() for p in folder.rglob('*') if p.is_file() and p.name != '.writer.lock'}


def worker_events():
    with tempfile.TemporaryDirectory(prefix='tinkery-TEST-seed-wait-') as directory:
        fixture = Fixture(Path(directory).resolve())
        session = files(fixture.session)
        code, result = wait(fixture)
        assert (code, result['event'], result['run']) == (0, 'confirmed', None), result
        assert 'seed start' in result['next'], result
        assert start(fixture, authorization(fixture, 'matching')).returncode == 0
        before = len(fixture.mutations())

        # The fake worker's turn has ended: the agent is needed now, with no wait.
        code, result = wait(fixture)
        assert (code, result['event'], result['run']) == (0, 'worker-finished', 'test-worker'), result
        assert result['waited_seconds'] < 2 and 'TEST inspect the worker report' in result['next'], result

        def running(state):
            state['turn'] = 'not-ended'
        fixture.change(running)
        code, result = wait(fixture, seconds=2)
        assert (code, result['event'], result['worker']) == (2, 'timeout', 'running'), result

        later(lambda: fixture.change(lambda state: state.update(turn='ended')))
        code, result = wait(fixture)
        assert (code, result['event']) == (0, 'worker-finished'), result
        assert result['waited_seconds'] >= 2, result

        fixture.change(running)
        later(lambda: fixture.change(lambda state: state['step'].update(state='stopped', stopped_by='cap')))
        code, result = wait(fixture)
        assert (code, result['event'], result['worker']) == (0, 'cap', 'stopped by response cap'), result

        def verifying(state):
            state['step'].update(state='completed')
            state['step'].pop('stopped_by')
            state['run']['verification_status'] = 'running'
        fixture.change(verifying)
        later(lambda: fixture.change(lambda state: state['run'].update(verification_status='failed')))
        code, result = wait(fixture)
        assert (code, result['event'], result['verification']) == (0, 'check-failed', 'failed'), result

        fixture.change(lambda state: state['run'].update(verification_status='running'))
        later(lambda: fixture.change(lambda state: state['run'].update(verification_status='current')))
        code, result = wait(fixture)
        assert (code, result['event']) == (0, 'verified'), result

        assert len(fixture.mutations()) == before, 'seed wait changed the run'
        assert files(fixture.session) == session, 'seed wait changed the session'
        assert not (fixture.root / 'unexpected-provider').exists()
    print('PASS seed wait returns on each worker, cap and verify event, and times out with exit 2')


def shape_events():
    with tempfile.TemporaryDirectory(prefix='tinkery-TEST-seed-wait-shape-') as directory:
        run = Run(directory, [QUESTION])
        session = run.ok('new', 'goal.txt', 'TEST issue 1059', drafting=False).strip()
        env = subprocess.environment({**os.environ, 'TINKERY_TEST_DIR': str(run.cwd)})
        waiting = subprocess.Popen([str(REPO / 'target/debug/tinkery'), '--wait-seconds', '30', 'seed', 'wait',
                                    session], cwd=run.cwd, env=env, stdout=subprocess.PIPE, text=True)
        time.sleep(3)
        assert waiting.poll() is None, 'seed wait returned before the Shape step'
        assert json.loads(run.ok('step', session, 'look'))['now']['stage'] == 'question'
        out, _ = waiting.communicate(timeout=30)
        result = json.loads(out)
        assert (waiting.returncode, result['event']) == (0, 'decision'), result
        assert result['current_question'] == 'probe-real-sample', result
        assert run.calls() == [], 'seed wait or the look reached the stub model'
    print('PASS seed wait returns when a Shape step elsewhere saves a question')


def refusals():
    with tempfile.TemporaryDirectory(prefix='tinkery-TEST-seed-wait-refused-') as directory:
        folder = Path(directory)
        result = subprocess.run([str(REPO / 'target/debug/tinkery'), 'seed', 'wait', str(folder)],
                                capture_output=True, text=True, timeout=30, env=ENV)
        assert result.returncode == 1 and not result.stdout and 'Tinkery refused' in result.stderr, result
        result = subprocess.run([str(REPO / 'target/debug/tinkery'), '--wait-seconds', 'soon', 'seed', 'wait',
                                 str(folder)], capture_output=True, text=True, timeout=30, env=ENV)
        assert result.returncode != 0 and '--wait-seconds needs a number' in result.stderr, result
    print('PASS seed wait refuses a folder that is not a session, and a malformed time limit')


if __name__ == '__main__':
    refusals()
    shape_events()
    worker_events()
