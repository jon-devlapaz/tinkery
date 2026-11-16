"""Mixed garden identity and authority through the real terminal, with TEST records only."""
import hashlib
import json
import os
from pathlib import Path
import tempfile

import tend_thought_terminal as thought

flow = thought.flow
FIRST = 'TEST same first words\nTEST exact first original'
SECOND = 'TEST same first words\nTEST exact second original'


def store(root, repo, words):
    folder = root/'links'
    folder.mkdir(exist_ok=True)
    name = 'thought-' + hashlib.sha256(os.fsencode(repo)).hexdigest() + '-' + hashlib.sha256(words.encode()).hexdigest() + '.json'
    path = folder/name
    path.write_text(json.dumps(dict(schema=1, repo=str(repo), words=words)))
    path.chmod(0o600)
    return path


def records(root):
    return {str(p.relative_to(root)): p.read_bytes() for folder in ['links', 'sessions', 'records']
            for p in (root/folder).rglob('*') if p.is_file()}


def click_label(term, text):
    lines = term.screen.text().splitlines()
    y = next(y for y, line in enumerate(lines[4:-4], 4) if text in line)
    x = lines[y].index(text) + 3
    return (f'\x1b[<0;{x+1};{y+1}M\x1b[<0;{x+1};{y+1}m').encode()


