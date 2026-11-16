"""Approve and Watch use the launch project unless an explicit root takes precedence."""
import json
import tempfile
from pathlib import Path

import test_launch as subprocess
from think_history_terminal import BINARY, MENU, Terminal

FAKE = '''import json, sys
from pathlib import Path
with Path('TEST-calls.jsonl').open('a') as log:
    log.write(json.dumps(sys.argv[1:]) + '\\n')
print(json.dumps(dict(protocol='tink-sdlc', api_version=1, workspace=str(Path.cwd()), runs=[])))
'''


def checkout(root):
    script = root / '_system/scripts/sdlc.py'
    script.parent.mkdir(parents=True)
    script.write_text(FAKE)


def calls(root):
    log = root / 'TEST-calls.jsonl'
    return [json.loads(line) for line in log.read_text().splitlines()] if log.exists() else []


def refused_roots():
    with tempfile.TemporaryDirectory(prefix='tinkery-TEST-sdlc-root-refusal-') as directory:
        root = Path(directory).resolve()
        checkout(root)
        for arguments in [
            ['--sdlc-root'],
            ['--sdlc-root', ''],
            ['--sdlc-root', str(root), '--sdlc-root', ''],
            ['--sdlc-root', '', '--sdlc-root', str(root)],
        ]:
            result = subprocess.run([str(BINARY), '--operator', 'agent', '--home', '--snapshot',
                                     *arguments], cwd=root, env=subprocess.environment(),
                                    capture_output=True, timeout=5)
            assert result.returncode != 0, arguments
            assert b'--sdlc-root needs a tink-sdlc checkout path' in result.stderr, result.stderr
            assert result.stdout == b'', 'invalid root reached screen startup'
            assert calls(root) == [], 'invalid root executed the current checkout'
        print('Missing and empty roots refused, including repeated options; no script ran.')


def launch_project(width, height, kind):
    with tempfile.TemporaryDirectory(prefix='tinkery-TEST-launch-project-') as directory:
        root = Path(directory).resolve()
        project = root / 'TEST-project'
        subfolder = project / 'TEST-subfolder'
        override = root / 'TEST-override'
        (project / '.git').mkdir(parents=True)
        subfolder.mkdir()
        checkout(override)
        if kind != 'no-install':
            checkout(project)
        launch = {'root': project, 'outside': root}.get(kind, subfolder)
        arg = ['--sdlc-root', override] if kind == 'override' else []
        expected = override if arg else project if kind in ('root', 'subfolder') else None
        sessions = root / 'TEST-sessions'
        sessions.mkdir()
        terminal = Terminal(launch, width, height, [
            '--operator', 'agent', '--home', '--shape-budget', '0',
            '--seed-session-root', sessions, *arg])
        try:
            terminal.wait(lambda screen: MENU in screen)
            assert calls(project) == calls(override) == []
            terminal.send(b'4', 'No verified Tinks are waiting.' if expected else
                          'No verified Tinks could be listed.')
            if expected:
                assert calls(expected) == [['status', '--json']]
            else:
                assert 'Open Tinkery from a project with tink-sdlc installed.' in terminal.screen.text()
            terminal.send(b'\x1b[21~', MENU)
            terminal.send(b'5', 'No runs in this checkout yet.' if expected else
                          'Open Tinkery from a project with tink-sdlc installed.')
            for path in (project, override):
                assert calls(path) == ([['status', '--json']] * 2 if path == expected else [])
            terminal.quit()
            print(f'{kind} launch at {width}x{height}: Approve and Watch selected the expected project.')
        finally:
            terminal.close()


def selected_root(width, height, kind):
    with tempfile.TemporaryDirectory(prefix='tinkery-TEST-sdlc-root-selection-') as directory:
        root = Path(directory).resolve()
        selected = root / 'TEST selected checkout'
        checkout(root)
        checkout(selected)
        argument = {'absolute': str(selected), 'relative': selected.name, 'current': '.'}[kind]
        expected = root if kind == 'current' else selected
        other = selected if kind == 'current' else root
        terminal = Terminal(root, width, height, ['--operator', 'agent', '--home',
            '--sdlc-root', argument, '--seed-session-root', root / 'TEST-sessions'])
        try:
            terminal.wait(lambda screen: MENU in screen)
            assert calls(root) == calls(selected) == [], 'a script ran before Watch opened'
            terminal.send(b'5', 'No runs in this checkout yet.')
            assert calls(expected) == [['status', '--json']], (argument, calls(expected))
            assert calls(other) == [], 'a different checkout ran'
            terminal.quit()
            print(f'{kind} root at {width}x{height}: only the selected checkout ran.')
        finally:
            terminal.close()


if __name__ == '__main__':
    refused_roots()
    for width, height in [(80, 24), (100, 30)]:
        for kind in ['absolute', 'relative', 'current']:
            selected_root(width, height, kind)
        for kind in ['root', 'subfolder', 'override', 'outside', 'no-install']:
            launch_project(width, height, kind)
