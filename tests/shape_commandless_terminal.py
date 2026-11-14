"""Commandless checks cannot confirm; both operator repairs are zero-call PTY actions."""
import json
import os
from pathlib import Path
import tempfile

from shape_answer_terminal import ESC, F2, Run, SEED, completion_review, files, helper, make as goal_session
from shape_terminal import make, seed

SIGNOFF = "agent evidence can't sign it off"
READING = "1 judged check: a person signs it off after seeing the built result; agent evidence can't."


BROWSER = (
    "In guest chat A, type exactly Draft check: explain fractions. Leave it unsent. "
    "Reload the same tab. Pass if the box restores the exact text with the period and A shows no sent copy. "
    "Open a distinct guest chat B in the same tab and reload. Pass if B never shows A's draft. "
    "Return to A, clear the box, and reload. Pass if the box stays empty. "
    "Fail if the text changes, a sent copy appears, B shows A's draft, or the cleared text returns."
)


def contract_journey(width, height, agent):
    with tempfile.TemporaryDirectory(prefix='tinkery-TEST-contract-') as directory:
        root = Path(directory).resolve()
        session = make(root / 'TEST-sessions', 'draft', goal='TEST exact goal', question=False)
        seed(session)
        path = session / 'seed-contract.md'
        old = helper.checks_block(helper.read_checks(session))
        goal = helper.load(session)['goal']
        records = [
            dict(statement='Standard library only', kind='provisional', source='TEST goal',
                 expect='No dependencies', cwd='.', missing='Dependency audit is absent'),
            dict(statement='Guest browser draft stays isolated', kind='judge', source='TEST goal'),
        ]
        helper.set_checks(session, records)
        path.write_text(path.read_text().replace(old, helper.checks_block(records)))
        run = Run(root, width, height, budget=0, agent=agent)
        evidence = os.environ.get('TINKERY_TEST_855_EVIDENCE')

        def capture(label):
            if evidence:
                (Path(evidence) / f'{width}x{height}-{label}.txt').write_text(run.screen.text())

        def paste(words, shown):
            run.send(b'\x1b[200~' + words.encode() + b'\x1b[201~', shown)

        def read_end():
            run.read_until('End of seed', whole_line=True)

        def save():
            run.send(F2, 'Check saved. Read the changed draft, then confirm. No model ran.')

        try:
            run.open_shape()
            run.wait('Tink Contract r1')
            run.wait('Missing manual steps. F1: Edit check 2.')
            first = run.flat()
            order = ['Pass/fail checks (proposed)', '1. Standard library only', 'Missing command.',
                     '2. Guest browser draft stays isolated', 'Missing manual steps.']
            assert all(first.index(a) < first.index(b) for a, b in zip(order, order[1:])), first
            assert 'You are confirming' not in first or first.index(order[-1]) < first.index('You are confirming')
            capture('first')
            run.read_until('Evidence')
            read_end()
            run.type('confirm')
            before = files(session)
            run.send(F2, 'Cannot confirm: check 1. Missing command. Nothing was confirmed.')
            assert files(session) == before

            # Edit the second check while the first remains incomplete.
            run.choose('Edit check 2', 'your how-to-judge words')
            run.wait('Edit check 2 Guest browser draft stays isolated')
            paste(BROWSER, 'the cleared text returns.')
            save()
            run.wait('Missing command. F1: Edit check 1.')
            assert 'Missing manual steps.' not in run.flat()
            capture('repaired')
            assert helper.read_checks(session) == [records[0], {**records[1], 'how_to_judge': BROWSER}]
            run.send(F2, "PgDn or Down to 'End of seed' first.")
            run.choose('Edit check 2', 'the cleared text returns.')
            run.wait('Edit check 2 Guest browser draft stays isolated')
            capture('reopened')
            save()
            assert helper.read_checks(session) == [records[0], {**records[1], 'how_to_judge': BROWSER}]

            # Another writer replaces this card while its old editor remains open.
            run.choose('Edit check 2', 'the cleared text returns.')
            run.send(b' TEST UNSAVED CORRECTION', 'TEST UNSAVED CORRECTION')
            current = helper.read_checks(session)
            replaced = [current[0], {**current[1], 'statement': 'TEST another browser check'}]
            helper.set_checks(session, replaced)
            path.write_text(path.read_text().replace(helper.checks_block(current), helper.checks_block(replaced)))
            other_writer = files(session)
            stale_notice = 'The draft changed. Read it again before changing a check.'
            run.send(F2, stale_notice)
            run.wait('Edit check 2 Guest browser draft stays isolated')
            run.wait('TEST UNSAVED CORRECTION')
            run.send(F2, 'Saving your check. No model runs.')
            run.wait(stale_notice)
            assert files(session) == other_writer, 'A stale retry overwrote the new card.'
            run.send(ESC, 'How to judge cancelled. Nothing was saved.')
            helper.set_checks(session, current)
            path.write_text(path.read_text().replace(helper.checks_block(replaced), helper.checks_block(current)))
            # Reopen the session to read the currently saved cards.
            run.menu()
            run.open_shape()
            run.wait('Tink Contract r1')
            run.type('confirm')

            run.choose('Edit check 1', 'your check command')
            run.wait('Edit check 1 Standard library only')
            command = 'python3 tools/check_stdlib.py'
            paste(command, command)
            save()
            assert helper.read_checks(session) == [
                {**records[0], 'cmd': command}, {**records[1], 'how_to_judge': BROWSER}]
            assert 'Missing command.' not in run.flat()
            read_end()
            run.send(F2, 'Seed ready:')
            assert helper.status(session)['handoff']['verified']
            assert helper.load(session)['goal'] == goal
            assert helper.read_checks(session)[1]['how_to_judge'] == BROWSER
            assert f'you judge: {BROWSER}' in path.read_text()
            assert not run.calls()
            print(f'Tink Contract passed {width}x{height}: exact edit, reopen, read and confirm; zero calls')
        finally:
            run.close()


