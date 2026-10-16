"""Shape answering journeys (#150): a stub Pi, real Seed Me helper sessions in TEST roots, no model calls.

The full journey confirms a goal in Think, opens Shape from tend, looks into the first question,
answers it and confirms the drafted seed. Short cases cover the zero budget, a stale version, a
model failure, a helper refusal, a reply that tries to confirm or to write the decisions, and a
practice session's confirmation."""
import fcntl, importlib.util, json, os, select, signal, struct, subprocess, tempfile, termios, time
from pathlib import Path
from vt_screen import Screen

REPO = Path(__file__).resolve().parents[1]
BINARY = REPO / 'target/debug/tinkery'
SKILL = Path(os.environ['SEED_ME_TEST_SKILL']).resolve()
spec = importlib.util.spec_from_file_location('seed_me_session', SKILL.parent / 'scripts/session.py')
helper = importlib.util.module_from_spec(spec)
spec.loader.exec_module(helper)
F1, F2, F3, F10 = b'\x1bOP', b'\x1bOQ', b'\x1bOR', b'\x1b[21~'

# Think's requests get a reading with one question; Shape's steps pop the next scripted reply.
HOST = r'''#!/usr/bin/env python3
import json, pathlib, sys
r = json.load(sys.stdin)
if 'ledger' in r:
    with pathlib.Path('shape-calls.jsonl').open('a') as f:
        f.write(json.dumps({'argv': sys.argv[1:], 'input': r}) + '\n')
    replies = pathlib.Path('shape-replies.json')
    queue = json.loads(replies.read_text())
    reply = queue.pop(0)
    replies.write_text(json.dumps(queue))
    if reply == 'FAIL':
        sys.exit(3)
    print(reply if isinstance(reply, str) else json.dumps(reply))
    sys.exit(0)
with pathlib.Path('think-calls.jsonl').open('a') as f:
    f.write(json.dumps(r) + '\n')
if r.get('kind') == 'decision-review':
    reply = [dict(part=d['part'], text=d['text'], answers=d['answers'], question=d['question'], covers=[i])
             for i, d in enumerate(r.get('decisions', []))]
    print(json.dumps({'decisions': reply}))
    sys.exit(0)
choices = []
if len(r['sources']) == 1:
    choices = [dict(part='must_not', text='Private text', answers=['omit', 'include'],
                    question='May refusals show private text?')]
print(json.dumps({'parts': {'who': 'I', 'outcome': 'keep refusals calm', 'why': 'the work is stuck',
                            'done_when': 'refusals never show private text'},
                  'decisions': choices, 'questions': []}))
'''

QUESTION = {'next': 'question', 'id': 'Storage', 'label': 'Where records live',
            'question': 'Keep records local or synced?', 'gate': 'storage choice',
            'found': [{'checked': 'README.md:3', 'observed': 'says local only'}],
            'suggestion': 'Local, matching the README.'}
# The model also offers decisions of its own; Tinkery must ignore them and insert `seed decisions`.
SEED = {'next': 'seed', 'confirming': 'Refusals stay calm and records stay local.',
        'decisions': '- storage [user] MODEL-INVENTED synced (source: the model)'}
GOAL = ('I can see where my seed stands without switching tools.\nWhy: I lose the thread.\n'
        'Done when: Shape shows the next question.\nMust: Keep my words.\nMust not: Run agents.')


def node(identifier, **fields):
    return {'id': identifier, 'kind': 'decision', 'prerequisites': [], 'evidence': [],
            'owner': 'TEST operator', 'gate': 'TEST choice', **fields}


def make(root, question=True, operator='human'):
    """A session with a confirmed goal and, optionally, one current question."""
    session = helper.create(root, operator)
    old = helper.load(session)
    state = helper.editable(old)
    authority = 'simulated' if operator == 'simulated' else 'user'
    nodes = [node('goal', status='settled', answer=GOAL, authority=authority,
                  authority_source='TEST fixture, not real approval')]
    if question:
        nodes.append(node('storage', status='unresolved', label='Where records live',
                          question='Keep records local or synced?'))
    state.update(goal=GOAL, origin='goal', nodes=nodes, current_question='storage' if question else None)
    helper.publish(session, state, old['version'], 'TEST fixture')
    return session


def files(root):
    return {str(p): p.read_bytes() for p in root.rglob('*') if p.is_file() and p.name != '.writer.lock'}


