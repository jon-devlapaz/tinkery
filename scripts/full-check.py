#!/usr/bin/env python3
"""Execute the single authoritative AGENTS full-check block in an isolated TEST home.

Keeps the existing installed compiler/cache locations; never installs tools or uses
operator application roots. No external skill checkout is needed.
"""
import argparse
import importlib.util
import os
from pathlib import Path
import re
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / 'tests'))
import test_launch

_spec = importlib.util.spec_from_file_location('bundled_sdlc', ROOT / '_system/scripts/sdlc.py')
sdlc = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(sdlc)


GROUPS = {
    'rust': ('cargo +stable build ', 'cargo +stable run ', 'python3 scripts/rust-tests.py'),
    'style': ('cargo +stable fmt ', 'cargo +stable clippy '),
    'node': ('node --test ',),
    'python': ('python3 scripts/python-tests.py',),
}


def block(group=None, shard_index=0, shard_count=1):
    source = (ROOT / 'AGENTS.md').read_text()
    markers = ['<!-- full-check:start -->', '<!-- full-check:end -->']
    if any(source.count(marker) != 1 for marker in markers):
        raise ValueError('Expected exactly one pair of full-check markers')
    section = source.split(markers[0], 1)[1].split(markers[1], 1)[0]
    blocks = re.findall(r'```bash\n(.*?)\n```', section, re.S)
    if len(blocks) != 1 or not blocks[0].strip():
        raise ValueError('Expected exactly one nonempty documented verification block')
    command = blocks[0] + '\n'
    if group is None:
        if (shard_index, shard_count) != (0, 1):
            raise ValueError('Full verification cannot select a Python shard.')
        return command
    if group not in GROUPS:
        raise ValueError(f'Unknown verification group: {group}')
    if shard_count < 1 or not 0 <= shard_index < shard_count:
        raise ValueError('Invalid Python shard.')
    if group != 'python' and (shard_index, shard_count) != (0, 1):
        raise ValueError('Only the Python group can select a shard.')
    selected = []
    for line in command.splitlines():
        if not line:
            continue
        common = line.startswith('export ') or line in ('rustc +stable --version',
                                                        'node --version', 'python3 --version')
        owners = [name for name, prefixes in GROUPS.items() if line.startswith(prefixes)]
        if not common and len(owners) != 1:
            raise ValueError(f'Unclassified verification command: {line}')
        if common or group in owners:
            if group == 'python' and owners and (shard_index, shard_count) != (0, 1):
                line += f' --shard-index {shard_index} --shard-count {shard_count}'
            selected.append(line)
    return '\n'.join(selected) + '\n'


def main(argv=()):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--group', choices=GROUPS)
    parser.add_argument('--shard-index', type=int, default=0)
    parser.add_argument('--shard-count', type=int, default=1)
    args = parser.parse_args(argv)
    command = block(args.group, args.shard_index, args.shard_count)
    with tempfile.TemporaryDirectory(prefix='tinkery-TEST-full-check-') as directory:
        home = Path(directory) / 'HOME'
        home.mkdir()
        env = dict(os.environ)
        installed_home = Path(env['HOME'])
        env.setdefault('CARGO_HOME', str(installed_home / '.cargo'))
        env.setdefault('RUSTUP_HOME', str(installed_home / '.rustup'))
        # A tink-sdlc build step marks its worker env; label tests run outside that session.
        env.pop('TINK_SDLC_STEP_SESSION', None)
        # The bundled verify refuses runner options from the caller (#197); cloud shells set some.
        for name in sdlc.RUNNER_ENV:
            env.pop(name, None)
        env.update(HOME=str(home), PYTHONDONTWRITEBYTECODE='1', TINKERY_NODE='/tinkery-TEST-no-node')
        script = Path(directory) / 'full-check.sh'
        script.write_text(command)
        label = 'Exact AGENTS full-check' if args.group is None else f'AGENTS verification group: {args.group}'
        print(f'{label}; temporary TEST HOME', flush=True)
        test_launch.run(['bash', '-euo', 'pipefail', str(script)], cwd=ROOT, env=env, check=True)


if __name__ == '__main__':
    main(sys.argv[1:])
