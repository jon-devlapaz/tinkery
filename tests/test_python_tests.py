"""Python tiers and fixture preparation use isolated fake tools, never product or provider calls."""
import ast
from collections import Counter
import json
import os
from pathlib import Path
import runpy
import shutil
import sys
import tempfile
import threading
import test_launch as subprocess
import unittest
from unittest import mock

ROOT = Path(__file__).resolve().parents[1]


class TierTests(unittest.TestCase):
    def setUp(self):
        api = runpy.run_path(str(ROOT / 'scripts/python-tests.py'), run_name='test_tiers')
        self.selected = api['selected']
        self.post_merge = api['POST_MERGE']
        self.names = lambda tier: [p.name for p in self.selected(tier)]

    def test_full_runs_every_test_file_once(self):
        helpers = {'vt_screen.py', 'session_fixture.py', 'test_launch.py', 'terminal_process.py'}
        files = sorted(p.name for p in (ROOT / 'tests').glob('*.py') if p.name not in helpers)
        self.assertEqual(sorted(self.names('full')), files)

    def test_pr_skips_exactly_the_post_merge_journeys(self):
        self.assertEqual(set(self.names('full')) - set(self.names('pr')), self.post_merge)
        self.assertIn('goal_confirmation_terminal.py', self.names('pr'))
        self.assertIn('agent_operation_terminal.py', self.names('pr'))
        for tier in ('pr', 'full'):
            self.assertNotIn('session_fixture.py', self.names(tier))
            self.assertNotIn('test_launch.py', self.names(tier))

    def test_every_file_lands_in_exactly_one_shard_in_each_tier(self):
        for tier in ('pr', 'full'):
            for count in (1, 2, 3, 8):
                with self.subTest(tier=tier, count=count):
                    shards = [self.selected(tier, index, count) for index in range(count)]
                    self.assertEqual(Counter(path for shard in shards for path in shard),
                                     Counter(self.selected(tier)))
                    self.assertTrue(all(shards))
                    self.assertEqual(shards, [self.selected(tier, index, count) for index in range(count)])

    def test_shards_balance_recorded_durations_and_keep_new_files(self):
        api = self.selected.__globals__
        with mock.patch.dict(api['DURATIONS'], {path.name: 1 for path in self.selected('full')}, clear=True):
            paths = self.selected('full')
            api['DURATIONS'][paths[0].name] = 100
            api['DURATIONS'].pop(paths[-1].name)
            shards = [self.selected('full', index, 3) for index in range(3)]
            self.assertIn(paths[-1], [path for shard in shards for path in shard])
            self.assertEqual(next(shard for shard in shards if paths[0] in shard), [paths[0]])

    def test_invalid_shards_are_refused(self):
        for index, count in ((0, 0), (0, -1), (-1, 3), (3, 3)):
            with self.subTest(index=index, count=count), self.assertRaises(SystemExit):
                self.selected('full', index, count)

    def test_unknown_tier_and_stale_names_are_refused(self):
        with self.assertRaises(SystemExit):
            self.selected('smoke')
        self.post_merge.add('TEST_removed_terminal.py')
        self.addCleanup(self.post_merge.discard, 'TEST_removed_terminal.py')
        with self.assertRaises(SystemExit):
            self.selected('full')


class WorkerTests(unittest.TestCase):
    def setUp(self):
        self.api = runpy.run_path(str(ROOT / 'scripts/python-tests.py'), run_name='test_workers')

    def test_affinity_limits_workers_when_available(self):
        with mock.patch.dict(os.environ, {}, clear=True), \
             mock.patch.object(os, 'sched_getaffinity', return_value={2, 4, 6}, create=True), \
             mock.patch.object(os, 'cpu_count', return_value=32):
            self.assertEqual(self.api['worker_count'](), 3)

    def test_cpu_count_is_not_capped_at_eight(self):
        with mock.patch.dict(os.environ, {}, clear=True), \
             mock.patch.object(os, 'sched_getaffinity', None, create=True), \
             mock.patch.object(os, 'cpu_count', return_value=15):
            self.assertEqual(self.api['worker_count'](), 15)
        with mock.patch.dict(os.environ, {}, clear=True), \
             mock.patch.object(os, 'sched_getaffinity', None, create=True), \
             mock.patch.object(os, 'cpu_count', return_value=None):
            self.assertEqual(self.api['worker_count'](), 2)

    def test_explicit_worker_count_wins_and_invalid_counts_fail(self):
        with mock.patch.dict(os.environ, TINKERY_TEST_JOBS='7'):
            self.assertEqual(self.api['worker_count'](), 7)
        for value in ('0', '-1', 'many'):
            with self.subTest(value=value), mock.patch.dict(os.environ, TINKERY_TEST_JOBS=value):
                with self.assertRaises(ValueError):
                    self.api['worker_count']()


class PreparationTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix='tinkery-TEST-python-preparation-')
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name).resolve()
        for name in ('scripts', 'tests', 'bin', 'HOME'):
            (self.root / name).mkdir()
        for name in ('scripts/python-tests.py', 'scripts/python-test-times.json',
                     'tests/session_fixture.py', 'tests/test_launch.py'):
            shutil.copyfile(ROOT / name, self.root / name)
        api = runpy.run_path(str(ROOT / 'scripts/python-tests.py'), run_name='test_preparation')
        for name in api['POST_MERGE']:
            (self.root / 'tests' / name).write_text('')
        self.fixture = self.root / 'target/debug/examples/seed_fixture'
        self.trace = self.root / 'TEST-builds.jsonl'
        fixture_source = (f'#!{sys.executable}\nimport json, sys\n'
                          'request = json.load(sys.stdin)\n'
                          'print(json.dumps({"operation": request["op"], "fixture": "prepared"}))\n')
        cargo = self.root / 'bin/cargo'
        cargo.write_text(f'''#!{sys.executable}
import json, os, sys
from pathlib import Path
root = Path.cwd()
args = sys.argv[1:]
with (root / 'TEST-builds.jsonl').open('a') as trace:
    trace.write(json.dumps(args) + '\\n')
fixture = root / 'target/debug/examples/seed_fixture'
if (root / 'TEST-first-ready').exists():
    fixture.unlink(missing_ok=True)
    (root / 'TEST-late-build').touch()
    raise SystemExit('TEST rebuild while another worker uses the fixture')
if os.environ.get('TEST_CARGO_FAIL'):
    raise SystemExit('TEST preparation failed')
if '--bin' in args:
    binary = root / 'target/debug/tinkery'
    binary.parent.mkdir(parents=True, exist_ok=True)
    binary.write_text('TEST main prepared')
if '--example' in args:
    fixture.parent.mkdir(parents=True, exist_ok=True)
    fixture.write_text({fixture_source!r})
    fixture.chmod(0o755)
''')
        cargo.chmod(0o755)
        self.env = subprocess.environment(home=self.root / 'HOME')
        self.env.update(PATH=str(self.root / 'bin'), TINKERY_TEST_JOBS='2', TINKERY_TEST_TIER='full')
        self.env.pop('TINKERY_TEST_SEED_FIXTURE', None)

    def run_python(self, *args):
        return subprocess.run([sys.executable, *args], cwd=self.root, env=self.env,
                              capture_output=True, text=True, timeout=20)

    def builds(self):
        return [json.loads(line) for line in self.trace.read_text().splitlines()] if self.trace.exists() else []

    def helper(self):
        return self.run_python('-c', "import sys; sys.path.insert(0, 'tests'); "
                               "import session_fixture; print(session_fixture.call('read'))")

    def test_parallel_workers_share_prepared_fixture_without_rebuilding(self):
        wait = '''
import time
from pathlib import Path
root = Path.cwd()
def wait_for(*names):
    deadline = time.monotonic() + 10
    while not any((root / name).exists() for name in names):
        assert time.monotonic() < deadline, names
        time.sleep(.01)
'''
        first = wait + '''
import session_fixture
assert (root / 'target/debug/tinkery').read_text() == 'TEST main prepared'
assert session_fixture.call('read')['fixture'] == 'prepared'
(root / 'TEST-first-ready').touch()
wait_for('TEST-second-done', 'TEST-late-build')
assert session_fixture.call('read')['fixture'] == 'prepared'
(root / 'TEST-first-done').touch()
'''
        second = wait + '''
wait_for('TEST-first-ready')
import session_fixture
assert session_fixture.call('read')['fixture'] == 'prepared'
(root / 'TEST-second-done').touch()
'''
        (self.root / 'tests/agent_operation_terminal.py').write_text(first)
        (self.root / 'tests/goal_confirmation_terminal.py').write_text(second)
        result = self.run_python('scripts/python-tests.py')
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertTrue((self.root / 'TEST-first-done').exists())
        self.assertEqual(len(self.builds()), 1)
        self.assertFalse((self.root / 'TEST-late-build').exists())

    def test_shard_command_runs_only_its_assigned_files(self):
        api = runpy.run_path(str(self.root / 'scripts/python-tests.py'), run_name='test_shards')
        for index in range(3):
            result = self.run_python('scripts/python-tests.py', '--shard-index', str(index),
                                     '--shard-count', '3')
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            names = [line.split()[1] for line in result.stdout.splitlines() if line.startswith('== ')]
            self.assertEqual(set(names), {'tests/' + path.name for path in api['selected']('full', index, 3)})

    def test_invalid_shard_starts_no_preparation_or_workers(self):
        result = self.run_python('scripts/python-tests.py', '--shard-index', '3', '--shard-count', '3')
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(self.builds(), [])

    def test_preparation_failure_starts_no_workers(self):
        (self.root / 'tests/agent_operation_terminal.py').write_text(
            "from pathlib import Path; Path('TEST-worker-started').touch()")
        self.env['TEST_CARGO_FAIL'] = '1'
        result = self.run_python('scripts/python-tests.py')
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('TEST preparation failed', result.stderr)
        self.assertFalse((self.root / 'TEST-worker-started').exists())

    def test_worker_failure_keeps_output_and_fails_the_runner(self):
        (self.root / 'tests/agent_operation_terminal.py').write_text(
            "print('TEST worker failure evidence'); raise SystemExit(3)")
        result = self.run_python('scripts/python-tests.py')
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('TEST worker failure evidence', result.stdout)
        self.assertIn('agent_operation_terminal.py FAILED (3)', result.stdout)

    def test_fast_failure_is_reported_before_blocked_worker_finishes(self):
        wait = """
import time
from pathlib import Path
root = Path.cwd()
def wait_for(name):
    deadline = time.monotonic() + 20
    while not (root / name).exists():
        assert time.monotonic() < deadline, name
        time.sleep(.01)
Path(__file__ + '.started').touch(exist_ok=False)
"""
        workers = {
            'agent_operation_terminal.py': wait + """
(root / 'TEST-blocked-ready').touch()
wait_for('TEST-release')
(root / 'TEST-blocked-done').touch()
""",
            'goal_confirmation_terminal.py': wait + """
import sys
wait_for('TEST-blocked-ready')
print('TEST fast failure first line')
print('TEST fast failure last line', file=sys.stderr)
raise SystemExit(3)
""",
            'test_second_failure.py': wait + """
print('TEST second failure first line')
print('TEST second failure last line')
raise SystemExit(4)
""",
            'test_later_success.py': wait + """
(root / 'TEST-later-done').touch()
""",
        }
        for name, source in workers.items():
            (self.root / 'tests' / name).write_text(source)
        selected = runpy.run_path(str(self.root / 'scripts/python-tests.py'),
                                  run_name='test_selection')['selected']('full')
        process = subprocess.Popen([sys.executable, 'scripts/python-tests.py'], cwd=self.root,
                                   env=self.env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
                                   text=True)
        lines = []
        reported = threading.Event()

        def read_output():
            for line in process.stdout:
                lines.append(line)
                if 'TEST fast failure last line' in line:
                    reported.set()

        reader = threading.Thread(target=read_output)
        reader.start()
        try:
            self.assertTrue(reported.wait(5), 'fast failure hidden behind blocked worker')
            self.assertTrue((self.root / 'TEST-blocked-ready').exists())
            self.assertFalse((self.root / 'TEST-blocked-done').exists())
        finally:
            (self.root / 'TEST-release').touch()
            try:
                process.wait(timeout=10)
            finally:
                if process.poll() is None:
                    process.kill()
                    process.wait(timeout=5)
                reader.join(timeout=5)
                process.stdout.close()
        self.assertFalse(reader.is_alive())
        output = ''.join(lines)
        self.assertIn(f'Python tests: tier full, {len(selected)} files, 2 workers', output)
        self.assertNotIn('== tests/session_fixture.py ', output)
        self.assertNotIn('== tests/test_launch.py ', output)
        self.assertNotEqual(process.returncode, 0, output)
        self.assertTrue((self.root / 'TEST-blocked-done').exists())
        self.assertTrue((self.root / 'TEST-later-done').exists())
        for name in workers:
            self.assertTrue((self.root / 'tests' / (name + '.started')).exists())
        for path in selected:
            self.assertEqual(output.count(f'== tests/{path.name} '), 1, output)
        for evidence in ('TEST fast failure first line', 'TEST fast failure last line',
                         'TEST second failure first line', 'TEST second failure last line'):
            self.assertIn(evidence, output)
        self.assertIn('2 Python test file(s) failed:', output)
        self.assertIn('goal_confirmation_terminal.py FAILED (3)', output)
        self.assertIn('test_second_failure.py FAILED (4)', output)

    def test_standalone_helper_prepares_its_own_fixture(self):
        # A marker from another worktree cannot skip this helper's preparation.
        self.env['TINKERY_TEST_SEED_FIXTURE'] = str(self.root / 'TEST-other/seed_fixture')
        result = self.helper()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("'fixture': 'prepared'", result.stdout)
        self.assertEqual(len(self.builds()), 1)

    def test_standalone_helper_reuses_an_existing_fixture(self):
        first = self.helper()
        self.assertEqual(first.returncode, 0, first.stderr)
        second = self.helper()
        self.assertEqual(second.returncode, 0, second.stderr)
        self.assertEqual(len(self.builds()), 1)

    def test_ordinary_journey_builds_require_a_missing_binary(self):
        for name in ('tend_connection_terminal.py', 'tend_correction_terminal.py',
                     'worker_relay_authorization_terminal.py', 'session_fixture.py'):
            tree = ast.parse((ROOT / 'tests' / name).read_text())
            parents = {child: node for node in ast.walk(tree) for child in ast.iter_child_nodes(node)}
            for node in ast.walk(tree):
                if not isinstance(node, ast.Call) or ast.unparse(node.func) != 'subprocess.run':
                    continue
                if not node.args or not isinstance(node.args[0], ast.List):
                    continue
                command = [item.value for item in node.args[0].elts if isinstance(item, ast.Constant)]
                if command[:3] != ['cargo', '+stable', 'build']:
                    continue
                ancestors = []
                parent = parents.get(node)
                while parent is not None:
                    ancestors.append(parent)
                    parent = parents.get(parent)
                if any(isinstance(parent, ast.FunctionDef) and parent.name == 'reversion_probe'
                       for parent in ancestors):
                    continue
                self.assertTrue(any(isinstance(parent, ast.If)
                                    and '.is_file()' in ast.unparse(parent.test)
                                    for parent in ancestors), (name, node.lineno))

    def test_standalone_preparation_failure_is_reported(self):
        self.env['TEST_CARGO_FAIL'] = '1'
        result = self.helper()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('TEST preparation failed', result.stderr)

    def test_missing_prepared_fixture_fails_without_rebuilding(self):
        self.env['TINKERY_TEST_SEED_FIXTURE'] = str(self.fixture)
        result = self.helper()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('Prepared TEST seed fixture is missing', result.stderr)
        self.assertEqual(self.builds(), [])


if __name__ == '__main__':
    unittest.main()