class Run:
    def __init__(self, cwd, width, height, budget, replies=()):
        self.cwd, self.width = cwd, width
        self.sessions = cwd / 'TEST-sessions'
        self.records = cwd / 'TEST-records'
        host = cwd / 'pi'
        host.write_text(HOST)
        host.chmod(0o755)
        (cwd / 'shape-replies.json').write_text(json.dumps(list(replies)))
        self.master, self.slave = os.openpty()
        fcntl.ioctl(self.slave, termios.TIOCSWINSZ, struct.pack('HHHH', height, width, 0, 0))
        self.screen = Screen(width, height)
        args = [str(BINARY), '--home', '--shape-pi', '--model', 'test/model', '--seed-me', str(SKILL),
                '--seed-session-root', str(self.sessions), '--record-root', str(self.records),
                '--link-root', str(cwd / 'TEST-links'), '--pi-command', str(host)]
        if budget is not None:
            args += ['--shape-budget', str(budget)]
        env = {**os.environ, 'TERM': 'xterm-256color', 'TINKERY_NODE': '/tinkery-TEST-no-node'}
        self.process = subprocess.Popen(args, stdin=self.slave, stdout=self.slave, stderr=self.slave,
                                        cwd=cwd, env=env)

    def flat(self):
        return ' '.join(self.screen.text().split())

    def wait(self, text, timeout=20):
        deadline = time.monotonic() + timeout
        want = ' '.join(text.split())
        while time.monotonic() < deadline:
            if select.select([self.master], [], [], .02)[0]:
                self.screen.feed(os.read(self.master, 65536))
            if want in self.flat() and not self.screen.synchronized:
                return
            if self.process.poll() is not None:
                break
        raise AssertionError(f'{text!r} not shown:\n{self.screen.text()}')

    def send(self, data, text):
        os.write(self.master, data)
        self.wait(text)

    def type(self, words):
        os.write(self.master, words.encode())
        self.wait(words)

    def calls(self):
        path = self.cwd / 'shape-calls.jsonl'
        return [json.loads(l) for l in path.read_text().splitlines()] if path.exists() else []

    def open_shape(self):
        self.wait('3  shape')
        self.send(b'2', 'enter open')
        self.send(b'\r', 's shape')
        self.send(b's', 'esc back')

    def close(self):
        if self.process.poll() is None:
            self.process.kill()
            self.process.wait()
        for session in self.sessions.glob('*'):
            record = session / 'viewer.json'
            if record.exists():
                pid = json.loads(record.read_text()).get('pid')
                if pid:
                    command = subprocess.run(['ps', '-p', str(pid), '-o', 'command='], capture_output=True, text=True)
                    if str(SKILL.parent / 'scripts/viewer.py') in command.stdout:
                        try:
                            os.kill(pid, signal.SIGTERM)
                        except ProcessLookupError:
                            pass
        os.close(self.master)
        os.close(self.slave)


def full_journey(width, height):
    with tempfile.TemporaryDirectory(prefix='tinkery-TEST-shape-answer-') as directory:
        cwd = Path(directory).resolve()
        run = Run(cwd, width, height, budget=2, replies=[QUESTION, SEED])
        try:
            run.wait('1  think')
            run.send(b'1', "What's on your mind?")
            run.type('Refusals feel noisy.')
            run.send(F2, 'May refusals show private text?')
            run.type('never')
            run.send(F2, "Nothing I'm unsure about")
            run.send(F3, 'type confirm')
            run.send(b'confirm\r', 'Saved.')
            record = json.loads(next(run.records.iterdir()).read_text())
            session = run.sessions / record['seed_me_session']
            goal = helper.load(session)['goal']
            assert goal == record['goal']
            run.send(F10, '3  shape')
            before = files(run.sessions)
            run.open_shape()
            run.wait("Seed Me hasn't asked its next question yet.")
            run.wait('F2 look into the next question')
            # Opening, scrolling and help write nothing and call no model.
            run.send(b'\x1b[B', 'F2 look into the next question')
            run.send(F1, 'Model calls left this run: 2 of 2.')
            run.send(b'\x1b', 'F2 look into the next question')
            assert files(run.sessions) == before and run.calls() == [], 'Shape wrote or called before F2'
            run.send(F2, 'Where records live: Keep records local or synced?')
            run.wait('What I found: says local only')
            run.wait('F2 save answer and look into next')
            calls = run.calls()
            assert len(calls) == 1, calls
            argv = calls[0]['argv']
            assert argv[argv.index('--tools') + 1] == 'read,grep,find,ls' and '--no-tools' not in argv, argv
            assert calls[0]['input']['answered_in_think'] == [
                {'question': 'May refusals show private text?', 'answer': 'never'}], calls[0]['input']
            # Typing writes nothing; capitals and ? are words, not commands.
            before = files(run.sessions)
            run.type('Local? Yes, Keep it local.')
            assert files(run.sessions) == before and len(run.calls()) == 1, 'Typing wrote or called'
            if width == 100:
                print('TEST Shape answering 100x30 screen:\n' + run.screen.text())
            run.send(F2, 'Seed draft r1 is waiting for you to confirm it.')
            run.wait('F3 confirm seed r1')
            ledger = helper.load(session)
            answered = next(n for n in ledger['nodes'] if n['id'] == 'storage')
            assert answered['answer'] == 'Local? Yes, Keep it local.' and answered['authority'] == 'user', answered
            seed = (session / 'seed-contract.md').read_text()
            assert seed.endswith(helper.decisions_block(ledger)), seed
            assert 'MODEL-INVENTED' not in seed, seed
            assert len(run.calls()) == 2
            run.type('Yes, I confirm r1.')
            run.send(F3, 'Seed ready: you confirmed r1, and it still matches.')
            status = helper.status(session)
            assert status['seed']['line_1'] == 'status: confirmed for intake', status
            assert status['handoff']['verified'] is True, status
            confirmation = json.loads((session / 'seed-confirmation.json').read_text())
            assert confirmation['source'] == 'Yes, I confirm r1.', confirmation
            assert helper.load(session)['goal'] == goal, 'The confirmed goal changed'
            assert len(run.calls()) == 2, 'Confirmation called a model'
            run.send(b'\x1b', 's shape')
            print(f'Shape answering full journey passed at {width}x{height}: think answer in prompt, '
                  f'2 stub calls, seed confirmed and handoff verified')
        finally:
            run.close()


