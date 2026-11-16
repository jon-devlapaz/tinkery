"""Raw thoughts through real terminals and private TEST roots. All providers are local stubs."""
import json
import os
from pathlib import Path
import tempfile
import time

import test_launch as subprocess
from terminal_process import TerminalProcess, wait_output
import tend_connection_terminal as flow

REPO = Path(__file__).resolve().parents[1]
WORDS = '  TEST raw thought café 👩‍💻\r\nKeep\tthese words.\rLast line.  \n'


def launch(root, repo, size, provider=False, budget=8, home=True):
    args = [str(REPO/'target/debug/tinkery'), '--operator', 'agent',
            '--seed-session-root', str(root/'sessions'), '--record-root', str(root/'records'),
            '--link-root', str(root/'links')]
    if home:
        args += ['--home']
    if provider:
        args += ['--shape-pi', '--model', 'test/model', '--shape-budget', str(budget),
                 '--pi-command', str(root/'fake-pi')]
    term = TerminalProcess(args, cwd=repo, env=subprocess.environment({**os.environ, 'TERM': 'xterm-256color',
        'NO_COLOR': '1', 'TINKERY_NODE': '/tinkery-TEST-no-node'}), width=size[0], height=size[1])
    if home:
        screen = flow.wait(term, '1  think')
        assert 'Clarify a goal or save a thought' in screen, screen
        assert 'Your goals and thoughts' in screen, screen
        flow.send(term, b'2', 'this repo')
    else:
        flow.wait(term, "What's on your mind?")
    return term


def paste(term, words):
    os.write(term.master, b'\x1b[200~'+words.encode()+b'\x1b[201~')
    flow.wait(term, 'Last line.' if words == WORDS else words.splitlines()[0].strip())


def saved(root):
    return sorted((root/'links').glob('thought-*.json'))


def no_goal(root, original):
    assert {p: p.read_bytes() for p in (root/'sessions').rglob('*') if p.is_file()} == original, 'A thought changed goal sessions'
    assert len(list((root/'records').glob('*.json'))) == 2, 'A thought wrote a goal confirmation record'
    for path in saved(root):
        value = json.loads(path.read_bytes())
        assert set(value) == {'schema', 'repo', 'words'}, 'A thought acquired goal authority'
        assert path.stat().st_mode & 0o777 == 0o600


