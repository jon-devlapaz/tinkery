"""Long synthetic clinic-planning checks retain every word in the structured Taste views."""
import json
import os
from pathlib import Path
import tempfile
from taste_quiet_terminal import REPO, run

if __name__ == '__main__':
    golden = json.loads((REPO / 'tests/fixtures/taste-structure-handoff-355.json').read_text())
    for width, height in [(100, 30), (80, 24)]:
        for mono in [False, True]:
            with tempfile.TemporaryDirectory(prefix='tinkery-TEST-structure-355-') as directory:
                base = Path(directory).resolve()
                baseline = os.environ.get('TINKERY_TASTE_STRUCTURE_BASELINE')
                before = run(base, width, height, mono, Path(baseline), legacy=True, long_checks=True) if baseline else None
                result = run(base, width, height, mono, REPO / 'target/debug/tinkery', long_checks=True)
                for action, expected in golden.items():
                    assert result['copies'][action] == expected.replace('{ROOT}', str(base)).encode(), action
                if before:
                    assert result['copies'] == before['copies'], 'long-check parent/new copy bytes changed'
                if (width, height, mono) == (100, 30, False):
                    (REPO / 'target/TEST-taste-structure-after-list-355.txt').write_text(result['list'])
                    (REPO / 'target/TEST-taste-structure-after-review-355.txt').write_text(result['main'])
                print(json.dumps(dict(journey='structured Taste/long verbatim checks/evidence/identical copies',
                                      width=width, height=height, no_color=mono,
                                      parent_comparison=bool(before), provider_calls=0, source_reads=3)))
