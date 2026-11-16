#!/usr/bin/env python3
"""Run every Python test file in parallel and fail if any file fails.

`TINKERY_TEST_TIER=pr` skips only the slow journeys listed in POST_MERGE; any new
file runs in both tiers. The default tier is `full`, which runs every file.
Every launch passes the shared guard, so the runner refuses a non-TEST HOME.
Each file is its own process; a failed file's output is printed whole.
"""
import argparse
from concurrent.futures import ThreadPoolExecutor, as_completed
import json
import os
from pathlib import Path
import subprocess
import sys
import time

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / 'tests'))
import test_launch
# Helper modules, not tests.
HELPERS = {'vt_screen.py', 'session_fixture.py', 'test_launch.py', 'terminal_process.py'}
# Slow terminal journeys that pull requests skip; pushes to main and nightly runs keep them.
POST_MERGE = {
    'hidden_review_terminal.py',
    'selection_terminal.py',
    'shape_answer_terminal.py',
    'taste_terminal.py',
    'think_history_terminal.py',
}
DURATIONS = json.loads((ROOT / 'scripts/python-test-times.json').read_text())


def selected(tier, shard_index=0, shard_count=1):
    if shard_count < 1 or not 0 <= shard_index < shard_count:
        raise SystemExit('Shard index must be in [0, shard count), with a positive shard count.')
    if tier not in ('pr', 'full'):
        raise SystemExit(f'Unknown TINKERY_TEST_TIER {tier!r}; use pr or full.')
    files = sorted(p for p in (ROOT / 'tests').glob('*.py') if p.name not in HELPERS)
    missing = POST_MERGE - {p.name for p in files}
    if missing:
        raise SystemExit(f'POST_MERGE names missing tests: {sorted(missing)}')
    files = sorted((p for p in files if tier == 'full' or p.name not in POST_MERGE),
                   key=lambda p: (-DURATIONS.get(p.name, 1), p.name))
    shards, seconds = [[] for _ in range(shard_count)], [0] * shard_count
    for path in files:
        index = min(range(shard_count), key=seconds.__getitem__)
        shards[index].append(path)
        seconds[index] += DURATIONS.get(path.name, 1)
    return shards[shard_index]


def worker_count():
    override = os.environ.get('TINKERY_TEST_JOBS')
    affinity = getattr(os, 'sched_getaffinity', None)
    workers = int(override) if override else len(affinity(0)) if affinity else os.cpu_count() or 2
    if workers < 1:
        raise ValueError('TINKERY_TEST_JOBS must be a positive integer.')
    return workers


def run(path, env):
    start = time.monotonic()
    result = test_launch.run([sys.executable, str(path)], cwd=ROOT, env=env,
                             stdin=subprocess.DEVNULL, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    return path, result.returncode, time.monotonic() - start, result.stdout.decode(errors='replace')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--shard-index', type=int, default=0)
    parser.add_argument('--shard-count', type=int, default=1)
    args = parser.parse_args()
    tier = os.environ.get('TINKERY_TEST_TIER', 'full')
    files = selected(tier, args.shard_index, args.shard_count)
    # Prepare both executables before workers can use them; even fresh Cargo builds
    # replace the example executable on macOS.
    env = dict(os.environ)
    test_launch.run(['cargo', '+stable', 'build', '--locked', '--quiet',
                     '--bin', 'tinkery', '--example', 'seed_fixture'], cwd=ROOT,
                    env=env, check=True)
    env['TINKERY_TEST_SEED_FIXTURE'] = str(ROOT / 'target/debug/examples/seed_fixture')
    workers = worker_count()
    print(f'Python tests: tier {tier}, {len(files)} files, {workers} workers', flush=True)
    failed = []
    with ThreadPoolExecutor(workers) as pool:
        futures = [pool.submit(run, path, env) for path in files]
        for future in as_completed(futures):
            path, code, seconds, output = future.result()
            name = path.relative_to(ROOT)
            print(f'== {name} {"ok" if code == 0 else f"FAILED ({code})"} {seconds:.1f}s', flush=True)
            if code:
                failed.append(name)
                print(output, flush=True)
    if failed:
        raise SystemExit(f'{len(failed)} Python test file(s) failed: ' + ', '.join(map(str, failed)))


if __name__ == '__main__':
    main()
