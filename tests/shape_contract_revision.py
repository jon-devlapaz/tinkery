"""Shape and seed show read one published contract, even when a writer has started another."""
import fcntl
import json
import tempfile
from pathlib import Path

from seed_command import Run as Command
from shape_answer_terminal import Run as Terminal, helper, make
from shape_terminal import seed


def fenced_example():
    with tempfile.TemporaryDirectory(prefix='tinkery-TEST-contract-example-') as directory:
        command = Command(directory, [], budget=0, agent=True)
        session = make(command.sessions, question=False)
        seed(session)
        draft = session / 'seed-contract.md'
        old_text = draft.read_text()
        old_block = helper.checks_block(helper.read_checks(session))
        check = 'TEST review the public copy'
        records = [{'statement': check, 'kind': 'judge', 'source': 'TEST review'}]
        helper.set_checks(session, records)
        new_block = helper.checks_block(records)
        draft.write_text(old_text + '\n```markdown\n' + new_block + '```\n')
        now = json.loads(command.ok('show', session, drafting=False))['now']
        assert now['stage'] == 'draft' and now['text'] == draft.read_text(), now
        assert now['signoff'] is None and now['needs_check'] is None, now
        draft.write_text(old_text.replace(old_block, new_block))
        now = json.loads(command.ok('show', session, drafting=False))['now']
        assert now['needs_check'] == check and '1 judged check' in now['signoff'], now
        assert not command.calls()


def publication(width, height):
    with tempfile.TemporaryDirectory(prefix='tinkery-TEST-contract-publication-') as directory:
        command = Command(directory, [], budget=0, agent=True)
        session = make(command.sessions, question=False)
        seed(session)
        draft = session / 'seed-contract.md'
        old_text = draft.read_text()
        old_block = helper.checks_block(helper.read_checks(session))
        check = 'TEST judge the second draft'
        next_checks = [{'statement': check, 'kind': 'judge', 'source': 'TEST review'}]
        ledger = session / 'ledger.json'
        first = helper.load(session)
        state = helper.editable(first)
        state['assumed'] = [{'text': 'TEST use the second draft', 'why': 'TEST default'}]
        second = helper.publish(session, state, first['version'], 'TEST new default')
        next_text = (old_text.replace('revision: r1', 'revision: r2').replace(old_block, helper.checks_block(next_checks))
                     .replace(helper.decisions_block(first), helper.decisions_block(second)))
        ledger.write_text(json.dumps(first))
        lock = session / '.writer.lock'
        terminal = Terminal(Path(directory).resolve(), width, height, budget=0, agent=True)
        try:
            terminal.wait('3  shape')
            terminal.open_saved_goal()
            with lock.open('rb') as writer:
                fcntl.flock(writer, fcntl.LOCK_EX | fcntl.LOCK_NB)
                draft.write_text(next_text)
                for args in [('show', session), ('status', '--json', session)]:
                    command.refused('The session is being updated.', *args, drafting=False)
                terminal.send(b's', 'The session is being updated.')
                assert 'Tink Contract r2' not in terminal.flat(), terminal.screen.text()
                (session / 'seed-checks.json').write_text(json.dumps(next_checks))
                ledger.write_text(json.dumps(second))
            shown = json.loads(command.ok('show', session, drafting=False))
            status = json.loads(command.ok('status', '--json', session, drafting=False))
            assert status['version'] == shown['version'] == second['version'], (status, shown)
            assert status['seed']['revision'] == 'r2' and status['seed']['sha256'] == shown['now']['sha256'], status
            assert [item['what'] for item in shown['decided_without_you']] == ['TEST use the second draft'], shown
            now = shown['now']
            assert now['revision'] == 'r2' and now['text'] == next_text and now['needs_check'] == check, now
            lock.unlink()  # Older sessions can have no writer lock.
            assert json.loads(command.ok('show', session, drafting=False)) == shown
            terminal.menu()
            terminal.open_shape()
            terminal.wait('Tink Contract r2')
            terminal.read_until(check)
            assert not lock.exists(), 'Reading a legacy session created a writer lock'
            assert not command.calls() and not terminal.calls()
        finally:
            terminal.close()


if __name__ == '__main__':
    fenced_example()
    for size in [(80, 24), (100, 30)]:
        publication(*size)
    print('Shape and seed show read published contracts at 80x24 and 100x30; zero provider calls')
