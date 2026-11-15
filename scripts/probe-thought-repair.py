#!/usr/bin/env python3
"""Reproduce offset link writes and failed directory sync, then restore and rebuild."""
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT/'tests'))
import test_launch
import tend_thought_terminal as thought


def main():
    env = test_launch.environment()
    paths = [ROOT/'src/tend/mod.rs', ROOT/'src/tend/thought.rs']
    originals = {path: path.read_text() for path in paths}

    def build():
        test_launch.run(['cargo', '+stable', 'build', '--locked', '--quiet'],
                        cwd=ROOT, env=env, check=True)

    def mutate(path, old, new):
        source = originals[path]
        if source.count(old) != 1:
            raise SystemExit('Mutation must match exactly once')
        path.write_text(source.replace(old, new))
        build()

    try:
        mutate(paths[0], '        if self.thought_summary().is_some() {',
               '        if self.connection_scope() && self.thoughts.as_ref().is_some_and(|t| !t.summary().is_empty()) {')
        for size in [(80, 24), (100, 30)]:
            command = ('import sys; sys.path.insert(0, "tests"); '
                       f'import tend_thought_terminal as t; t.repository_link({size!r})')
            result = test_launch.run([sys.executable, '-c', command], cwd=ROOT, env=env,
                                     capture_output=True, text=True, timeout=60)
            if result.returncode == 0 or 'Empty-space click wrote a needs link' not in result.stderr:
                raise SystemExit(f'Link probe failed at {size}:\n{result.stdout}{result.stderr}')
            print(f'Link offset rejected at {size}: Empty-space click wrote a needs link', flush=True)
        paths[0].write_text(originals[paths[0]])
        mutate(paths[1], 'fs::File::open(root).and_then(|f| f.sync_all())',
               'Err::<(), _>(std::io::Error::other("TEST directory sync failed"))')
        for size in [(80, 24), (100, 30)]:
            import tempfile
            with tempfile.TemporaryDirectory(prefix='tinkery-TEST-sync-') as directory:
                root = Path(directory).resolve()
                repo = thought.flow.fixture(root)
                term = thought.launch(root, repo, size)
                try:
                    thought.flow.send(term, b't', 'F2 save thought only')
                    thought.paste(term, thought.WORDS)
                    screen = thought.flow.send(term, thought.flow.F2, 'persistence is uncertain.')
                    assert 'Thought saved, but' in screen, screen
                    assert 'Thought not saved' not in screen, screen
                    import json
                    assert json.loads(thought.saved(root)[0].read_bytes())['words'] == thought.WORDS
                    assert not thought.flow.requests(root)
                    print(f'Directory sync failure disclosed after save at {size}', flush=True)
                finally:
                    term.close()
    finally:
        for path, source in originals.items():
            path.write_text(source)
        build()
    test_launch.run([sys.executable, 'tests/tend_thought_terminal.py'], cwd=ROOT, env=env, check=True)


if __name__ == '__main__':
    main()
