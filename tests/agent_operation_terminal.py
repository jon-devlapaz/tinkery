"""Authorized Think → Shape → Approve, TEST roots and stub replies, never a live provider."""
import copy
import json
import os
import tempfile
from pathlib import Path
import test_launch as subprocess
from shape_answer_terminal import Run, BINARY, QUESTION, SEED, F2, F3, F10, helper

ACTOR = {'by': 'agent', 'agent': 'TEST-Codex', 'authorizing_operator': 'TEST-person',
         'source': 'TEST explicit permission for this disposable Think and Shape run',
         'class': {'door': 'two_way', 'door_reason': 'TEST disposable session, no execution',
                   'blast_radius': 'small', 'blast_radius_reason': 'TEST root only'}}
NOTE = 'External intake does not support agent confirmations yet.'


def status(session):
    return json.loads(subprocess.check_output(
        [BINARY, 'seed', 'status', '--json', session], env=subprocess.environment()))


def journey(width, height):
    with tempfile.TemporaryDirectory(prefix='tinkery-TEST-agent-operation-') as directory:
        root = Path(directory).resolve()
        (root / 'README.md').write_text('# TEST\n\nLocal only.\n')
        checkout = root / 'TEST-sdlc'
        script = checkout / '_system/scripts/sdlc.py'
        script.parent.mkdir(parents=True)
        script.write_text('print(\'{"protocol":"tink-sdlc","api_version":1,"runs":[]}\')\n')
        run = Run(root, width, height, 2, [QUESTION, SEED], authorization=ACTOR, sdlc_root=checkout)
        try:
            run.wait('1  think')
            run.send(b'1', "What's on your mind?")
            run.wait('declared: agent')
            run.type('Refusals feel noisy.')
            run.send(F2, 'May refusals show private text?')
            run.type('never')
            run.send(F2, "Nothing I'm unsure about")
            run.send(F3, 'You are confirming')
            run.read_until('Caller-declared confirmation attribution:')
            os.write(run.master, b'\x1b[6~' * 12)
            run.send(b'confirm\r', 'Saved.')
            record = json.loads(next(run.records.iterdir()).read_text())
            assert record['goal_confirmation'] == ACTOR
            session = run.sessions / record['seed_me_session']
            ledger = helper.load(session)
            goal = ledger['goal']
            origin = next(n for n in ledger['nodes'] if n['id'] == ledger['origin'])
            assert origin['authority'] == 'delegated'
            assert origin['delegation']['agent'] == ACTOR['agent']
            assert origin['delegation']['authorizing_operator'] == ACTOR['authorizing_operator']
            assert origin['delegation']['authorization_source'] == ACTOR['source']
            assert any(e['observed'] == 'Refusals feel noisy.' for e in origin['evidence'])
            run.send(F10, '3  shape')
            run.open_shape()
            run.wait('F2 look into the next question')
            run.send(F2, 'Probe: pre-mortem')
            run.type('TEST private text could appear.')
            run.send(F2, 'Keep records local or synced?')
            run.type('Local only.')
            run.send(F2, 'F3 confirm seed r1')
            os.write(run.master, b'\x1b[6~' * 20)
            run.type('TEST I confirm r1 for this isolated run.')
            run.send(F3, 'Seed ready: you confirmed r1, and it still matches.')
            ledger = helper.load(session)
            assert ledger['goal'] == record['goal'] == goal
            for node in ledger['nodes']:
                if node['id'] in ('storage', 'probe-pre-mortem'):
                    assert node['authority'] == 'delegated', node
                    for field in ('agent', 'authorizing_operator', 'source'):
                        assert ACTOR[field] in node['authority_source'], node
                    assert "operator's own answer" not in node['authority_source']
            receipt = json.loads((session / 'seed-confirmation.json').read_text())
            assert receipt['schema_version'] == 3 and receipt['operator'] == 'agent'
            assert receipt['actor'] == ACTOR
            assert status(session)['handoff']['verified'] is True
            run.read_until(NOTE)
            assert 'c copy intake' not in run.flat()
            assert ACTOR['agent'] in (session / 'seed-contract.md').read_text()
            assert len(run.calls()) == 2
            # Approve must disclose the human-only consumer, not a missing-checkout excuse.
            run.send(F10, '4  approve')
            run.send(b'4', 'Seed 1 of 1')
            run.read_until(NOTE)
            assert 'c copy' not in run.flat() and 'sdlc.py' not in run.flat()
            run.send(b'c', NOTE)
            # Changed, missing or malformed attribution must not panic or verify.
            for changed in [None, {}, {'by': 'agent'}, {**ACTOR, 'agent': 'TEST-imposter'}]:
                altered = copy.deepcopy(receipt)
                if changed is None:
                    altered.pop('actor')
                else:
                    altered['actor'] = changed
                (session / 'seed-confirmation.json').write_text(json.dumps(altered))
                assert status(session)['handoff']['verified'] is False
            (session / 'seed-confirmation.json').write_text(json.dumps(receipt))
            assert status(session)['handoff']['verified'] is True
            print(f'Authorized Think → Shape → Approve {width}x{height}: exact goal, named actors, '
                  '2 stub Shape calls, intake withheld, changed attribution refused')
        finally:
            run.close()


def refused_contexts():
    with tempfile.TemporaryDirectory(prefix='tinkery-TEST-agent-context-') as directory:
        root = Path(directory)
        context = root / 'actor.json'
        invalid = []
        for field, value in [('agent', ''), ('authorizing_operator', ''), ('source', ' '),
                             ('class.door', 'one_way'), ('class.blast_radius', 'large'),
                             ('class.door_reason', ''), ('class.blast_radius_reason', '')]:
            actor = copy.deepcopy(ACTOR)
            if '.' in field:
                parent, key = field.split('.')
                actor[parent][key] = value
            else:
                actor[field] = value
            invalid.append(json.dumps(actor))
        invalid += ['{', '{}', json.dumps({'by': 'unrecorded'})]
        for contents in invalid:
            context.write_text(contents)
            result = subprocess.run([BINARY, '--operator', 'agent', '--agent-confirmation', context,
                                    '--snapshot', '--seed-session-root', root / 'sessions'],
                                    capture_output=True, env=subprocess.environment())
            assert result.returncode != 0, (contents, result.stdout, result.stderr)
            assert not (root / 'sessions').exists()
        context.write_text(json.dumps(ACTOR))
        for args in [['--operator', 'human'], [], ['--operator', 'agent', '--agent-confirmation', str(context)]]:
            result = subprocess.run([BINARY, *args, '--agent-confirmation', context, '--snapshot'],
                                    capture_output=True, env=subprocess.environment())
            assert result.returncode != 0
        missing = subprocess.run([BINARY, '--operator', 'agent', '--agent-confirmation', root / 'absent',
                                  '--snapshot'], capture_output=True, env=subprocess.environment())
        assert missing.returncode != 0
        print('14 invalid/missing/mismatched agent contexts refused before session writes')


if __name__ == '__main__':
    refused_contexts()
    journey(100, 30)
    os.environ['NO_COLOR'] = '1'
    journey(80, 24)
