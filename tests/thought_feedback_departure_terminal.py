"""Thought save feedback ends when Tend opens Shape, without model requests."""
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
                thought.flow.send(term, b't', 'F3 Save thought and open Tend')
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


if __name__ == '__main__':
    for size in [(80, 24), (100, 30)]:
        departure(size)
    print('Save feedback clears after Shape at 80x24 and 100x30. No provider requests.')
