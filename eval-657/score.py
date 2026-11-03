#!/usr/bin/env python3
"""#657 scorer: run each proposed check command in a fresh copy of its TEST repo.

    python3 score.py REPOS_OUT RESULTS.json [more results...]

A results file is the driver's result.json ({repo, label, checks}) or a list of them.
A command passes when it gets past setup: no import error, missing module or missing command.
A test that runs and fails (the bug is still there) still passes. When a command names a test
file that does not exist yet (a provisional regression), the repo's probe test is written there
first, so the command still has to reach the package. Each run gets its own venv and HOME.
"""
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from make_repos import REPOS  # noqa: E402

SETUP = re.compile(r'ModuleNotFoundError|ImportError|No module named|command not found|ERR_MODULE_NOT_FOUND|'
                   r'Cannot find (?:module|package)|No rule to make target|could not find `Cargo.toml`|'
                   r'not found: |No such file or directory|Ran 0 tests|no tests ran|NO TESTS RAN')
TESTFILE = re.compile(r"[\w./-]*(?:test_[\w-]+\.py|[\w-]+\.test\.m?[jt]s)")


def score(repos, cmd, cwd, repo):
    with tempfile.TemporaryDirectory(prefix='tinkery-TEST-657-') as tmp:
        tmp = Path(tmp)
        clone = tmp / 'repo'
        subprocess.run(['git', 'clone', '-q', str(repos / repo), str(clone)], check=True)
        probe = REPOS[repo]['probe']
        work = (clone / cwd).resolve()
        wrote = []
        for name in TESTFILE.findall(cmd):
            target = work / name
            if probe and not target.exists() and target.is_relative_to(clone):
                target.parent.mkdir(parents=True, exist_ok=True)
                target.write_text(probe)
                wrote.append(name)
        subprocess.run([sys.executable, '-m', 'venv', str(tmp / 'venv')], check=True)
        home = tmp / 'home'
        home.mkdir()
        env = {**os.environ, 'HOME': str(home), 'VIRTUAL_ENV': str(tmp / 'venv'),
               'PATH': f"{tmp / 'venv/bin'}:{os.environ['PATH']}", 'PYTHONPATH': ''}
        env.pop('PYTHONPATH')
        try:
            done = subprocess.run(['bash', '-c', cmd], cwd=work, env=env, capture_output=True, text=True, timeout=300)
            out, code = done.stdout + done.stderr, done.returncode
        except subprocess.TimeoutExpired:
            out, code = 'TIMEOUT', None
        setup = SETUP.search(out)
        return {'cmd': cmd, 'cwd': cwd, 'exit': code, 'probe': wrote,
                'pass': code is not None and not setup, 'why': setup.group(0) if setup else None,
                'tail': out[-600:]}


def main():
    repos = Path(sys.argv[1]).resolve()
    runs = []
    for path in sys.argv[2:]:
        data = json.loads(Path(path).read_text())
        runs.extend(data if isinstance(data, list) else [data])
    table = []
    for run in runs:
        rows = [score(repos, c['cmd'], c.get('cwd', '.'), run['repo'])
                for c in (run.get('checks') or []) if c.get('cmd')]
        table.append({'label': run['label'], 'repo': run['repo'], 'ending': run.get('ending'), 'rows': rows,
                      'pass': bool(rows) and all(r['pass'] for r in rows)})
    print(json.dumps(table, indent=2))
    drafted = [t for t in table if t['rows']]
    print(f"# {sum(t['pass'] for t in drafted)}/{len(drafted)} drafts with every command past setup "
          f"({len(table) - len(drafted)} runs without a command)", file=sys.stderr)


if __name__ == '__main__':
    main()
