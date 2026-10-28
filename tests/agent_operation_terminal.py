"""Agent-operated Think → Shape → Approve, TEST roots and stub replies, never a live provider."""
import base64
import copy
import json
import os
import re
import select
import shlex
import tempfile
import time
from pathlib import Path
import test_launch as subprocess
from shape_answer_terminal import Run, BINARY, QUESTION, SEED, ESC, F2, helper, files

ACTOR = {'by': 'terminal', 'declaration': 'declared: agent',
         'action': 'typed confirm at this terminal', 'identity': 'declaration is not verified identity'}
# A why long enough that review needs scrolling at every tested size.
LONG_WHY = 'the work is stuck' + ', and TEST detail keeps this review long' * 80
NOTE = 'Agent-confirmed seed. Intake requires tink-sdlc 1.38.0 or newer;'


def status(session):
    return json.loads(subprocess.check_output(
        [BINARY, 'seed', 'status', '--json', session], env=subprocess.environment()))


def confirmation_after_reading(width, height, correct_typo=False):
    with tempfile.TemporaryDirectory(prefix='tinkery-TEST-confirm-input-') as directory:
        actor = ACTOR
        run = Run(Path(directory), width, height, 4, agent=True, why=LONG_WHY)

        def type_confirmation(data=b'confirm', word='confirm'):
            os.write(run.master, data)
            deadline = time.monotonic() + 5
            while time.monotonic() < deadline:
                if select.select([run.master], [], [], .02)[0]:
                    run.screen.feed(os.read(run.master, 65536))
                if not run.screen.synchronized and any(
                        line.strip() == word for line in run.screen.text().splitlines()):
                    return
            raise AssertionError(f'Typed confirmation did not redraw:\n{run.screen.text()}')

        try:
            run.wait('1  think')
            run.send(b'1', "What's on your mind?")
            run.type('Refusals feel noisy.')
            run.send(F2, 'May refusals show private text?')
            run.type('never')
            run.send(F2, "Ready for your review")
            run.choose('Review the goal', 'type confirm')
            # Typing alone and confirming an unread attribution must write nothing.
            type_confirmation()
            assert not run.records.exists() and not run.sessions.exists()
            run.send(b'\r', 'Read to the end first:')
            assert not run.records.exists() and not run.sessions.exists()
            # Reopen with an empty input, read fully, then wait for typing to redraw.
            run.send(b'\x1b', "Ready for your review")
            run.choose('Review the goal', 'type confirm')
            run.send(b'\x1b[6~' * 200, 'Done when: Refusals never show')
            if correct_typo:
                type_confirmation(b'confirn', 'confirn')
                run.send(b'\r', 'Type confirm, then Enter. Nothing saved.')
                assert not run.records.exists() and not run.sessions.exists()
                type_confirmation(b'\x7f\x1b[200~m\x1b[201~')
            else:
                type_confirmation()
            assert not run.records.exists() and not run.sessions.exists()
            run.send(b'\r', 'Saved.')
            written = list(run.records.iterdir())
            assert len(written) == 1
            record = json.loads(written[0].read_text())
            ledger = helper.load(run.sessions / record['seed_me_session'])
            assert record['goal'] == ledger['goal']
            assert record['goal_confirmation'] == actor
            assert len(list(run.sessions.iterdir())) == 1 and not run.calls()
            print(f'Think confirmation {width}x{height}, typo={correct_typo}: unread refused; '
                  'read then type/redraw/Enter saved exact goal and attribution')
        finally:
            run.close()


