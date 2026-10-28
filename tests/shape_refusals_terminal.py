"""#361: recorded unsafe reviews refuse plainly; recoverable framing uses one stub call."""
import json
import os
from pathlib import Path
import tempfile

from shape_answer_terminal import Run, make, helper, REPO, SEED, GOAL, F1, F2


def journey(width, height, case):
    with tempfile.TemporaryDirectory(prefix='tinkery-TEST-361-') as directory:
        root = Path(directory)
        recoverable = case == 'extra-brace'
        session = make(root / 'TEST-sessions', question=not recoverable, operator='agent')
        if recoverable:
            reply = json.dumps({**SEED, 'found': []}) + '}'
        else:
            reply = (REPO / f'tests/fixtures/shape-361/call-{case}.json').read_text()
        run = Run(root, width, height, budget=1, replies=[reply], agent=True)
        try:
            run.open_shape()
            if recoverable:
                run.wait('F2 look into the next question')
                before = helper.load(session)
                run.send(F2, 'Seed draft r1')
                assert helper.read_checks(session) == SEED['checks']
                assert helper.load(session)['nodes'][1]['answer'] == before['nodes'][1]['answer']
                assert (session / 'seed-contract.md').exists()
            else:
                run.wait('Keep records local or synced?')
                words = 'TEST keep these exact words café\r\n  '
                os.write(run.master, b'\x1b[200~' + words.encode() + b'\x1b[201~')
                run.wait('café')
                run.send(F2, f'received {4 if case == 19 else 1} for 2 checks')
                run.wait('Your answer is saved.')
                state = helper.load(session)
                assert next(n for n in state['nodes'] if n['id'] == 'storage')['answer'] == words
                assert not (session / 'seed-contract.md').exists()
                assert not (session / 'seed-checks.json').exists()
                assert not any(n.get('kind') == 'fact' for n in state['nodes'])
            assert helper.load(session)['goal'] == GOAL
            assert not (session / 'seed-confirmation.json').exists()
            assert len(run.calls()) == 1, 'No automatic or manual retry'
            assert json.loads((root / 'shape-replies.json').read_text()) == []
            run.send(F1, 'Drafting runs left this launch: 0 of 1.')
            assert len(run.calls()) == 1
            print(json.dumps(dict(size=[width, height], case=case, stub_calls=1,
                                  live_calls=0, retries=0, confirmed=False)))
        finally:
            run.close()


if __name__ == '__main__':
    for size in [(100, 30), (160, 40)]:
        for case in [19, 20, 'extra-brace']:
            journey(*size, case)