def draft_and_redraft():
    """Replay the proposal and a redraft with two stub replies, never a real model."""
    with tempfile.TemporaryDirectory(prefix='tinkery-TEST-contract-redraft-') as directory:
        root = Path(directory).resolve()
        criterion = "Guest browser draft stays isolated"
        goal = f"Keep unsent chat drafts.\nDone when: {criterion}"
        session = goal_session(root / 'TEST-sessions', goal=goal, question=False)
        manual = dict(statement=criterion, kind='judge', source='TEST goal')
        draft = {**SEED, 'checks': [{**manual, 'how_to_judge': BROWSER}], 'found': []}
        draft['review'] = completion_review(draft['checks'])
        draft['review']['failure_cases'] = [
            "Fail if the period is missing, a sent copy appears, B sees A's draft, or it returns after clearing."]
        changed = {**draft, 'checks': [manual]}
        run = Run(root, 100, 30, budget=2, replies=[draft, changed], agent=True)
        try:
            run.open_shape()
            run.wait('F2 look into the next question')
            run.send(F2, 'Tink Contract r1')
            run.wait('Manual steps: In guest chat A, type exactly Draft check: explain fractions.')
            run.read_until('the cleared text returns.')
            run.read_until('Must fail: Fail if the period is missing')
            assert helper.read_checks(session) == draft['checks']
            run.choose('Revise the draft', 'your change')
            run.type('TEST revisit the steps.')
            run.send(F2, 'Tink Contract r2')
            run.wait('Missing manual steps. F1: Edit check 1.')
            run.read_until('End of seed', whole_line=True)
            run.type('confirm')
            before = files(session)
            run.send(F2, 'Cannot confirm: check 1. Missing manual steps. Nothing was confirmed.')
            assert files(session) == before
            assert len(run.calls()) == 2
            assert helper.load(session)['goal'] == goal
            print('Tink Contract draft and redraft passed 100x30: 2 stub calls, no provider calls')
        finally:
            run.close()


def objective_browser_check():
    """A stub proves Shape transports the planned check without turning it into sign-off."""
    with tempfile.TemporaryDirectory(prefix='tinkery-TEST-objective-check-') as directory:
        root = Path(directory).resolve()
        criterion = 'Guest browser draft stays isolated'
        goal = f"Keep unsent chat drafts.\nDone when: {criterion}"
        session = goal_session(root / 'TEST-sessions', goal=goal, question=False)
        saved = helper.load(session)
        state = helper.editable(saved)
        original = 'Use this exact draft: Draft check: explain fractions.  '
        state['nodes'][0]['evidence'] = [
            {'checked': 'Tinkery original 1', 'observed': original, 'at': '2026-11-14T00:00:00Z'}
        ]
        helper.publish(session, state, saved['version'], 'TEST browser intent')
        (root / 'TEST-browser-check.sh').write_text('touch check-ran\n')
        actions, failure = BROWSER.rsplit(' Fail if ', 1)
        record = dict(statement=criterion, kind='cmd', source='TEST-browser-check.sh:1',
                      cmd='sh TEST-browser-check.sh', cwd='.', expect=actions)
        reply = {**SEED, 'checks': [{**record, 'failure_case': 'Fail if ' + failure}], 'found': []}
        reply['review'] = {key: value for key, value in completion_review([record]).items()
                           if key != 'failure_cases'}
        run = Run(root, 100, 30, budget=1, replies=[reply], agent=True)
        try:
            run.open_shape()
            run.wait('F2 look into the next question')
            run.send(F2, 'Tink Contract r1')
            run.wait('Command: sh TEST-browser-check.sh')
            run.read_until('Draft check: explain fractions.')
            run.read_until("Pass if B never shows A's draft.")
            run.read_until('Pass if the box stays empty.')
            run.read_until('Must fail: Fail if the text changes')
            run.read_until('End of seed', whole_line=True)
            run.type('confirm')
            run.send(F2, 'Seed ready:')
            assert helper.status(session)['handoff']['verified']
            assert helper.read_checks(session) == [record]
            assert not (root / 'check-ran').exists(), 'Shape ran the command without run authorization'
            request, = run.calls()
            origin = request['input']['ledger']['nodes'][0]
            assert origin['evidence'][0]['observed'] == original
            assert request['input']['goal'] == goal
            print('Objective browser check passed 100x30: draft and confirmation, 1 stub call, no check execution')
        finally:
            run.close()