def journey(width, height):
    with tempfile.TemporaryDirectory(prefix='tinkery-TEST-agent-operation-') as directory:
        root = Path(directory).resolve()
        (root / 'README.md').write_text('# TEST\n\nLocal only.\n')
        checkout = root / 'TEST-sdlc'
        script = checkout / '_system/scripts/sdlc.py'
        script.parent.mkdir(parents=True)
        script.write_text('print(\'{"protocol":"tink-sdlc","api_version":1,"runs":[]}\')\n')
        run = Run(root, width, height, 6, [QUESTION, SEED], agent=True, sdlc_root=checkout)
        output = bytearray()
        feed = run.screen.feed

        def capture(data):
            output.extend(data)
            feed(data)

        run.screen.feed = capture

        def copies():
            return [base64.b64decode(data).decode() for data in
                    re.findall(rb'\x1b\]52;[^;]*;([^\x07\x1b]*)(?:\x07|\x1b\\)', output)]

        try:
            run.wait('1  think')
            run.send(b'1', "What's on your mind?")
            run.wait('declared: agent')
            run.type('Refusals feel noisy.')
            run.send(F2, 'May refusals show private text?')
            run.type('never')
            run.send(F2, "Ready for your review")
            run.choose('Review the goal', 'You are confirming')
            run.send(b'confirm\r', 'Saved.')
            record = json.loads(next(run.records.iterdir()).read_text())
            assert record['goal_confirmation'] == ACTOR
            session = run.sessions / record['seed_me_session']
            ledger = helper.load(session)
            assert ledger['operator'] == 'agent'
            goal = ledger['goal']
            origin = next(n for n in ledger['nodes'] if n['id'] == ledger['origin'])
            assert origin['authority'] == 'delegated'
            assert origin['delegation'] == {'agent': 'agent'}
            assert any(e['observed'] == 'Refusals feel noisy.' for e in origin['evidence'])
            run.send(ESC, '3  shape')
            run.open_shape()
            run.wait('F2 look into the next question')
            run.send(F2, 'Probe: pre-mortem')
            run.type('TEST private text could appear.')
            run.send(F2, 'Keep records local or synced?')
            run.type('Local only.')
            run.send(F2, 'pgup/dn read to end')
            os.write(run.master, b'\x1b[6~' * 20)
            run.type('TEST I confirm r1 for this isolated run.')
            run.send(F2, 'Seed ready: you confirmed r1, and it still matches.')
            ledger = helper.load(session)
            assert ledger['operator'] == 'agent'
            assert ledger['goal'] == record['goal'] == goal
            for node in ledger['nodes']:
                if node['id'] in ('storage', 'probe-pre-mortem'):
                    assert node['authority'] == 'delegated', node
                    assert node['authority_source'].startswith('The agent '), node
                    assert 'declared: agent' in node['authority_source'], node
                    assert "operator's own answer" not in node['authority_source']
            receipt = json.loads((session / 'seed-confirmation.json').read_text())
            assert receipt['schema_version'] == 3 and receipt['operator'] == 'agent'
            assert receipt['actor'] == ACTOR
            assert status(session)['handoff']['verified'] is True
            run.read_until(NOTE)
            assert 'c copy intake' not in run.flat(), run.screen.text()
            before = files(root)
            os.write(run.master, b'c')
            run.read_until('Approve copies the command that starts the build.')
            assert copies() == [], 'Shape copied a command'
            assert 'declared: agent' in (session / 'seed-contract.md').read_text()
            assert len(run.calls()) == 2
            # Both views disclose agent confirmation; copying never creates a run or approves.
            run.menu()
            run.send(b'4', 'Seed 1 of 1')
            run.read_until(NOTE)
            assert 'c copy command' in run.flat()
            run.send(b'c', 'Copied.')
            assert len(copies()) == 1
            words = shlex.split(copies()[0])
            assert words[:6] == ['cd', str(checkout), '&&', 'python3',
                                 '_system/scripts/sdlc.py', 'new'], words
            assert words[7:12] == ['&&', 'python3', '_system/scripts/sdlc.py',
                                   'stage', words[6]], words
            assert words[12:] == ['1', '--seed-contract', str(session / 'seed-contract.md'),
                                  '--tinkery', str(BINARY)], words
            assert files(root) == before, 'Approve or copying changed files'
            assert len(run.calls()) == 2
            # Changed, missing or malformed attribution must not panic or verify.
            for changed in [None, {}, {'by': 'agent'}, {**ACTOR, 'declaration': 'declared: human'},
                            {**ACTOR, 'identity': 'verified'}, {'by': 'unrecorded'}]:
                altered = copy.deepcopy(receipt)
                if changed is None:
                    altered.pop('actor')
                else:
                    altered['actor'] = changed
                (session / 'seed-confirmation.json').write_text(json.dumps(altered))
                assert status(session)['handoff']['verified'] is False
            (session / 'seed-confirmation.json').write_text(json.dumps(receipt))
            assert status(session)['handoff']['verified'] is True
            print(f'Agent Think → Shape → Approve {width}x{height}: exact goal, agent actor, '
                  '2 stub Shape calls, exact intake copied without writes, invalid attribution refused')
        finally:
            run.close()


def retired_option():
    result = subprocess.run([BINARY, '--operator', 'agent', '--agent-confirmation', 'TEST.json',
                             '--snapshot'], capture_output=True, env=subprocess.environment())
    assert result.returncode != 0
    print('--agent-confirmation is no longer an option')


if __name__ == '__main__':
    retired_option()
    confirmation_after_reading(94, 51)
    confirmation_after_reading(94, 51, correct_typo=True)
    confirmation_after_reading(100, 30)
    confirmation_after_reading(100, 30, correct_typo=True)
    journey(100, 30)
    os.environ['NO_COLOR'] = '1'
    confirmation_after_reading(80, 24)
    confirmation_after_reading(80, 24, correct_typo=True)
    journey(80, 24)