def mixed_identity_and_link_guards(size):
    with tempfile.TemporaryDirectory(prefix='tinkery-TEST-mixed-') as directory:
        root = Path(directory).resolve()
        repo = flow.fixture(root)
        for i in range(2):
            flow.update_goal(root, f'goal-{i}', goal=f'TEST goal {i}')
        first = store(root, repo, FIRST)
        store(root, repo, SECOND)
        prefix = first.name.rsplit('-', 1)[0]
        (first.parent/f'{prefix}-broken.json').write_text('{')
        original = records(root)
        term = thought.launch(root, repo, size, provider=True, budget=0)
        try:
            screen = flow.wait(term, 'TEST same first words')
            assert 'Thoughts:' not in screen, screen
            assert all('Thought:' not in line for line in screen.splitlines()[:-3]), screen
            flow.send(term, b'jjj', 'Thought: TEST same first words')
            screen = flow.send(term, b'\r', 'TEST exact second original')
            assert 'TEST exact first original' not in screen, screen
            flow.send(term, b'\x1b', 'Thought: TEST same first words')
            flow.send(term, b'k', 'Thought: TEST same first words')
            screen = flow.send(term, b'\r', 'TEST exact first original')
            assert 'TEST exact second original' not in screen, screen
            flow.send(term, b'j', 'TEST exact second original')
            screen = flow.send(term, b'\x1b', 'Thought: TEST same first words')
            assert '1 thoughts could not be read.' in screen.splitlines()[-3], screen
            flow.send(term, b'\r', 'TEST exact second original')
            flow.send(term, b'\x1b', 'Thought: TEST same first words')
            flow.send(term, b'a', 'all · a this repo')
            assert 'TEST same first words' not in term.screen.text()
            flow.send(term, b'a', 'Thought: TEST same first words')
            flow.send(term, b'\r', 'TEST exact second original')
            added = store(root, repo, 'AAA TEST inserted before both thoughts')
            flow.send(term, b'\x1b', 'Thought: TEST same first words')
            flow.send(term, b'\r', 'TEST exact second original')
            flow.send(term, b'\x1b', 'Thought: TEST same first words')
            original[str(added.relative_to(root))] = added.read_bytes()
            assert records(root) == original, 'Opening and closing details rewrote originals'
            flow.send(term, b'l', 'Links connect goals, not thoughts.')
            flow.send(term, b'f', 'Find connections uses goals, not thoughts.')
            screen = flow.send(term, b's', 'Thought: TEST same first words')
            assert 'tinkery › shape' not in screen and 'ready' not in screen, screen
            assert records(root) == original and not flow.requests(root), 'Thought action acquired goal authority'
            flow.send(term, b'kkkkk', '› TEST goal 0')
            before = term.screen.text().splitlines()[4:-4]
            flow.send(term, b'l', 'enter link')
            after = term.screen.text().splitlines()[4:-4]
            assert [line for line in before if any('\u2801' <= c <= '\u28ff' for c in line)] == [
                line for line in after if any('\u2801' <= c <= '\u28ff' for c in line)], f'Link mode moved the graph:\n{before!r}\n{after!r}'
            lines = term.screen.text().splitlines()
            diamond = [(x, y) for y, line in enumerate(lines[4:size[1]//2], 4)
                       for x, char in enumerate(line) if x > size[0]*2//3 and '\u2801' <= char <= '\u28ff']
            assert diamond, term.screen.text()
            x = round((min(x for x, y in diamond) + max(x for x, y in diamond))/2)
            y = round((min(y for x, y in diamond) + max(y for x, y in diamond))/2)
            click = (f'\x1b[<0;{x+1};{y+1}M\x1b[<0;{x+1};{y+1}m').encode()
            flow.send(term, click, 'enter link')
            assert records(root) == original, 'Thought node click wrote a needs link'
            flow.send(term, click_label(term, 'AAA TEST'), 'enter link')
            assert records(root) == original, 'Thought label click wrote a needs link'
            flow.send(term, b'j', 'TEST goal 0 needs TEST goal 1')
            flow.send(term, b'j', 'TEST goal 0 needs TEST goal 1')
            assert records(root) == original, 'Link navigation wrote a needs link'
            flow.send(term, b'\x1b', '› TEST goal 0')
            flow.send(term, b'l', 'enter link')
            flow.send(term, click_label(term, 'TEST goal 1'), 'needs TEST goal 1 · not ready')
            assert json.loads((root/'links'/'needs.json').read_text()) == {
                'version': 1, 'needs': [{'goal': 'goal-0', 'needs': 'goal-1'}]}
            flow.send(term, click_label(term, 'AAA TEST'), 'Thought: AAA TEST')
            flow.send(term, b'\r', 'AAA TEST inserted before both thoughts')
            flow.send(term, b'g', 'enter choose')
            assert not flow.requests(root)
        finally:
            term.close()
        term = thought.launch(root, repo, size, provider=True, budget=0)
        try:
            flow.send(term, b'jjj', 'Thought: TEST same first words')
            flow.send(term, b'\r', 'TEST exact first original')
            first.unlink()
            flow.send(term, b'\x1b', 'Thought unavailable. Select another entry.')
            assert '› TEST goal 0' in term.screen.text()
            assert not flow.requests(root)
        finally:
            term.close()


def thoughts_only_and_empty(size):
    with tempfile.TemporaryDirectory(prefix='tinkery-TEST-thoughts-only-') as directory:
        root = Path(directory).resolve()
        repo = flow.fixture(root)
        for path in (root/'sessions').glob('*/ledger.json'):
            value = json.loads(path.read_text())
            value['status'] = 'stopped'
            path.write_text(json.dumps(value))
        term = thought.launch(root, repo, size, provider=True, budget=0)
        try:
            flow.wait(term, 'No active goals are recorded')
            flow.send(term, b't', 'enter choose')
            thought.paste(term, SECOND)
            flow.send(term, b'\x1bOR', 'Thought saved.')
            flow.send(term, b'\r', 'TEST exact second original')
            flow.send(term, b'\x1b', 'Thought: TEST same first words')
            assert 'No active goals are recorded' not in term.screen.text()
            assert not flow.requests(root)
        finally:
            term.close()
        term = thought.launch(root, repo, size, provider=True, budget=0)
        try:
            flow.wait(term, 'Thought: TEST same first words')
            flow.send(term, b'\r', 'TEST exact second original')
            flow.send(term, b'\x1b', 'Thought: TEST same first words')
            flow.send(term, b'a', 'all · a this repo')
            flow.send(term, b'a', 'Thought: TEST same first words')
            flow.send(term, b'\r', 'TEST exact second original')
            assert not flow.requests(root)
        finally:
            term.close()


if __name__ == '__main__':
    for size in [(80, 24), (100, 30)]:
        mixed_identity_and_link_guards(size)
        thoughts_only_and_empty(size)
    print('Mixed garden identity, pointer/link guards, refresh, scope, reader return, restart and empty gardens passed at 80x24 and 100x30 without colour. Zero provider calls.')
