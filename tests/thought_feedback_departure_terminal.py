"""Thought feedback clears on departure and preserves honest read errors without requests."""
from pathlib import Path
import tempfile

import tend_thought_terminal as thought


def departure(size):
    with tempfile.TemporaryDirectory(prefix='tinkery-TEST-thought-departure-') as directory:
        root = Path(directory).resolve()
        repo = thought.flow.fixture(root)
        original = {p: p.read_bytes() for p in (root/'sessions').rglob('*') if p.is_file()}
        term = thought.launch(root, repo, size, provider=True, budget=0)
        try:
            for outcome in ['Thought saved.', 'Thought already saved.']:
                thought.flow.send(term, b't', 'enter choose')
                thought.paste(term, 'TEST notice lifetime')
                thought.flow.send(term, b'\x1bOR', outcome)
                thought.flow.send(term, b'\r', 's shape')
                thought.flow.send(term, b's', 'tinkery › shape')
                thought.flow.send(term, b'\x1b', 'tinkery › tend')
                screen = thought.flow.send(term, b'\x1b', 'Thoughts: 1')
                assert outcome not in screen, screen
                assert 'Thought: TEST notice lifetime' in screen, screen
                assert not thought.flow.requests(root)
                thought.no_goal(root, original)
        finally:
            term.close()


def failed_refresh(size):
    """A failed read reports its error, not the previous read's warning count."""
    with tempfile.TemporaryDirectory(prefix='tinkery-TEST-thought-refresh-') as directory:
        root = Path(directory).resolve()
        repo = thought.flow.fixture(root)
        original = {p: p.read_bytes() for p in (root/'sessions').rglob('*') if p.is_file()}
        term = thought.launch(root, repo, size, provider=True, budget=0)
        try:
            thought.flow.send(term, b't', 'enter choose')
            thought.paste(term, 'TEST refresh failure')
            thought.flow.send(term, b'\x1bOR', 'Thought saved.')
            path = thought.saved(root)[0]
            prefix = path.name.rsplit('-', 1)[0]
            (path.parent/f'{prefix}-malformed.json').write_text('{')
            thought.flow.send(term, b'v', '1 thoughts could not be read.')
            thought.flow.send(term, b'\x1b', 'Thoughts: 1')
            (root/'links').rename(root/'saved-links')
            (root/'links').write_text('TEST not a directory')
            thought.flow.send(term, b'v', 'Thoughts could not be read:')
            screen = thought.flow.send(term, b'\x1b', 'Thoughts could not be read. Press v for details.')
            assert '1 thoughts could not be read.' not in screen, screen
            assert 'Thought saved.' not in screen, screen
            assert not thought.flow.requests(root)
            thought.no_goal(root, original)
        finally:
            term.close()


if __name__ == '__main__':
    for size in [(80, 24), (100, 30)]:
        departure(size)
        failed_refresh(size)
    print('Save feedback clears after Shape and failed reads at 80x24 and 100x30. No provider requests.')
