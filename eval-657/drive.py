#!/usr/bin/env python3
"""#657 eval: one live Shape draft for one TEST repo, through the real Tinkery binary in a PTY.

Run from a Tinkery worktree root (needs tests/vt_screen.py and tests/session_fixture.py):

    python3 eval-657/drive.py --binary <tinkery> --repos <make_repos OUT> --repo tox-src \
        --label base-1 --out <eval root> --model openai-codex/gpt-6.1-sol --thinking medium --budget 3

The session holds the repo's confirmed goal and an answered pre-mortem, so F2 goes straight to the
model. After each model step: a seed draft ends the run; a question gets "Use your suggestion." and
F2; a refused reply gets F2 (Tinkery's retry). At most --budget calls; a wrapper around `pi` counts
them and keeps each prompt and reply. Writes only under the TEST root; Pi reads its auth from
PI_CODING_AGENT_DIR.
"""
import argparse
import fcntl
import json
import os
import select
import shutil
import struct
import subprocess
import sys
import termios
import time
from pathlib import Path

sys.path.insert(0, str(Path.cwd() / 'tests'))
from vt_screen import Screen  # noqa: E402
import session_fixture as helper  # noqa: E402

F2 = b'\x1bOQ'
REAL = [Path.home() / '.local/share/tinkery', Path.home() / '.local/share/seed-me']
WRAPPER = '''#!/bin/sh
dir="{calls}"
n=$(ls "$dir" | grep -c '^call-.*\\.start$')
n=$((n + 1))
touch "$dir/call-$n.start"
printf '%s\\n' "$@" > "$dir/call-$n.argv"
{pi} "$@" > "$dir/call-$n.out"
code=$?
cat "$dir/call-$n.out"
echo $code > "$dir/call-$n.end"
exit $code
'''


def node(identifier, **fields):
    return {'id': identifier, 'kind': 'decision', 'prerequisites': [], 'evidence': [],
            'owner': 'TEST operator', 'gate': 'TEST eval', **fields}


def main():
    parser = argparse.ArgumentParser()
    for flag in ('binary', 'repos', 'repo', 'label', 'out', 'model', 'thinking'):
        parser.add_argument('--' + flag, required=True)
    parser.add_argument('--budget', type=int, default=3)
    args = parser.parse_args()
    root = Path(args.out).resolve() / args.label
    if root.exists():
        sys.exit(f'{root} exists; pick a new label')
    if any(root.is_relative_to(path) for path in REAL):
        sys.exit('refusing a TEST root inside the operator data folders')
    home, sessions, calls = root / 'home', root / 'sessions', root / 'calls'
    for path in (home, sessions, calls):
        path.mkdir(parents=True)
    repo = root / 'repo'
    subprocess.run(['git', 'clone', '-q', str(Path(args.repos) / args.repo), str(repo)], check=True)
    goal = (Path(args.repos) / f'{args.repo}.goal.txt').read_text()

    session = helper.create(sessions, 'agent')
    old = helper.load(session)
    state = helper.editable(old)
    authority = dict(authority='user', authority_source='TEST eval fixture, not operator approval')
    state.update(goal=goal, origin='goal', current_question=None, nodes=[
        node('goal', status='settled', answer=goal, **authority),
        node('probe-pre-mortem', status='settled', label='Probe: pre-mortem',
             question='Imagine this shipped and turned out wrong. What is the most likely reason?',
             answer='The fix passes its own test but breaks the existing tests.', **authority)])
    helper.publish(session, state, old['version'], 'TEST #657 eval fixture')

    pi = shutil.which('pi')
    assert pi, 'pi is not on PATH'
    wrapper = root / 'pi-wrapper'
    wrapper.write_text(WRAPPER.format(calls=calls, pi=pi))
    wrapper.chmod(0o755)

    master, slave = os.openpty()
    fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack('HHHH', 30, 100, 0, 0))
    screen = Screen(100, 30)
    argv = [str(Path(args.binary).resolve()), '--operator', 'agent', '--home', '--shape-pi', '--model', args.model,
            '--thinking', args.thinking, '--shape-budget', str(args.budget),
            '--seed-session-root', str(sessions), '--record-root', str(root / 'think'),
            '--link-root', str(root / 'tend'), '--sdlc-root', str(repo), '--pi-command', str(wrapper)]
    env = {**os.environ, 'HOME': str(home), 'TERM': 'xterm-256color'}
    assert env.get('PI_CODING_AGENT_DIR'), 'set PI_CODING_AGENT_DIR to the Pi agent dir with auth'
    process = subprocess.Popen(argv, stdin=slave, stdout=slave, stderr=slave, cwd=repo, env=env)

    def pump(seconds):
        deadline = time.monotonic() + seconds
        while time.monotonic() < deadline and process.poll() is None:
            if select.select([master], [], [], .05)[0]:
                screen.feed(os.read(master, 65536))

    def wait(text, timeout=20):
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline and process.poll() is None:
            pump(.1)
            if ' '.join(text.split()) in ' '.join(screen.text().split()) and not screen.synchronized:
                return
        raise SystemExit(f'{text!r} not shown:\n{screen.text()}')

    def model_step():
        """Press F2 and wait for the wrapper to finish one call, then for the screen to settle."""
        before = len(list(calls.glob('*.end')))
        os.write(master, F2)
        deadline = time.monotonic() + 900
        while len(list(calls.glob('*.end'))) == before:
            if time.monotonic() > deadline or process.poll() is not None:
                raise SystemExit(f'no call finished:\n{screen.text()}')
            pump(.5)
        pump(3)

    endings = []
    try:
        wait('3  shape')
        for key, text in [(b'3', 'enter open'), (b'\r', 'F2')]:
            os.write(master, key)
            wait(text)
        while len(list(calls.glob('*.end'))) < args.budget:
            model_step()
            flat = ' '.join(screen.text().split())
            if (session / 'seed-checks.json').exists():
                endings.append('draft')
                break
            if helper.load(session).get('current_question'):
                endings.append('question')
                os.write(master, b'\x1b[200~Use your suggestion.\x1b[201~')
                pump(1)
            else:
                endings.append('refused' if 'refused' in flat else 'other')
    finally:
        process.kill()
        process.wait()
    checks = session / 'seed-checks.json'
    report = {'label': args.label, 'repo': args.repo, 'model': args.model, 'thinking': args.thinking,
              'binary': args.binary, 'endings': endings, 'ending': endings[-1] if endings else None,
              'calls': len(list(calls.glob('*.start'))), 'screen': screen.text(),
              'checks': json.loads(checks.read_text()) if checks.exists() else None}
    (root / 'result.json').write_text(json.dumps(report, indent=2, ensure_ascii=False))
    print(json.dumps({k: report[k] for k in ('label', 'repo', 'endings', 'calls')}
                     | {'cmds': [c.get('cmd') for c in report['checks'] or []]}, ensure_ascii=False))


if __name__ == '__main__':
    main()
