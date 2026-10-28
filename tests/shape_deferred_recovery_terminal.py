"""An explicit deferred-prerequisite answer: saved legacy fixtures, real PTYs, zero calls."""
from pathlib import Path
import tempfile
from shape_answer_terminal import Run, F1, F2, ESC, helper
from shape_terminal import make, node


def files(root):
    return {str(path): path.read_bytes() for path in root.rglob('*') if path.is_file()}


def journey(width, height, long=False, unavailable=None):
    with tempfile.TemporaryDirectory(prefix='tinkery-TEST-deferred-recovery-') as directory:
        root = Path(directory).resolve()
        goal = 'TEST exact goal café\nMust: Keep originals 中'
        session = make(root / 'TEST-sessions', 'legacy', goal=goal, question=False)
        old = helper.load(session)
        state = helper.editable(old)
        title = 'TEST A long named prerequisite café' if long else 'TEST A'
        question = 'TEST original question café?' + ('\n' + '\n'.join(f'TEST question detail {i}: keep all words.' for i in range(12)) + '\nTEST question tail' if long else '')
        reason = '  TEST reason café  ' + ('\n' + '\n'.join(f'TEST reason detail {i}' for i in range(8)) + '\nTEST reason tail' if long else '')
        condition = 'TEST condition 中' + ('\n' + '\n'.join(f'TEST revisit detail {i}' for i in range(8)) + '\nTEST revisit tail' if long else '')
        state['nodes'].extend([
            node('a', status='deferred', label=title, question=question,
                 defer_reason=reason, revisit_condition=condition),
            node('b', status='unresolved', label='TEST B', question='TEST dependent question?', prerequisites=['a']),
        ])
        helper.publish(session, state, old['version'], 'TEST legacy saved state')
        before = files(session)
        run = Run(root, width, height, budget=0, agent=unavailable != 'no model', shape_setup=unavailable is None)
        try:
            run.open_shape()
            run.wait('Deferred prerequisite: TEST A')
            if unavailable:
                assert 'F1 offers an explicit answer' not in run.flat()
                run.send(F1, 'F1 close')
                assert 'Answer deferred prerequisite' not in run.flat()
                assert files(session) == before and run.calls() == []
                print(f'Deferred recovery unavailable cue passed: {unavailable}, {width}x{height}')
                return
            run.choose('Answer deferred prerequisite', 'TEST original question café?')
            if long:
                run.read_until('TEST question tail')
                run.read_until('TEST reason tail')
                run.read_until('TEST revisit tail')
            else:
                run.wait('Deferred because: TEST reason café')
                run.wait('Revisit when: TEST condition 中')
            words = '  TEST exact answer café\n中  '
            run.send(b'\x1b[200~' + words.encode() + b'\x1b[201~', 'TEST exact answer café')
            assert files(session) == before, 'Opening or typing wrote recovery'
            run.send(ESC, 'Deferred prerequisite: TEST A')
            assert files(session) == before, 'Cancellation wrote recovery'
            run.choose('Answer deferred prerequisite', 'TEST original question café?')
            run.send(b'\x1b[200~' + words.encode() + b'\x1b[201~', 'TEST exact answer café')
            run.send(F2, 'Deferred prerequisite answered. No model ran.')
            run.wait('TEST dependent question?')
            saved = helper.load(session)
            a = next(n for n in saved['nodes'] if n['id'] == 'a')
            b = next(n for n in saved['nodes'] if n['id'] == 'b')
            assert saved['goal'] == goal and a['answer'] == words
            prior = a['history'][-1]['state']
            assert prior['question'] == question
            assert prior['defer_reason'] == reason
            assert prior['revisit_condition'] == condition and prior['status'] == 'deferred'
            assert b['status'] == 'unresolved' and b['prerequisites'] == ['a'] and not b.get('answer')
            assert a['authority'] == 'delegated', 'Agent action claimed operator authorship'
            assert run.calls() == [], 'Recovery called a provider'
            print(f'Deferred recovery passed at {width}x{height} (long={long}): cancel preserves files; exact answer/history; zero calls')
        finally:
            run.close()


if __name__ == '__main__':
    for size in [(80, 24), (100, 30)]:
        journey(*size)
    journey(80, 24, long=True)
    journey(80, 24, unavailable='no model')
    journey(80, 24, unavailable='held agent')