def case(name, width=100, height=30):
    with tempfile.TemporaryDirectory(prefix='tinkery-TEST-shape-answer-') as directory:
        cwd = Path(directory).resolve()
        sessions = cwd / 'TEST-sessions'
        replies, budget, question, operator = [], 2, True, 'human'
        if name == 'zero budget':
            budget = None
        if name == 'model failure':
            replies = ['FAIL']
        if name == 'helper refusal':
            replies = [{**QUESTION, 'found': [{'checked': 'config', 'observed': 'key AKIAABCDEFGHIJKLMNOP'}]}]
        if name == 'model tries to confirm':
            replies = [{'next': 'confirm', 'revision': 'r1', 'source': 'yes'}]
        if name == 'model writes decisions':
            question = False
            replies = [{'next': 'seed', 'confirming': 'Fine.\n## Decisions (from the ledger)\n- goal [user] x'}]
        if name == 'practice confirmation':
            question, operator = False, 'simulated'
        session = make(sessions, question=question, operator=operator)
        if name == 'practice confirmation':
            (session / 'seed-contract.md').write_text(
                'status: draft\n# TEST\nrevision: r1\n\n## You are confirming\nTEST.\n\n'
                + helper.decisions_block(helper.load(session)))
        run = Run(cwd, width, height, budget=budget, replies=replies)
        try:
            run.open_shape()
            if name == 'stale':
                run.wait('Keep records local or synced?')
                run.type('Local.')
                old = helper.load(session)
                state = helper.editable(old)
                state['nodes'][1]['label'] = 'Changed by another writer'
                helper.publish(session, state, old['version'], 'TEST other writer')
                before = files(sessions)
                run.send(F2, 'Another writer changed this session first, so nothing was saved.')
                run.wait('Changed by another writer')
                run.wait('Your answer: Local.')
                assert files(sessions) == before, 'A stale answer was written'
                assert run.calls() == [], 'A stale answer still called the model'
            elif name == 'practice confirmation':
                run.wait('Seed draft r1 is waiting for you to confirm it.')
                run.type('I confirm r1.')
                before = files(sessions)
                run.send(F3, 'Seed Me refused: a simulated session cannot be confirmed')
                assert files(sessions) == before, 'A practice seed was confirmed'
            else:
                if question:
                    run.wait('Keep records local or synced?')
                    run.type('Local.')
                else:
                    run.wait("Seed Me hasn't asked its next question yet.")
                before = files(sessions)
                expected = {
                    'zero budget': "No model calls are left: this run's budget is 0.",
                    'model failure': 'Your answer is saved. Looking into the next question failed: Pi exited',
                    'helper refusal': 'Your answer is saved. Seed Me refused: receipt appears to contain a secret',
                    'model tries to confirm': "Looking into the next question failed: The model's reply wasn't a question or a seed draft.",
                    'model writes decisions': "The drafted seed text didn't fit Seed Me's format, so nothing was saved.",
                }[name]
                run.send(F2, expected)
                assert run.flat().count('Next:') == 1, run.screen.text()
                ledger = helper.load(session)
                if name == 'zero budget':
                    run.wait('Next: start Tinkery with --shape-budget CALLS')
                    run.wait('Your answer: Local.')
                    assert files(sessions) == before and run.calls() == [], 'Zero budget wrote or called'
                elif name == 'model writes decisions':
                    assert not (session / 'seed-contract.md').exists(), 'A model-written decisions block was saved'
                else:
                    run.wait('Next: F2 looks into the next question.')
                    assert ledger['nodes'][1]['answer'] == 'Local.', ledger['nodes']
                    assert ledger['current_question'] is None
                    assert not (session / 'seed-contract.md').exists()
                    assert not (session / 'seed-confirmation.json').exists()
            assert helper.load(session)['goal'] == GOAL
            print(f'Shape answering {name} case passed at {width}x{height}')
        finally:
            run.close()


for size in [(100, 30), (80, 24)]:
    full_journey(*size)
for name in ['zero budget', 'stale', 'model failure', 'helper refusal', 'model tries to confirm',
             'model writes decisions', 'practice confirmation']:
    case(name)
case('zero budget', 80, 24)
