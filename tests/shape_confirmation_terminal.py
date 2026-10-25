"""F3 confirms only the displayed draft and ledger: real PTYs, TEST roots, zero calls."""
import json
from pathlib import Path
import tempfile

from shape_answer_terminal import F3, Run, files, helper
from shape_terminal import make, seed


def journey(width, height, changed):
    with tempfile.TemporaryDirectory(prefix='tinkery-TEST-displayed-seed-') as directory:
        root = Path(directory).resolve()
        session = make(root / 'TEST-sessions', 'draft', goal='TEST exact goal', question=False)
        seed(session)
        run = Run(root, width, height, budget=0, agent=True)
        try:
            run.open_shape()
            run.wait('Seed draft r1')
            run.read_until('End of seed', whole_line=True)
            words = ('  TEST confirm only what I read  ' if width == 80
                     else '\u2003\tTEST confirm only what I read\u00a0')
            run.send(b'\x1b[200~' + words.encode() + b'\x1b[201~', words)
            path = session / 'seed-contract.md'
            if changed == 'seed':
                path.write_text(path.read_text().replace('TEST draft.', 'TEST unseen replacement.'))
            elif changed == 'ledger':
                current = helper.load(session)
                state = helper.editable(current)
                state['draft']['outcome'] = 'TEST changed after reading'
                helper.publish(session, state, current['version'], 'TEST concurrent writer')
            before = files(session)
            if changed:
                refusal = ('The draft changed. Read it again before confirming.' if changed == 'seed'
                           else 'Another writer changed this session first, so nothing was saved.')
                run.send(F3, refusal)
                run.wait('your words ' + words)
                assert files(session) == before, 'Stale confirmation wrote a skip, receipt or seed'
                assert not (session / 'seed-confirmation.json').exists()
                if changed == 'seed':
                    run.send(F3, "PgDn to 'End of seed' first. Nothing was confirmed.")
                    assert files(session) == before, 'The changed draft skipped the reading gate'
                    run.read_until('End of seed', whole_line=True)
            run.send(F3, 'Seed ready:')
            assert helper.status(session)['handoff']['verified']
            receipt = json.loads((session / 'seed-confirmation.json').read_text())
            assert receipt['source'] == words
            note = next(line for line in path.read_text().split('\n') if line.startswith('Confirmed by:'))
            assert receipt['schema_version'] == 3 and receipt['operator'] == 'agent'
            assert note.startswith(f'Confirmed by: "{words}; caller-declared declared: agent;'), repr(note)
            assert run.calls() == [], 'Confirmation or a refusal called a model'
            print(f'Shape displayed confirmation passed at {width}x{height}: '
                  f'{changed or "unchanged"}, exact words retained, zero calls')
        finally:
            run.close()


if __name__ == '__main__':
    for size in [(80, 24), (100, 30)]:
        for changed in ['', 'seed', 'ledger']:
            journey(*size, changed)
