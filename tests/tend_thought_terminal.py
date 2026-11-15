"""Raw thoughts through real terminals and private TEST roots. All providers are local stubs."""
import json
import os
from pathlib import Path
import tempfile

import test_launch as subprocess
from terminal_process import TerminalProcess
import tend_connection_terminal as flow

REPO = Path(__file__).resolve().parents[1]
WORDS = '  TEST raw thought café 👩‍💻\r\nKeep\tthese words.\rLast line.  \n'


def launch(root, repo, size, provider=False):
    args = [str(REPO/'target/debug/tinkery'), '--operator', 'agent', '--home',
            '--seed-session-root', str(root/'sessions'), '--record-root', str(root/'records'),
            '--link-root', str(root/'links')]
    if provider:
        args += ['--shape-pi', '--model', 'test/model', '--shape-budget', '8',
                 '--pi-command', str(root/'fake-pi')]
    term = TerminalProcess(args, cwd=repo, env=subprocess.environment({**os.environ, 'TERM': 'xterm-256color',
        'NO_COLOR': '1', 'TINKERY_NODE': '/tinkery-TEST-no-node'}), width=size[0], height=size[1])
    flow.wait(term, '1  think')
    flow.send(term, b'2', 'this repo')
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
            flow.send(term, b't', 'F2 save thought only')
            screen = term.screen.text()
            assert str(repo) in ''.join(screen.split()) and 'No model call.' in screen, screen
            paste(term, WORDS)
            assert not saved(root)
            flow.send(term, flow.F2, 'Thought saved. No goal confirmed.')
            assert len(saved(root)) == 1
            assert json.loads(saved(root)[0].read_bytes())['words'].encode() == WORDS.encode(), 'Saved words differ from typed words'
            no_goal(root, original)
            flow.send(term, b's', 'Saved thoughts')
            assert 'tinkery › shape' not in term.screen.text()
            flow.send(term, b'\x1b', 'Thoughts: 1')
            assert 'Thought: ' in term.screen.text()
        finally:
            term.close()
        term = launch(root, repo, size)
        try:
            flow.wait(term, 'Thoughts: 1')
            flow.send(term, b'v', 'Thought 1 of 1')
            flow.send(term, b'\x1b', 'Thoughts: 1')
            flow.send(term, b't', 'F2 save thought only')
            paste(term, WORDS)
            flow.send(term, flow.F2, 'Thought already saved. One entry.')
            assert len(saved(root)) == 1
            flow.send(term, b'\x1b', 'Thoughts: 1')
            flow.send(term, b't', 'F2 save thought only')
            paste(term, 'TEST cancelled thought')
            flow.send(term, b'\x1b', 'Thoughts: 1')
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
            flow.send(term, b't', 'F2 save thought only')
            paste(term, WORDS)
            flow.send(term, flow.F2, 'Thought saved. No goal confirmed.')
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


def unwritable(size):
    with tempfile.TemporaryDirectory(prefix='tinkery-TEST-thought-refusal-') as directory:
        root = Path(directory).resolve()
        repo = flow.fixture(root)
        (root/'links').write_text('TEST not a directory')
        term = launch(root, repo, size)
        try:
            flow.send(term, b't', 'F2 save thought only')
            paste(term, 'TEST keep unsaved words')
            flow.send(term, flow.F2, 'Thought not saved:')
            assert 'TEST keep unsaved words' in term.screen.text()
            assert (root/'links').read_text() == 'TEST not a directory'
            assert not flow.requests(root)
            flow.send(term, b'\x1b', 'this repo')
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


def promotion(size):
    with tempfile.TemporaryDirectory(prefix='tinkery-TEST-thought-goal-') as directory:
        root = Path(directory).resolve()
        repo = flow.fixture(root)
        (root/'fake-pi').write_text(THINK_HOST)
        term = launch(root, repo, size, provider=True)
        try:
            flow.send(term, b't', 'F2 save thought only')
            paste(term, WORDS)
            flow.send(term, flow.F2, 'Thought saved. No goal confirmed.')
            thought = saved(root)[0].read_bytes()
            flow.send(term, b'g', "What's on your mind?")
            flow.wait(term, 'TEST raw thought')
            assert len(list((root/'sessions').iterdir())) == 2
            assert not (repo/'provider-calls').exists()
            flow.send(term, flow.F2, 'Which UI')
            flow.send(term, b'\x1bOP', 'Review the goal')
            flow.send(term, term.screen.pick('Review the goal'), 'You are confirming')
            os.write(term.master, b'\x1b[6~'*30)
            flow.wait(term, 'confirm')
            flow.send(term, b'confirm\r', 'Saved.')
            records = [json.loads(p.read_bytes()) for p in (root/'records').glob('*.json')]
            record = next(r for r in records if r['origin'] == 'live')
            assert record['turns'][0]['text'].encode() == WORDS.encode()
            assert record['repo'] == str(repo)
            assert len(list((root/'sessions').iterdir())) == 3
            assert saved(root)[0].read_bytes() == thought
        finally:
            term.close()


if __name__ == '__main__':
    for size in [(80, 24), (100, 30)]:
        journey(size)
        unwritable(size)
        promotion(size)
    print('Thought journeys passed at 80x24 and 100x30 without colour. Local stubs only.')