def journey(size):
    with tempfile.TemporaryDirectory(prefix='tinkery-TEST-thought-') as directory:
        root = Path(directory).resolve()
        repo = flow.fixture(root)
        original = {p: p.read_bytes() for p in (root/'sessions').rglob('*') if p.is_file()}
        term = launch(root, repo, size)
        try:
            screen = term.screen.text()
            assert 't think' in screen, screen
            assert '↑↓←→ move   enter open   f connections   l link   esc back' in screen, screen
            flow.send(term, b't', 'enter choose')
            screen = term.screen.text()
            assert 'tinkery · repo' in screen.splitlines()[1], screen
            assert 'enter choose' in screen.splitlines()[-1], screen
            assert 'F3' not in screen and 'no model call' not in screen, screen
            paste(term, WORDS)
            # Ctrl+J is a raw line feed: a new line, never a send. Backspace removes it again.
            flow.send(term, b'\n', 'enter choose')
            flow.send(term, b'\x7f', 'enter choose')
            assert not saved(root) and not flow.requests(root)
            # Ctrl+S is a raw DC3 byte in raw mode.
            flow.send(term, b'\x13', 'Thought saved.')
            screen = flow.send(term, b'v', 'Saved thoughts')
            assert 'g copies these words into Think, unsent.' in screen, screen
            assert 'Confirm a goal there' not in screen and 'No Shape.' not in screen, screen
            assert len(saved(root)) == 1
            assert json.loads(saved(root)[0].read_bytes())['words'].encode() == WORDS.encode(), 'Saved words differ from typed words'
            no_goal(root, original)
            flow.send(term, b's', 'Saved thoughts')
            assert 'tinkery › shape' not in term.screen.text()
            flow.send(term, b'\x1b', 'this repo')
            assert 'Thought: ' in term.screen.text()
        finally:
            term.close()
        term = launch(root, repo, size)
        try:
            flow.wait(term, 'this repo')
            flow.send(term, b'v', 'Thought 1 of 1')
            flow.send(term, b'\x1b', 'this repo')
            flow.send(term, b't', 'enter choose')
            paste(term, WORDS)
            flow.send(term, b'\x1bOR', 'this repo')
            flow.send(term, b'v', 'Saved thoughts')
            assert len(saved(root)) == 1
            flow.send(term, b'\x1b', 'this repo')
            flow.send(term, b't', 'enter choose')
            paste(term, 'TEST cancelled thought')
            flow.send(term, b'\x1b', 'Back to menu and lose')
            flow.send(term, b'y', '1  think')
            flow.send(term, b'2', 'this repo')
            assert len(saved(root)) == 1
            assert not flow.requests(root)
            no_goal(root, original)
        finally:
            term.close()
        other = root/'other'
        (other/'.git').mkdir(parents=True)
        term = launch(root, other, size)
        try:
            flow.send(term, b'v', 'No thoughts saved.')
            flow.send(term, b'\x1b', 'this repo')
            flow.send(term, b't', 'enter choose')
            paste(term, WORDS)
            flow.send(term, b'\x1bOR', 'Thought saved.')
            flow.send(term, b'v', 'Saved thoughts')
            assert len(saved(root)) == 2
            assert {json.loads(p.read_bytes())['repo'] for p in saved(root)} == {str(repo), str(other)}
        finally:
            term.close()
        term = launch(root, repo, size, provider=True)
        try:
            flow.send(term, b'f', 'F2 find one connection')
            flow.send(term, flow.F2, 'p prepare this goal')
            request = flow.requests(root)[0]['input']
            assert len(request['sources']) == 2, 'A thought reached the provider request'
            assert WORDS not in json.dumps(request, ensure_ascii=False)
            assert all('raw thought' not in source['goal'] for source in request['sources'])
            no_goal(root, original)
        finally:
            term.close()


def save_feedback(size, unreadable_records=False, evidence=None):
    """Saving selects the exact thought, with honest feedback and usable goals."""
    with tempfile.TemporaryDirectory(prefix='tinkery-TEST-thought-feedback-') as directory:
        root = Path(directory).resolve()
        repo = flow.fixture(root)
        for index in range(2):
            flow.update_goal(root, f'goal-{index}', goal=f'TEST goal {index}')
        original = {p: p.read_bytes() for p in (root/'sessions').rglob('*') if p.is_file()}
        term = launch(root, repo, size, provider=True, budget=0)
        try:
            for words in ['A TEST first thought', 'B TEST second thought']:
                flow.send(term, b't', 'enter choose')
                paste(term, words)
                flow.send(term, b'\x1bOR', 'Thought saved.')
            if unreadable_records:
                path = saved(root)[0]
                prefix = path.name.rsplit('-', 1)[0]
                (path.parent/f'{prefix}-malformed.json').write_text('{')
            for words, outcome, position in [
                ('Z TEST newly saved thought', 'Thought saved.', 3),
                ('B TEST second thought', 'Thought already saved.', 2),
            ]:
                before = {p: p.read_bytes() for p in saved(root)}
                flow.send(term, b't', 'enter choose')
                paste(term, words)
                screen = flow.send(term, b'\x1bOR', outcome)
                if evidence is not None:
                    case = 'new' if position == 3 else 'duplicate'
                    warning = '-warning' if unreadable_records else ''
                    (evidence/f'{size[0]}x{size[1]}-{case}{warning}.txt').write_text(screen)
                assert 'tinkery › tend' in screen, screen
                assert 'Thoughts:' not in screen, screen
                assert 'Thought:' in screen and outcome in screen.splitlines()[-3], screen
                assert 'TEST goal 0' in screen and 'TEST goal 1' in screen, screen
                if unreadable_records:
                    assert '1 thoughts could not be read.' in screen, screen
                if position == 2:
                    assert {p: p.read_bytes() for p in saved(root)} == before, 'Duplicate changed thought records'
                else:
                    assert len(saved(root)) == len(before) + 1
                # The label hit must select the displayed goal after the summary is removed.
                lines = screen.splitlines()
                y = next(y for y, line in enumerate(lines[:-4]) if 'TEST goal 1' in line)
                x = lines[y].index('TEST goal 1') + 4
                click = (f'\x1b[<0;{x+1};{y+1}M\x1b[<0;{x+1};{y+1}m').encode()
                screen = flow.send(term, click, '› TEST goal 1')
                assert outcome not in screen, screen
                screen = flow.send(term, b'v', f'Thought {position} of 3')
                assert words in screen and outcome not in screen, screen
                if unreadable_records:
                    assert '1 thoughts could not be read.' in screen, screen
                screen = flow.send(term, b'\x1b', 'this repo')
                assert outcome not in screen, screen
                assert not flow.requests(root)
            # A new save notice also clears when Tend closes, without opening the reader.
            flow.send(term, b't', 'enter choose')
            paste(term, 'Z TEST newly saved thought')
            flow.send(term, b'\x1bOR', 'Thought already saved.')
            flow.send(term, b'\x1b', '1  think')
            screen = flow.send(term, b'2', 'this repo')
            assert 'Thought already saved.' not in screen, screen
            assert not flow.requests(root)
            if unreadable_records:
                (path.parent/f'{prefix}-malformed.json').unlink()
            no_goal(root, original)
        finally:
            term.close()


