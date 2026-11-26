#!/usr/bin/env python3
"""Install or update Tinkery for the current user: the one supported install path.

Promises: builds this checkout (`cargo +stable build --locked --release`), or takes `--binary`,
and refuses to install a build whose `--snapshot` fails. Puts it at
~/.local/share/tinkery/bin/tinkery, keeps the build it replaces as tinkery.previous, and
writes the ~/.local/bin/tinkery launcher with the binary's absolute path, so the launcher
works under any HOME. Prints the commit and SHA-256 it installed. Safe to run again.
Runs no model, downloads only what cargo needs, and writes nothing else.
"""
import argparse
import hashlib
import os
from pathlib import Path
import shlex
import shutil
import subprocess
import sys
import tempfile

REPO = Path(__file__).resolve().parents[1]


class Refusal(Exception):
    pass


def snapshot(program):
    """Run `program --snapshot` in a throwaway HOME, as scripts/first-task-check.py does."""
    with tempfile.TemporaryDirectory(prefix='tinkery-install-check-') as home:
        try:
            result = subprocess.run([str(program), '--snapshot', '--no-color'], env={**os.environ, 'HOME': home},
                                    capture_output=True, text=True, timeout=30)
        except (OSError, subprocess.TimeoutExpired) as error:
            raise Refusal(f'{program} did not start: {error}') from error
    if result.returncode:
        raise Refusal(f'{program} --snapshot failed: {(result.stderr or result.stdout).strip()[:300]}')


def write(target, data, previous=None):
    """Write `data` to `target` atomically; copy a differing old file to `previous` first."""
    target.parent.mkdir(parents=True, exist_ok=True)
    if target.is_file() and target.read_bytes() == data:
        return
    handle, temporary = tempfile.mkstemp(prefix=f'.{target.name}-', dir=target.parent)
    try:
        with os.fdopen(handle, 'wb') as out:
            out.write(data)
        os.chmod(temporary, 0o755)
        if previous is not None and target.is_file():
            shutil.copy2(target, previous)
        os.replace(temporary, target)
    except BaseException:
        Path(temporary).unlink(missing_ok=True)
        raise


def install(binary=None):
    if binary is None:
        if subprocess.run(['cargo', '+stable', 'build', '--locked', '--release'], cwd=REPO).returncode:
            raise Refusal('the release build failed; nothing was installed.')
        binary = REPO / 'target/release/tinkery'
    binary = Path(binary).resolve()
    if not binary.is_file():
        raise Refusal(f'{binary} is not a file; nothing was installed.')
    snapshot(binary)
    home = Path.home()
    if not home.is_absolute():
        raise Refusal(f'HOME must be absolute, not {home}.')
    installed = home / '.local/share/tinkery/bin/tinkery'
    data = binary.read_bytes()
    write(installed, data, installed.with_name('tinkery.previous'))
    write(home / '.local/bin/tinkery', (
        '#!/bin/sh\n# Written by scripts/install.py in the tinkery repository. Run it again; do not edit.\n'
        f'exec {shlex.quote(str(installed))} --home "$@"\n').encode())
    revision = subprocess.run(['git', '-C', str(REPO), 'describe', '--always', '--dirty', '--abbrev=12'],
                              capture_output=True, text=True).stdout.strip() or 'unknown'
    print(f'Installed Tinkery {revision} at {installed}\nsha256 {hashlib.sha256(data).hexdigest()}')


def main():
    parser = argparse.ArgumentParser(description=__doc__.split('\n\n')[0])
    parser.add_argument('--binary', type=Path, help='install this built binary instead of building')
    args = parser.parse_args()
    try:
        install(args.binary)
    except Refusal as refusal:
        print(f'install refused: {refusal}', file=sys.stderr)
        return 1
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
