#!/usr/bin/env python3
"""Execute the single authoritative AGENTS full-check block in an isolated TEST home.

Keeps the existing installed compiler/cache locations; never installs tools or uses
operator application roots. Fixture contents must match the CI-pinned revision.
"""
import os
from pathlib import Path
import re
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]


def block():
    source = (ROOT / 'AGENTS.md').read_text()
    markers = ['<!-- full-check:start -->', '<!-- full-check:end -->']
    if any(source.count(marker) != 1 for marker in markers):
        raise ValueError('Expected exactly one pair of full-check markers')
    section = source.split(markers[0], 1)[1].split(markers[1], 1)[0]
    blocks = re.findall(r'```bash\n(.*?)\n```', section, re.S)
    if len(blocks) != 1 or not blocks[0].strip():
        raise ValueError('Expected exactly one nonempty documented verification block')
    return blocks[0] + '\n'


def main():
    ci = (ROOT / '.github/workflows/ci.yml').read_text()
    refs = re.findall(r'^\s+ref: ([0-9a-f]{40})$', ci, re.M)
    if len(refs) != 1:
        raise ValueError('Expected one pinned Seed Me CI revision')
    fixture = ROOT / '.ci-seed-me'
    revision = subprocess.check_output(['git', '-C', str(fixture), 'rev-parse', 'HEAD'], text=True).strip()
    if revision != refs[0]:
        raise ValueError('Disposable Seed Me fixture does not match CI')
    subprocess.run(['git', '-C', str(fixture), 'diff', '--exit-code', 'HEAD'], check=True)
    command = block()
    with tempfile.TemporaryDirectory(prefix='tinkery-TEST-full-check-') as directory:
        home = Path(directory) / 'HOME'
        home.mkdir()
        env = dict(os.environ)
        installed_home = Path(env['HOME'])
        env.setdefault('CARGO_HOME', str(installed_home / '.cargo'))
        env.setdefault('RUSTUP_HOME', str(installed_home / '.rustup'))
        env.update(HOME=str(home), PYTHONDONTWRITEBYTECODE='1', TINKERY_NODE='/tinkery-TEST-no-node')
        script = Path(directory) / 'full-check.sh'
        script.write_text(command)
        print(f'Exact AGENTS full-check; fixture {revision}; temporary TEST HOME', flush=True)
        subprocess.run(['bash', '-euo', 'pipefail', str(script)], cwd=ROOT, env=env, check=True)


if __name__ == '__main__':
    main()