def unwritable(size):
    with tempfile.TemporaryDirectory(prefix='tinkery-TEST-thought-refusal-') as directory:
        root = Path(directory).resolve()
        repo = flow.fixture(root)
        (root/'links').write_text('TEST not a directory')
        term = launch(root, repo, size)
        try:
            flow.send(term, b't', 'enter choose')
            paste(term, 'TEST keep unsaved words')
            flow.send(term, b'\x1bOR', 'File exists')
            assert 'TEST keep unsaved words' in term.screen.text()
            assert (root/'links').read_text() == 'TEST not a directory'
            assert not flow.requests(root)
            flow.send(term, b'\x1b', 'Back to menu and lose')
        finally:
            term.close()


def unreadable(size):
    with tempfile.TemporaryDirectory(prefix='tinkery-TEST-thought-malformed-') as directory:
        root = Path(directory).resolve()
        repo = flow.fixture(root)
        term = launch(root, repo, size)
        try:
            flow.send(term, b't', 'enter choose')
            paste(term, 'TEST readable thought')
            flow.send(term, b'\x1bOR', 'Thought saved.')
            flow.send(term, b'v', 'Saved thoughts')
        finally:
            term.close()
        path = saved(root)[0]
        prefix = path.name.rsplit('-', 1)[0]
        (path.parent/f'{prefix}-malformed.json').write_text('{')
        foreign = json.loads(path.read_bytes())
        foreign['schema'] = 99
        (path.parent/f'{prefix}-foreign.json').write_text(json.dumps(foreign))
        term = launch(root, repo, size)
        try:
            flow.wait(term, 'this repo')
            flow.wait(term, '2 thoughts could not be read.')
            flow.send(term, b'v', 'TEST readable thought')
            assert 'Thought 1 of 1' in term.screen.text()
            assert '2 thoughts could not be read.' in term.screen.text()
            assert not flow.requests(root)
        finally:
            term.close()


THINK_HOST = '''#!/usr/bin/env python3
import json,sys
from pathlib import Path
with Path('provider-calls').open('a') as log: log.write('call\\n')
r=json.load(sys.stdin)
if r.get('kind')=='meaning-preservation': print(json.dumps(dict(missing=[],false_choice=False)));sys.exit(0)
if r.get('kind')=='question-continuity':print('{"repeated":[]}');sys.exit(0)
print(json.dumps(dict(parts=dict(who='I',outcome='keep project thoughts',why='I need exact originals',done_when='thoughts remain local'),open=['why'],supports=[dict(source=s['id'],quote=s['text'],occurrence=0) for s in r['sources']],misfits=[],questions=[dict(target='why',id='open',text='Which UI must keep these originals?')],alternatives=[])))
'''


