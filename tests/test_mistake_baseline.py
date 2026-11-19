"""The #926 baseline counts what its docstring promises, from fixture logs, with no network."""
import json
from pathlib import Path
import sys
import tempfile
import test_launch as subprocess
import unittest

ROOT = Path(__file__).resolve().parents[1]


def bash(session, tool_id, start, seconds, *, background=False, reject=False, output='ok'):
    began = f'2026-11-10T{start}:00Z'
    hours, minutes = map(int, start.split(':'))
    total = hours * 3600 + minutes * 60 + seconds
    ended = f'2026-11-10T{total // 3600:02d}:{total % 3600 // 60:02d}:{total % 60:02d}Z'
    call = {'type': 'assistant', 'timestamp': began, 'message': {'content': [
        {'type': 'tool_use', 'id': tool_id, 'name': 'Bash',
         'input': {'command': 'TEST', 'run_in_background': background}}]}}
    result = {'type': 'user', 'timestamp': ended, 'message': {'content': [
        {'type': 'tool_result', 'tool_use_id': tool_id, 'content': output}]}}
    if reject:
        result['permissionDecision'] = {'decision': 'reject'}
    with session.open('a') as file:
        file.write(json.dumps(call) + '\n' + json.dumps(result) + '\n')


class BaselineTests(unittest.TestCase):
    def test_counts_foreground_waits_once_and_finds_the_pyc_failure(self):
        with tempfile.TemporaryDirectory(prefix='tinkery-TEST-baseline-') as temp:
            home = Path(temp) / 'TEST-home'
            project = home / '.claude/projects/TEST-project'
            (project / 'TEST-main/subagents').mkdir(parents=True)
            main = project / 'TEST-session-aaaa1111.jsonl'
            bash(main, 'a', '10:00', 120)
            bash(main, 'b', '10:01', 120)  # Overlaps a by one minute.
            bash(main, 'c', '11:00', 30)  # Shorter than 45 s.
            bash(main, 'd', '12:00', 600, background=True)
            bash(main, 'e', '13:00', 6000, reject=True)
            bash(main, 'f', '14:00', 5,
                 output='FAIL: test_untracked_python_cache_does_not_fail_passing_suite')
            bash(project / 'TEST-main/subagents/agent-bbbb2222.jsonl', 'g', '10:00', 600)
            env = subprocess.environment(home=home)
            env['PATH'] = str(Path(temp) / 'TEST-no-gh')
            out = subprocess.run(
                [sys.executable, '-I', str(ROOT / 'scripts/mistake-baseline.py'),
                 '2026-11-09', '2026-11-16'],
                env=env, capture_output=True, text=True, check=True).stdout
        rows = {line.split()[0]: line.split()[1:] for line in out.splitlines()[2:]
                if line.startswith(('2026-', 'total'))}
        self.assertEqual(rows['2026-W47'], ['0', '0', '0', '0'])
        self.assertEqual(rows['2026-W46'], ['0', '3', '1', '0'])
        self.assertIn('pyc: aaaa1111', out)
        self.assertIn("unread: GitHub: [Errno 2] No such file or directory: 'gh'", out)


if __name__ == '__main__':
    unittest.main()
