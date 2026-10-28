#!/usr/bin/env python3
"""Run the workspace once in a fresh TEST home and refuse default application storage."""
from pathlib import Path
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / 'tests'))
import test_launch


def main():
    with tempfile.TemporaryDirectory(prefix='tinkery-TEST-rust-workspace-') as directory:
        home = Path(directory) / 'HOME'
        home.mkdir()
        result = test_launch.run(['cargo', '+stable', 'test', '--workspace', '--locked'],
                                 cwd=ROOT, env=test_launch.environment(home=home))
        if result.returncode:
            raise SystemExit(result.returncode)
        for application in ('tinkery', 'seed-me'):
            root = home / '.local/share' / application
            if root.exists():
                raise SystemExit(f'Rust tests wrote to default application data: {root}')


if __name__ == '__main__':
    main()