def save_after_send(size):
    with tempfile.TemporaryDirectory(prefix='tinkery-TEST-save-after-send-') as directory:
        root = Path(directory).resolve()
        repo = flow.fixture(root)
        (root/'fake-pi').write_text(THINK_HOST)
        original = {p: p.read_bytes() for p in (root/'sessions').rglob('*') if p.is_file()}
        term = launch(root, repo, size, provider=True)
        try:
            flow.send(term, b't', 'enter choose')
            paste(term, WORDS)
            flow.send(term, flow.F2, '1  Clarify into a goal')
            flow.send(term, b'1', 'Which UI')
            second = '  TEST second original café\r\nKeep\tthis too.  '
            flow.send(term, b'\x1bOP', 'Add a dump to this conversation')
            flow.send(term, term.screen.pick('Add a dump'), 'add more')
            paste(term, second)
            flow.send(term, flow.F2, 'Should these added words')
            flow.send(term, b'\x1b[5;5~', 'TEST raw thought')
            pending = 'TEST unsent answer stays here'
            paste(term, pending)
            before = term.screen.text()
            calls = (repo/'provider-calls').read_bytes()

            def save(expected, duplicate=False):
                flow.send(term, b'\x1bOP', 'Right now')
                label = 'Save my words as a thought'
                for _ in range(3):
                    try:
                        digit = term.screen.pick(label)
                        break
                    except AssertionError:
                        flow.send(term, b'\x1b[C', 'Right now')
                else:
                    raise AssertionError(term.screen.text())
                screen = flow.send(term, digit, 'Thought already saved.' if duplicate else 'Thought saved.')
                assert pending in screen, screen
                assert 'Should these added words' in screen, screen
                assert (repo/'provider-calls').read_bytes() == calls
                assert {json.loads(p.read_bytes())['words'] for p in saved(root)} == expected
                no_goal(root, original)
                return screen

            after = save({WORDS})
            assert before.splitlines()[3:15] == after.splitlines()[3:15], (before, after)
            records = {p: p.read_bytes() for p in saved(root)}
            save({WORDS}, duplicate=True)
            assert {p: p.read_bytes() for p in saved(root)} == records
            flow.send(term, b'\x1b[6;5~', 'TEST second original')
            save({WORDS, second})
            flow.send(term, b'\x1b[5;5~', 'TEST raw thought')
            save({WORDS, second}, duplicate=True)
            os.write(term.master, b'\x01\x03')
            for _ in wait_output(term.master, term.process, term.screen.feed, time.monotonic()+15):
                if term.screen.clipboards and not term.screen.synchronized:
                    break
            assert term.screen.clipboards[-1] == pending
        finally:
            term.close()


