"""The #926 baseline counts what its docstring promises, from fixture logs and a stub gh."""
import json
from pathlib import Path
import sys
import tempfile
import test_launch as subprocess
import textwrap
import unittest

ROOT = Path(__file__).resolve().parents[1]
# Built from parts so this file is never itself a pyc hit.
PYC_FAIL = 'FAIL' + ': test_untracked_python_cache_does_not_fail_passing_suite (tests.X)'


def stamp(start, seconds=0):
    hours, minutes = map(int, start.split(':'))
    total = hours * 3600 + minutes * 60 + seconds
    return f'2026-11-10T{total // 3600:02d}:{total % 3600 // 60:02d}:{total % 60:02d}Z'


def append(path, *records):
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open('a') as file:
        file.writelines(json.dumps(r) + '\n' for r in records)


def claude(path, tool_id, start, seconds, *, name='Bash', background=False, reject=False,
           output='ok'):
    result = {'type': 'user', 'timestamp': stamp(start, seconds), 'message': {'content': [
        {'type': 'tool_result', 'tool_use_id': tool_id, 'content': output}]}}
    if reject:
        result['permissionDecision'] = {'decision': 'reject'}
    append(path, {'type': 'assistant', 'timestamp': stamp(start), 'message': {'content': [
        {'type': 'tool_use', 'id': tool_id, 'name': name,
         'input': {'command': 'TEST', 'run_in_background': background}}]}}, result)


def codex(path, call_id, start, name, wall, output=''):
    append(path,
           {'timestamp': stamp(start), 'type': 'response_item',
            'payload': {'type': 'function_call', 'name': name, 'call_id': call_id}},
           {'timestamp': stamp(start, 1), 'type': 'response_item',
            'payload': {'type': 'function_call_output', 'call_id': call_id,
                        'output': [{'type': 'text', 'text': f'Wall time: {wall} seconds\n{output}'}]}})


GH = textwrap.dedent('''\
    import json, sys
    path = sys.argv[-1]
    def pr(number, title, body, created, merged=None):
        return {'number': number, 'title': title, 'body': body,
                'created_at': created, 'merged_at': merged}
    def comment(issue, body, created):
        return {'issue_url': f'https://api.github.com/repos/R/issues/{issue}', 'body': body,
                'created_at': created}
    data = {
        'pulls': [pr(1, 'Keep Watch responsive when inputs are named pipes', '',
                     '2026-11-01T00:00:00Z', '2026-11-11T00:00:00Z'),
                  pr(2, 'Fix a test', 'I re-ran the failed job once.', '2026-11-10T00:00:00Z'),
                  pr(3, 'Other', '', '2026-11-01T00:00:00Z')],
        'comments': [comment(2, 'Re-ran the failed job.', '2026-11-12T00:00:00Z'),
                     comment(3, 'Re-ran the failed job.', '2026-11-17T00:00:00Z'),
                     comment(9, 'Re-ran the failed job.', '2026-11-12T00:00:00Z')],
    }
    repo_one = '/tinkery/' in path
    print(json.dumps([data['pulls' if '/pulls?' in path else 'comments'] if repo_one else []]))
''')


class BaselineTests(unittest.TestCase):
    def test_counts_only_what_the_docstring_promises(self):
        with tempfile.TemporaryDirectory(prefix='tinkery-TEST-baseline-') as temp:
            home, tools = Path(temp) / 'TEST-home', Path(temp) / 'TEST-tools'
            tools.mkdir()
            (tools / 'gh').write_text(f'#!{sys.executable}\n' + GH)
            (tools / 'gh').chmod(0o755)
            project = home / '.claude/projects/TEST-project'
            main = project / 'TEST-session-aaaa1111.jsonl'
            claude(main, 'a', '10:00', 120)
            claude(main, 'b', '10:01', 120)  # Overlaps a by one minute.
            claude(main, 'c', '11:00', 30)  # Shorter than 45 s.
            claude(main, 'd', '12:00', 600, background=True)
            claude(main, 'e', '13:00', 6000, reject=True)
            claude(main, 'f', '14:00', 600, name='Agent')
            claude(main, 'g', '15:00', 5, output=PYC_FAIL)
            claude(project / 'TEST-read-cccc3333.jsonl', 'h', '10:00', 5, name='Read',
                   output=PYC_FAIL)
            claude(project / 'TEST-main/subagents/agent-bbbb2222.jsonl', 'i', '10:00', 600)
            session = home / '.codex/sessions/2026/11/10/rollout-dddd4444.jsonl'
            codex(session, 'j', '10:00', 'exec', 120, PYC_FAIL)
            codex(session, 'k', '11:00', 'mcp__browser__wait', 600)
            env = subprocess.environment(home=home)
            env['PATH'] = str(tools)
            out = subprocess.run(
                [sys.executable, '-I', str(ROOT / 'scripts/mistake-baseline.py'),
                 '2026-11-09', '2026-11-16'],
                env=env, capture_output=True, text=True, check=True).stdout
        rows = {line.split()[0]: line.split()[1:] for line in out.splitlines()[2:]
                if line.startswith(('2026-', 'total'))}
        self.assertEqual(rows['2026-W46'], ['1', '5', '2', '1'])
        self.assertEqual(rows['2026-W47'], ['0', '0', '0', '0'])
        self.assertIn('reruns: tinkery#2\n', out)
        self.assertIn('fifo: tinkery#1\n', out)
        self.assertIn('pyc: aaaa1111, dddd4444\n', out)
        self.assertNotIn('unread', out)


if __name__ == '__main__':
    unittest.main()