def journey(width, height, agent, repair):
    with tempfile.TemporaryDirectory(prefix='tinkery-TEST-commandless-') as directory:
        root = Path(directory).resolve()
        session = make(root / 'TEST-sessions', 'draft', goal='TEST exact goal', question=False)
        seed(session)
        old = helper.checks_block(helper.read_checks(session))
        record = dict(statement='standard library only', kind='provisional', source='TEST goal',
                      expect='No dependencies', cwd='.', missing='Dependency audit is absent')
        # #790: judge words that describe agent evidence must not hide the human sign-off.
        words = ('  python3 -I -S -B tools/check_stdlib.py  ' if repair != 'judge'
                 else '  Agent technical review only, not human acceptance.  ')
        if repair == 'proposed':
            record['cmd'] = words
        helper.set_checks(session, [record])
        path = session / 'seed-contract.md'
        path.write_text(path.read_text().replace(old, helper.checks_block([record])))
        run = Run(root, width, height, budget=0, agent=agent)
        try:
            run.open_shape()
            run.wait('Tink Contract r1')
            run.read_until('End of seed', whole_line=True)
            run.send(b'confirm', 'your words confirm')
            if repair != 'proposed':
                before = files(session)
                run.send(F2, 'Cannot confirm: check 1. Missing command.')
                assert files(session) == before, 'Refusal wrote a skip, receipt or seed'
                run.choose('Make check 1 a judgment' if repair == 'judge' else 'Edit check 1',
                           'your how-to-judge words' if repair == 'judge' else 'your check command')
                if repair == 'judge':
                    run.wait(SIGNOFF)
                run.send(b'\x1b[200~' + words.encode() + b'\x1b[201~', words.strip())
                if repair == 'judge':
                    os.write(run.master, ESC)
                    run.wait('How to judge cancelled. Nothing was saved.')
                    assert files(session) == before, 'Cancelling a judgment saved something'
                    run.choose('Make check 1 a judgment', SIGNOFF)
                    run.send(b'\x1b[200~' + words.encode() + b'\x1b[201~', words.strip())
                run.send(F2, 'Check saved. Read the changed draft, then confirm. No model ran.')
                run.read_until('End of seed', whole_line=True)
                if repair == 'judge':
                    run.read_until(READING)
            run.send(F2, 'Seed ready:')
            if repair == 'judge':
                run.wait(READING)
            else:
                assert 'judged check' not in run.flat(), 'A command check claimed human sign-off'
            assert helper.status(session)['handoff']['verified']
            saved = json.loads((session / 'seed-checks.json').read_text())[0]
            assert saved['statement'] == record['statement']
            assert saved['kind'] == ('judge' if repair == 'judge' else 'provisional')
            assert saved['how_to_judge' if repair == 'judge' else 'cmd'] == words
            if repair == 'judge':
                assert 'you judge:' in path.read_text()
            assert run.calls() == [], 'Check repair or confirmation called a model'
            assert not list(session.rglob('*attest*')), 'Saving a judgment created an attestation'
            print(f'Shape commandless passed {width}x{height}: {"agent" if agent else "human"}, {repair}, zero calls')
        finally:
            run.close()


if __name__ == '__main__':
    draft_and_redraft()
    objective_browser_check()
    for size in [(80, 24), (100, 30)]:
        contract_journey(*size, agent=size[0] == 100)
        for agent in [False, True]:
            for repair in ['command', 'judge', 'proposed']:
                journey(*size, agent, repair)