def promotion(size):
    with tempfile.TemporaryDirectory(prefix='tinkery-TEST-thought-goal-') as directory:
        root = Path(directory).resolve()
        repo = flow.fixture(root)
        (root/'fake-pi').write_text(THINK_HOST)
        term = launch(root, repo, size, provider=True)
        try:
            flow.send(term, b't', 'enter choose')
            paste(term, WORDS)
            flow.send(term, b'\x1bOR', 'Thought saved.')
            flow.send(term, b'v', 'Saved thoughts')
            thought = saved(root)[0].read_bytes()
            flow.send(term, b'g', "What's on your mind?")
            flow.wait(term, 'TEST raw thought')
            assert len(list((root/'sessions').iterdir())) == 2
            assert not (repo/'provider-calls').exists()
            flow.send(term, flow.F2, '1  Clarify into a goal')
            flow.send(term, b'1', 'Which UI')
            flow.send(term, b'\x1bOP', 'Review the goal')
            flow.send(term, term.screen.pick('Review the goal'), 'You are confirming')
            os.write(term.master, b'\x1b[6~'*30)
            flow.wait(term, 'confirm')
            flow.send(term, b'confirm', 'confirm')
            before = {p: p.read_bytes() for folder in ['sessions', 'records']
                      for p in (root/folder).rglob('*') if p.is_file()}
            calls = (repo/'provider-calls').read_bytes()
            for chord in [b'\x1b[13;2u', b'\x1b[13;3u', b'\x1b\r', b'\x1b[13;5u',
                          b'\x1b[106;5u', b'\n', b'\x1b[13;1:2u']:
                screen = flow.send(term, chord, 'Press plain Enter to confirm.')
                assert 'Saved.' not in screen, screen
                assert {p: p.read_bytes() for folder in ['sessions', 'records']
                        for p in (root/folder).rglob('*') if p.is_file()} == before
                assert (repo/'provider-calls').read_bytes() == calls
                # Clear the refusal before the next key, so an old frame cannot pass it.
                os.write(term.master, b'\x7fm')
                for code in wait_output(term.master, term.process, term.screen.feed, time.monotonic()+15):
                    if not term.screen.synchronized and 'Press plain Enter' not in term.screen.text():
                        break
                    if code is not None:
                        raise AssertionError(term.screen.text())
                else:
                    raise AssertionError('Confirmation refusal did not clear')
            flow.send(term, b'\r', 'Saved.')
            records = [json.loads(p.read_bytes()) for p in (root/'records').glob('*.json')]
            record = next(r for r in records if r['origin'] == 'live')
            assert record['turns'][0]['text'].encode() == WORDS.encode()
            assert record['repo'] == str(repo)
            assert len(list((root/'sessions').iterdir())) == 3
            assert saved(root)[0].read_bytes() == thought
        finally:
            term.close()


