import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import shutil
import sys
import tempfile
import time
import unittest
from unittest import mock

import test_launch

ROOT = Path(__file__).resolve().parents[1]


def run_journey():
    import runpy
    try:
        runpy.run_path(str(ROOT / 'tests/approve_terminal.py'), run_name='__main__')
    except BaseException as error:
        trace = error.__traceback__
        while trace:
            if trace.tb_frame.f_code.co_name == 'journey':
                state = trace.tb_frame.f_locals
                screen = state.get('screen')
                if screen is not None:
                    print(f'TEST failed frame: {screen.frames=}, {screen.synchronized=}, '
                          f'{screen.pending=!r}, raw_tail={bytes(state.get("output", b""))[-8192:]!r}',
                          file=sys.stderr)
            trace = trace.tb_next
        raise


def benchmark(destination, runs):
    env = test_launch.environment()
    test_launch.run(['cargo', '+stable', 'build', '--locked', '--bin', 'tinkery',
                     '--example', 'seed_fixture'], cwd=ROOT, env=env, check=True)
    env['TINKERY_TEST_SEED_FIXTURE'] = str(ROOT / 'target/debug/examples/seed_fixture')
    journey = ROOT / 'tests/approve_terminal.py'
    source = journey.read_bytes()
    revision = test_launch.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT,
                                       env=env, text=True).strip()
    result = {'revision': revision, 'journey_sha256': hashlib.sha256(source).hexdigest(),
              'platform': platform.platform(), 'python': sys.version,
              'logical_cpus': os.cpu_count(), 'wait_scale': env.get('TINKERY_TEST_WAIT_SCALE', '1'),
              'mode': 'standalone, serial, warm build', 'attempts': []}
    destination.parent.mkdir(parents=True, exist_ok=True)
    for index in range(runs):
        if journey.read_bytes() != source:
            raise RuntimeError('Approve journey changed during the benchmark.')
        attempt = {'run': index + 1, 'state': 'started'}
        result['attempts'].append(attempt)
        destination.write_text(json.dumps(result, indent=2) + '\n')
        started = time.monotonic()
        completed = test_launch.run([sys.executable, __file__, '--journey'], cwd=ROOT, env=env,
                                    capture_output=True, stdin=test_launch.DEVNULL, start_new_session=True)
        log = destination.with_name(f'{destination.stem}-{index + 1}.log')
        log.write_bytes(completed.stdout + completed.stderr)
        attempt.update(seconds=time.monotonic() - started, exit_code=completed.returncode,
                       state='passed' if completed.returncode == 0 else 'failed', log=log.name)
        destination.write_text(json.dumps(result, indent=2) + '\n')
        print(json.dumps(attempt), flush=True)
        if completed.returncode:
            raise SystemExit(completed.returncode)


class FixtureTests(unittest.TestCase):
    def setUp(self):
        import approve_terminal
        self.approve = approve_terminal
        self.approve.fixture_template.cache_clear()
        self.addCleanup(self.approve.fixture_template.cache_clear)
        temporary = tempfile.TemporaryDirectory(prefix='tinkery-TEST-approve-copies-')
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)

    def snapshot(self, root):
        return {str(path.relative_to(root)): path.read_bytes()
                for path in root.rglob('*') if path.is_file()}

    def test_real_ledger_states_and_tall_goal_survive_copying(self):
        import session_fixture
        for tall in (False, True):
            with self.subTest(tall=tall):
                root = self.root / str(tall)
                expected = self.approve.copy_sessions(root, tall)
                self.assertEqual(len(expected), 2)
                sessions = sorted(root.iterdir())
                self.assertEqual(len(sessions), 5)
                verified = [path for path in sessions
                            if session_fixture.status(path)['handoff']['verified']]
                self.assertEqual(set(verified), set(expected))
                unconfirmed = [path for path in sessions if path not in expected]
                self.assertEqual(sum((path / 'seed-confirmation.json').is_file()
                                     for path in unconfirmed), 1)
                self.assertEqual(sum((path / 'seed-contract.md').is_file()
                                     for path in unconfirmed), 2)
                question = next(path for path in sessions if not (path / 'seed-contract.md').exists())
                self.assertEqual(session_fixture.load(question)['current_question'], 'storage')
                goal = session_fixture.load(expected[0])['goal']
                self.assertEqual('TEST goal line 19' in goal, tall)
                for path in sessions:
                    self.assertEqual(session_fixture.load(path)['session_id'], path.name)

    def test_mutating_or_deleting_a_copy_cannot_change_the_template_or_next_copy(self):
        for tall in (False, True):
            with self.subTest(tall=tall):
                template, names = self.approve.fixture_template(tall)
                original = self.snapshot(Path(template.name))
                first_root = self.root / f'first-{tall}'
                first = self.approve.copy_sessions(first_root, tall)
                self.assertEqual([path.name for path in first], list(names))
                self.assertEqual(self.snapshot(first_root), original)
                for relative in original:
                    copied = first_root / relative
                    self.assertFalse(os.path.samefile(copied, Path(template.name) / relative))
                    copied.write_bytes(b'TEST changed copy')
                shutil.rmtree(first[0])
                second_root = self.root / f'second-{tall}'
                self.approve.copy_sessions(second_root, tall)
                self.assertEqual(self.snapshot(Path(template.name)), original)
                self.assertEqual(self.snapshot(second_root), original)

    def test_each_template_is_built_once_and_destinations_must_be_fresh(self):
        with mock.patch.object(self.approve, 'make', wraps=self.approve.make) as make:
            for tall in (False, True):
                for index in range(2):
                    self.approve.copy_sessions(self.root / f'{tall}-{index}', tall)
            self.assertEqual(make.call_count, 10)
        with self.assertRaises(FileExistsError):
            self.approve.copy_sessions(self.root / 'False-0', False)


if __name__ == '__main__':
    if '--journey' in sys.argv:
        run_journey()
    elif '--benchmark' in sys.argv:
        parser = argparse.ArgumentParser()
        parser.add_argument('--benchmark', type=Path, required=True)
        parser.add_argument('--runs', type=int, choices=(1, 2, 3), default=3)
        args = parser.parse_args()
        benchmark(args.benchmark.resolve(), args.runs)
    else:
        unittest.main()