def repository_link(size):
    with tempfile.TemporaryDirectory(prefix='tinkery-TEST-thought-link-') as directory:
        root = Path(directory).resolve()
        repo = flow.fixture(root)
        for index in range(2):
            flow.update_goal(root, f'goal-{index}', goal=f'TEST goal {index}')
        term = launch(root, repo, size)
        try:
            flow.send(term, b'\x1bOP', 'Right now')
            help_text = term.screen.text()
            help_text += flow.send(term, b'\x1b[6~', 'F1 close')
            assert 't think · v read thoughts' in help_text, help_text
            assert 'g in the thought reader opens Think, unsent.' in help_text, help_text
            flow.send(term, b'\x1b', 'this repo')
            flow.send(term, b'l', 'enter link')
            lines = term.screen.text().splitlines()
            ring = [(x, y) for y, line in enumerate(lines[4:-4], 4)
                    for x, char in enumerate(line) if x > size[0] // 2 and '\u2801' <= char <= '\u28ff']
            assert ring, term.screen.text()
            x = round((min(x for x, y in ring) + max(x for x, y in ring)) / 2)
            y = round((min(y for x, y in ring) + max(y for x, y in ring)) / 2)
            assert lines[y+3][x] == ' ', term.screen.text()
            def click(row):
                return (f'\x1b[<0;{x+1};{row+1}M\x1b[<0;{x+1};{row+1}m').encode()
            os.write(term.master, click(y+3))
            flow.wait(term, 'esc')
            assert not (root/'links'/'needs.json').exists(), 'Empty-space click wrote a needs link'
            assert 'enter link' in term.screen.text(), term.screen.text()
            flow.send(term, click(y), 'needs TEST goal 1 · not ready')
            assert json.loads((root/'links'/'needs.json').read_text()) == {
                'version': 1, 'needs': [{'goal': 'goal-0', 'needs': 'goal-1'}]}
            assert not flow.requests(root)
        finally:
            term.close()


def capture_guards(size):
    for budget in (0, 8):
        with tempfile.TemporaryDirectory(prefix='tinkery-TEST-capture-') as directory:
            root = Path(directory).resolve()
            repo = flow.fixture(root)
            original = {p: p.read_bytes() for p in (root/'sessions').rglob('*') if p.is_file()}
            term = launch(root, repo, size, provider=True, budget=budget)
            try:
                flow.send(term, b't', 'enter choose')
                flow.send(term, b'\x1bOR', 'Write a thought of 1 to 131072 bytes')
                assert not saved(root)
                paste(term, WORDS)
                flow.send(term, b'\x1bOP', 'Right now')
                os.write(term.master, b'\x1bOR')
                flow.wait(term, 'Right now')
                assert not saved(root)
                flow.send(term, b'\x1b', 'enter choose')
                flow.send(term, b'\x1bOR', 'this repo')
                flow.send(term, b'v', 'Saved thoughts')
                flow.send(term, b'g', "What's on your mind?")
                flow.send(term, b'\x1bOR', 'this repo')
                assert len(saved(root)) == 1
                flow.send(term, b'v', 'Saved thoughts')
                flow.send(term, b'g', "What's on your mind?")
                paste(term, 'TEST appended copy')
                flow.send(term, b'\x1bOR', 'Thought saved.')
                assert {json.loads(p.read_bytes())['words'] for p in saved(root)} == {WORDS, WORDS+'TEST appended copy'}
                flow.send(term, b't', 'enter choose')
                paste(term, 'TEST cancelled fresh thought')
                flow.send(term, b'\x1b', 'Back to menu and lose')
                flow.send(term, b'y', '1  think')
                assert len(saved(root)) == 2
                assert not flow.requests(root)
                no_goal(root, original)
            finally:
                term.close()
            term = launch(root, repo, size, provider=True, budget=budget, home=False)
            try:
                flow.wait(term, "What's on your mind?")
                assert 'keep thought' not in term.screen.text(), term.screen.text()
                paste(term, 'TEST standalone unsaved')
                flow.send(term, b'\x13', 'Open --home to save a thought in Tend.')
                assert 'TEST standalone unsaved' in term.screen.text()
                assert len(saved(root)) == 2
                assert not flow.requests(root)
            finally:
                term.close()


def fresh_choice(size):
    with tempfile.TemporaryDirectory(prefix='tinkery-TEST-fresh-choice-') as directory:
        root = Path(directory).resolve()
        repo = flow.fixture(root)
        term = launch(root, repo, size, provider=True)
        original = {p: p.read_bytes() for p in (root/'sessions').rglob('*') if p.is_file()}
        try:
            flow.send(term, b't', 'enter choose')
            paste(term, WORDS)
            screen = flow.send(term, b'\r', '1  Clarify into a goal')
            assert 'a model reads your words' in screen and 'nothing reads them' in screen, screen
            for keys in [b'\r\r', b'\x1b[13;1:2u', b'\x1b[49;1:2u',
                         b'\x1b[200~1\n2\r\n\x1b[201~', flow.F2, b'x']:
                flow.send(term, keys, '1  Clarify into a goal')
                assert not flow.requests(root) and not saved(root)
            flow.send(term, b'\x1b', 'enter choose')
            os.write(term.master, b'\x01\x03')
            for _ in wait_output(term.master, term.process, term.screen.feed, time.monotonic()+15):
                if term.screen.clipboards and not term.screen.synchronized:
                    break
            assert term.screen.clipboards[-1] == WORDS
            assert not flow.requests(root)
            flow.send(term, b'\r', '1  Clarify into a goal')
            flow.send(term, b'2', 'Thought saved.')
            assert not flow.requests(root)
            assert len(saved(root)) == 1
            assert json.loads(saved(root)[0].read_text())['words'] == WORDS
            no_goal(root, original)
        finally:
            term.close()
        term = launch(root, repo, size, provider=True, home=False)
        try:
            paste(term, 'TEST standalone choice')
            screen = flow.send(term, b'\r', 'Unavailable: Open --home')
            assert '2  Save as a thought' in screen, screen
            before = {p: p.read_bytes() for p in saved(root)}
            flow.send(term, b'2', 'Open --home to save a thought in Tend.')
            assert {p: p.read_bytes() for p in saved(root)} == before
            assert not flow.requests(root)
        finally:
            term.close()


if __name__ == '__main__':
    for size in [(80, 24), (100, 30)]:
        save_after_send(size)
        fresh_choice(size)
        save_feedback(size)
        save_feedback(size, unreadable_records=True)
        capture_guards(size)
        repository_link(size)
        journey(size)
        unwritable(size)
        promotion(size)
        unreadable(size)
    print('Thought journeys passed at 80x24 and 100x30 without colour. Local stubs only.')
