"""Shape answering journeys (#150, #159): a stub Pi, real Seed Me helper sessions in TEST roots, no model calls.

The full journey confirms a goal in Think, opens Shape from tend, sees which folder the step reads and
picks another, looks into the first question, answers it, sees the second step build on the saved
finding, and confirms the drafted seed with Seed Me's canonical sections. Short cases cover the zero budget, a stale version, a
model failure, stopping a step, a helper refusal, a reply that tries to confirm or to write the decisions, a
seed finding that looks like a secret, a practice session's confirmation, a citation that isn't in the folder,
and steps that found nothing."""
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
F6, F7, F8, F9 = b'\x1b[17~', b'\x1b[18~', b'\x1b[19~', b'\x1b[20~'

# Think's requests get a reading with one question; Shape's steps pop the next scripted reply.
HOST = r'''#!/usr/bin/env python3
import json, pathlib, sys
import os
r = json.load(sys.stdin)
home = pathlib.Path(os.environ['TINKERY_TEST_DIR'])
if 'ledger' in r:
    with (home / 'shape-calls.jsonl').open('a') as f:
        f.write(json.dumps({'argv': sys.argv[1:], 'cwd': os.getcwd(), 'input': r}) + '\n')
    replies = home / 'shape-replies.json'
    queue = json.loads(replies.read_text())
    reply = queue.pop(0)
    replies.write_text(json.dumps(queue))
    if reply == 'FAIL':
        sys.exit(3)
    if reply == 'SLOW':
        import time
        time.sleep(60)
    print(reply if isinstance(reply, str) else json.dumps(reply))
    sys.exit(0)
with (home / 'think-calls.jsonl').open('a') as f:
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
SEED_BODY = '''Refusals stay calm and records stay local.
Build calm refusals.
Do not build sync.
Accepted unchanged: 0 defaults; chosen: 1 storage decision.
No blocking unknowns or reversed decisions.

## Problem statement
Refusals are noisy; the operator loses the thread.

## Proposed outcome
Refusals stay calm and records stay local.

## Affected users and systems
The operator and local refusal path.

## Constraints and boundaries
Keep the operator's words. Do not run agents without permission.

## Knowledge map
What we know, with proof: see recorded evidence below.
What we know we don't know: operator judgment of calmness.
What we have not read: paths not listed in the evidence.
What could surprise us: private text in a refusal; re-check when refusal code changes.

## Suggested first slice
Proposal only: inspect the local refusal path before changing it.
cmd: git status --short
cwd: .

## Risks and verification
Private text could leak; the proposed criterion is not completed verification.

## Open questions and deferrals
The operator judges calmness after implementation. No blocking decisions remain.

## Owners and next steps
The operator judges refusal output; implementation checks privacy.

## Downstream handoff
Carry seed-contract.md, ledger.json and seed-confirmation.json together; check session.py status for handoff.verified. This is not implementation approval.
'''
SEED = {'next': 'seed', 'confirming': SEED_BODY,
        'checks': [{'statement': 'Refusals never show private text.', 'kind': 'judge',
                    'source': 'TEST goal', 'how_to_judge': 'Inspect refusal output for private text.'}],
        'found': [{'checked': 'src/main.rs:12', 'observed': 'the refusal path; the change touches it'}],
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
    # Most cases are about later steps, so the pre-mortem is already answered.
    nodes.append(node('probe-pre-mortem', status='settled', answer='TEST pre-mortem answer', authority=authority,
                      authority_source='TEST fixture, not real approval', label='Probe: pre-mortem',
                      question='Imagine this shipped and turned out wrong. What is the most likely reason?'))
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
        env = {**os.environ, 'TERM': 'xterm-256color', 'TINKERY_NODE': '/tinkery-TEST-no-node',
               'TINKERY_TEST_DIR': str(cwd)}
        self.process = subprocess.Popen(args, stdin=self.slave, stdout=self.slave, stderr=self.slave,
                                        cwd=cwd, env=env)

    def flat(self):
        return ' '.join(self.screen.text().split())

    def wait(self, text, timeout=20, *, wrapped_path=False):
        deadline = time.monotonic() + timeout
        normalize = (lambda value: ''.join(value.split())) if wrapped_path else (lambda value: ' '.join(value.split()))
        want = normalize(text)
        while time.monotonic() < deadline:
            if select.select([self.master], [], [], .02)[0]:
                self.screen.feed(os.read(self.master, 65536))
            if want in normalize(self.screen.text()) and not self.screen.synchronized:
                return
            if self.process.poll() is not None:
                break
        raise AssertionError(f'{text!r} not shown:\n{self.screen.text()}')

    def send(self, data, text):
        os.write(self.master, data)
        self.wait(text)

    def read_until(self, text, timeout=20):
        """Read each rendered page; reaching the bottom can hide an earlier wrapped warning."""
        deadline = time.monotonic() + timeout
        want = ' '.join(text.split())
        for _ in range(80):
            if want in self.flat() and not self.screen.synchronized:
                return
            before = self.screen.text()
            os.write(self.master, b'\x1b[6~')
            while time.monotonic() < deadline:
                if select.select([self.master], [], [], .02)[0]:
                    self.screen.feed(os.read(self.master, 65536))
                if not self.screen.synchronized:
                    if want in self.flat():
                        return
                    if self.screen.text() != before:
                        break
                if self.process.poll() is not None:
                    raise AssertionError(f'Exited while reading {text!r}:\n{self.screen.text()}')
            else:
                break
        raise AssertionError(f'{text!r} not found while reading:\n{self.screen.text()}')

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
            target = cwd / 'TEST-target\nfolder'
            target.mkdir()
            # The question's citation is in the folder; the seed's src/main.rs:12 is not (#161).
            (target / 'README.md').write_text('# TEST\n\nLocal only.\n')
            run.open_shape()
            run.wait("Seed Me hasn't asked its next question yet.")
            run.wait('F2 look into the next question')
            # Before the first step, Shape says which folder it reads; F4 picks another.
            run.wait(f'The step reads files in {cwd.name}. F4 picks another folder.')
            # Help may run past a short screen; the Rust tests check it names the full path.
            run.send(F1, 'The next step reads files in:')
            run.send(b'\x1b', 'F2 look into the next question')
            run.send(b'\x1bOS', 'Next: F4 uses this folder')
            # The editor may scroll off the path's start; exact destinations are checked in both calls below.
            assert cwd.name in ''.join(run.screen.text().split('Next:')[0].split()), run.screen.text()
            os.write(run.master, b'/TEST-targat')
            # Fix inside the folder, and prove Enter edits rather than applying it.
            run.send(b'\x1b[D' * 2 + b'\x1b[3~e\x1b[F\rfolder', 'enter newline')
            run.wait('TEST-target\nfolder', wrapped_path=True)
            assert files(run.sessions) == before and run.calls() == [], 'Folder editing wrote or called'
            run.send(b'\x1bOS', 'The step reads files in TEST-targetfolder.')
            # Opening, scrolling and help write nothing and call no model.
            run.send(b'\x1b[B', 'F2 look into the next question')
            run.send(F1, 'Model calls left this run: 2 of 2.')
            run.send(b'\x1b', 'F2 look into the next question')
            assert files(run.sessions) == before and run.calls() == [], 'Shape wrote or called before F2'
            # Seed Me's pre-mortem comes first, in the operator's words, before any finding (#234).
            run.send(F2, 'Probe: pre-mortem Imagine this shipped and turned out wrong. What is the most likely reason?')
            assert run.calls() == [], 'The pre-mortem called a model'
            assert 'What I found' not in run.flat() and 'Nothing found' not in run.flat(), run.screen.text()
            run.type('TEST It showed private text.')
            run.send(F2, 'Where records live Keep records local or synced?')
            run.wait('What I found: README.md:3: says local only')
            probe = next(n for n in helper.load(session)['nodes'] if n['id'] == 'probe-pre-mortem')
            assert probe['answer'] == 'TEST It showed private text.' and probe['authority'] == 'user', probe
            assert 'citation not found' not in run.flat(), run.screen.text()
            run.wait('F2 save answer and look into next')
            calls = run.calls()
            assert len(calls) == 1, calls
            argv = calls[0]['argv']
            assert argv[argv.index('--tools') + 1] == 'read,grep,find,ls' and '--no-tools' not in argv, argv
            assert calls[0]['input']['answered_in_think'] == [
                {'question': 'May refusals show private text?', 'answer': 'never'}], calls[0]['input']
            assert calls[0]['cwd'] == str(target), calls[0]['cwd']
            assert calls[0]['input']['found_so_far'] == [], calls[0]['input']
            # Typing writes nothing; capitals and ? are words, not commands.
            before = files(run.sessions)
            before_all = files(cwd)
            run.type('Local? Yes, Keep it remote.')
            run.send(b'\x1b[D\x17local', 'Local? Yes, Keep it local.')
            run.send(b'\x1b[F\rOnly on this machine.', 'Only on this machine.')
            run.send(b'\x1b[200~\rTEST CR\r\nTEST CRLF\x1b[201~', 'TEST CRLF')
            answer = 'Local? Yes, Keep it local.\nOnly on this machine.\rTEST CR\r\nTEST CRLF'
            # Kitty repeat events must not act, even with nonempty edited words.
            run.send(b'\x1b[1;1:2Q' * 3 + F1, 'Type your answer. Enter adds a line.')
            run.wait('Model calls left this run: 1 of 2.')
            run.send(b'\x1b', 'TEST CRLF')
            assert files(run.sessions) == before and files(cwd) == before_all and len(run.calls()) == 1, 'Typing, paste, editing or held F2 wrote or called'
            if width == 100:
                print('TEST Shape answering 100x30 screen:\n' + run.screen.text())
            run.send(F2, 'Seed draft r1')
            run.wait('Confirming the seed is not approval to build.')
            # The displayed card comes from Seed Me, not model-written criteria.
            run.read_until('you judge: Inspect refusal output for private text.')
            # Inspect each page: the flag can be above the bottom viewport on either CI platform.
            run.read_until('citation not found: src/main.rs')
            run.wait('F3 confirm seed r1')
            assert 'status: draft' not in run.screen.text(), run.screen.text()
            ledger = helper.load(session)
            answered = next(n for n in ledger['nodes'] if n['id'] == 'storage')
            assert answered['answer'] == answer and answered['authority'] == 'user', answered
            seed = (session / 'seed-contract.md').read_text()
            assert helper.read_checks(session) == SEED['checks']
            assert helper.checks_block(SEED['checks']) in seed
            assert helper.decisions_block(ledger) in seed, seed
            assert seed.index('## Decisions (from the ledger)') < seed.index('## Knowledge map')
            assert 'MODEL-INVENTED' not in seed, seed
            assert len(run.calls()) == 2
            # The second step built on the saved finding, in the folder the operator picked.
            second = run.calls()[1]
            assert second['cwd'] == str(target), second['cwd']
            assert second['input']['found_so_far'] == [
                {'checked': 'README.md:3', 'observed': 'says local only'}], second['input']
            # The seed's grounding is the saved findings, word for word, before the decisions.
            date = next(n for n in ledger['nodes'] if n['id'] == 'storage')['evidence'][0]['at'][:10]
            today = ("## Evidence and uncertainty\n"
                     "Grounded in:\n- README.md\n\n"
                     "Provenance unavailable: reported reads do not verify filesystem origins or unrecorded reads. "
                     "Some recorded findings have no attributable read path.\n\n"
                     f"Checked by read-only file tools in {' '.join(str(target).split())}:\n"
                     f"- README.md:3: says local only (observed {date})\n"
                     f"Checked by read-only file tools in {' '.join(str(target).split())}; "
                     "citation not found: src/main.rs isn't in the folder:\n"
                     f"- src/main.rs:12: the refusal path; the change touches it (observed {date})\n\n")
            assert today + '## Suggested first slice\n' in seed, seed
            # Each finding appears once: the seed step's findings never become ledger facts (#161).
            assert not any(n['kind'] == 'fact' for n in ledger['nodes']), ledger['nodes']
            for checked in ('README.md:3', 'src/main.rs:12'):
                assert seed.count(checked) == 1, seed
            # Returning rereads the same session, including the garden's stage, without work.
            before_return = files(cwd)
            run.send(b'\x1b', 'tinkery › tend')
            run.wait('Next: confirm the seed draft with Seed Me.')
            run.send(b'\x1b', 'seed drafted')
            run.send(b'\r', 'Next: confirm the seed draft with Seed Me.')
            assert files(cwd) == before_return and len(run.calls()) == 2, 'Returning from draft wrote or called'
            run.send(b's', 'Seed draft r1')
            run.wait('F3 confirm seed r1')
            before = files(run.sessions)
            before_all = files(cwd)
            run.type('Yes, I reject r1.')
            run.send(b'\x1b[D' * 4 + b'\x17confirm', 'Yes, I confirm r1.')
            run.send(b'\x1b[H' + b'\x1b[1;2C' * 3 + b'Indeed', 'Indeed, I confirm r1.')
            run.send(b'\x1b[13;1:2~' * 3 + F1, 'Read the seed, type your confirming words on one line')
            run.send(b'\x1b', 'Indeed, I confirm r1.')
            assert files(run.sessions) == before and files(cwd) == before_all and len(run.calls()) == 2, 'Editing or held F3 wrote or called'
            run.send(F3, 'Seed ready: you confirmed r1, and it still matches.')
            status = helper.status(session)
            assert status['seed']['line_1'] == 'status: confirmed for intake', status
            assert status['handoff']['verified'] is True, status
            confirmation = json.loads((session / 'seed-confirmation.json').read_text())
            assert confirmation['source'] == 'Indeed, I confirm r1.', confirmation
            assert helper.load(session)['goal'] == goal, 'The confirmed goal changed'
            assert len(run.calls()) == 2, 'Confirmation called a model'
            before_return = files(cwd)
            run.send(b'\x1b', 'tinkery › tend')
            run.wait('Next: check the seed with Seed Me.')
            run.send(b'\x1b', 'seed needs checking')
            run.send(b'\r', 'Next: check the seed with Seed Me.')
            assert ' '.join(goal.split()) in run.flat(), 'Return changed the displayed goal'
            assert files(cwd) == before_return and len(run.calls()) == 2, 'Returning from confirmation wrote or called'
            # Ending takes F6 twice; Esc between keeps the session open (#231).
            run.send(b's', 'Seed ready: you confirmed r1, and it still matches.')
            run.wait('F6 end session')
            before = files(run.sessions)
            run.send(F6, 'End this session as completed?')
            run.wait('F6 end for good   esc keep it')
            run.send(b'\x1b', 'Nothing was ended.')
            assert files(run.sessions) == before, 'Asking or Esc ended the session'
            run.send(F6, 'Next: F6 again ends it; Esc keeps it open.')
            run.send(F6, 'Session completed. It\'s read-only now.')
            run.wait('This session is completed, so nothing in it can change.')
            # Intake needs only the verified seed, so copying it stays available.
            run.wait('c copy intake')
            assert 'F6 end session' not in run.flat(), run.screen.text()
            assert helper.load(session)['status'] == 'completed'
            assert helper.status(session)['handoff']['verified'] is True
            assert helper.load(session)['goal'] == goal, 'Ending changed the confirmed goal'
            assert len(run.calls()) == 2, 'Ending called a model'
            # Tend now counts the session ended, as its readiness needs.
            run.send(b'\x1b', 'tinkery › tend')
            run.wait('No active goals. 1 ended not shown.')
            print(f'Shape answering full journey passed at {width}x{height}: folder shown and picked, '
                  f'think answer and saved findings in the input, 2 stub calls, grounded seed confirmed '
                  f'and handoff verified; Tend detail and garden refreshed after draft and confirmation; '
                  f'F6 twice completed the session with intake still copyable')
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
        if name == 'stop':
            replies = ['SLOW']
        if name == 'helper refusal':
            replies = [{**QUESTION, 'found': [{'checked': 'config', 'observed': 'key AKIAABCDEFGHIJKLMNOP'}]}]
        if name == 'model tries to confirm':
            replies = [{'next': 'confirm', 'revision': 'r1', 'source': 'yes'}]
        if name == 'seed secret':
            question = False
            replies = [{**SEED, 'found': [{'checked': 'config', 'observed': 'key AKIAABCDEFGHIJKLMNOP'}]}]
        if name == 'model writes decisions':
            question = False
            replies = [{**SEED, 'confirming': 'Fine.\n## Decisions (from the ledger)\n- goal [user] x'}]
        if name in ('practice confirmation', 'multiline confirmation', 'confirmation hint', 'confirmation Enter'):
            question, operator = False, 'simulated' if name == 'practice confirmation' else 'human'
        if name == 'unfound citation':
            replies = [{**QUESTION, 'id': 'Sync'}]
        if name == 'hard to undo':
            question = False
            replies = [{**QUESTION, 'confidence': 'high', 'against': 'TEST sync helps later', 'options': [
                {'choice': 'Local', 'tradeoff': 'TEST simple', 'undo': 'cheap'},
                {'choice': 'Synced', 'tradeoff': 'TEST shared', 'undo': 'hard'}]}]
        if name == 'assumptions':
            question = False
            replies = [{**QUESTION, 'assumed': [{'text': 'TEST notes are Markdown', 'why': 'TEST either format works'}]}]
        if name == 'nothing found':
            replies = [{**QUESTION, 'found': []}, {**SEED, 'found': []}]
        session = make(sessions, question=question, operator=operator)
        if name in ('practice confirmation', 'multiline confirmation', 'confirmation hint', 'confirmation Enter'):
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
                run.wait('your answer Local.')
                assert files(sessions) == before, 'A stale answer was written'
                assert run.calls() == [], 'A stale answer still called the model'
            elif name == 'stop':
                run.wait('Keep records local or synced?')
                run.type('Local.')
                # While the step runs, the question and the saved answer stay in view.
                run.send(F2, f'Answer saved. Looking in {cwd.name} for the next question.')
                run.wait('Keep records local or synced? your answer Local.')
                run.wait('esc stop')
                deadline = time.monotonic() + 10
                while not run.calls() and time.monotonic() < deadline:
                    time.sleep(.05)
                run.send(b'\x1b', 'Your answer is saved. Stopped before the next step finished.')
                run.wait('Next: F2 looks into the next question.')
                run.wait("still counts against this run's budget.")
                assert helper.load(session)['nodes'][1]['answer'] == 'Local.'
                assert not (session / 'seed-contract.md').exists()
            elif name == 'end the interview':
                run.wait('Keep records local or synced?')
                run.type('Local.')
                before = files(sessions)
                run.send(F8, 'Stop this session?')
                run.wait('your answer Local.')
                # Asked, typing is not words and other keys do nothing; Esc keeps it open.
                run.send(b'x' + F2, 'F8 stop for good   esc keep it')
                run.send(b'\x1b', 'Nothing was ended.')
                run.wait('your answer Local.')
                assert files(sessions) == before and run.calls() == [], 'Asking or Esc wrote or called'
                run.send(F8, 'Next: F8 again stops it; Esc keeps it open.')
                run.send(F8, 'This session has ended (stopped).')
                ledger = helper.load(session)
                assert ledger['status'] == 'stopped', ledger['status']
                # The open question stays recorded, unanswered; the typed words were never saved.
                assert ledger['nodes'][1]['status'] == 'unresolved', ledger['nodes'][1]
                assert 'answer' not in ledger['nodes'][1], ledger['nodes'][1]
                assert run.calls() == [], 'Stopping called a model'
            elif name == 'hard to undo':
                run.wait("Seed Me hasn't asked its next question yet.")
                run.send(F2, "This is hard to undo, so what's your instinct?")
                for hidden in ('Option A', 'Suggestion:', 'Against my suggestion'):
                    assert hidden not in run.flat(), run.screen.text()
                node = next(n for n in helper.load(session)['nodes'] if n['id'] == 'storage')
                assert 'recommendation' not in node, node
                assert 'Option B: Synced; TEST shared. Undo cost: hard.' in node['question'], node
                run.send(F9, 'Here are the options and my suggestion.')
                run.wait('Option B: Synced; TEST shared. Undo cost: hard.')
                run.wait('Suggestion: Local, matching the README. Confidence: high.')
                node = next(n for n in helper.load(session)['nodes'] if n['id'] == 'storage')
                assert node['recommendation'] == 'Local, matching the README. Confidence: high.', node
                assert 'F9' not in run.screen.text().splitlines()[-1], run.screen.text()
                assert len(run.calls()) == 1, 'Showing the suggestion called a model'
            elif name == 'assumptions':
                run.wait("Seed Me hasn't asked its next question yet.")
                run.send(F2, "I'll assume these unless you object:")
                run.wait('1. TEST notes are Markdown (why: TEST either format works)')
                assert helper.load(session)['assumed'] == [{'text': 'TEST notes are Markdown', 'why': 'TEST either format works'}]
                assert len(run.calls()) == 1
                # Asked which, typing is not words; Esc keeps them, then F7 and its number objects.
                before = files(sessions)
                run.send(F7 + b'x', 'Which assumption do you object to?')
                run.send(b'\x1b', 'Nothing was changed.')
                assert files(sessions) == before, 'Asking which or Esc wrote'
                run.send(F7, 'Which assumption do you object to?')
                run.send(b'1', 'Not assumed: TEST notes are Markdown.')
                ledger = helper.load(session)
                assert ledger['assumed'] == [], ledger['assumed']
                objection = next(n for n in ledger['nodes'] if n.get('label') == 'Instead of assuming')
                assert objection['status'] == 'unresolved', objection
                assert "I'll assume these" not in run.flat(), run.screen.text()
                # Answering the current question asks the objection next, with no model call.
                run.type('Local.')
                run.send(F2, 'You objected to assuming: TEST notes are Markdown. What should it be instead?')
                assert len(run.calls()) == 1, 'Asking the objection called a model'
                assert helper.load(session)['current_question'] == objection['id']
            elif name == 'unfound citation':
                run.wait('Keep records local or synced?')
                run.type('Local.')
                run.send(F2, "What I found: README.md:3: says local only (citation not found: README.md isn't in the folder)")
            elif name == 'nothing found':
                run.wait('Keep records local or synced?')
                run.type('Local.')
                run.send(F2, 'Nothing found: this question records nothing checked in the repository.')
                run.type('Yes.')
                run.send(F2, 'Nothing found: this seed records nothing checked in the repository.')
                ledger = helper.load(session)
                assert helper.decisions_block(ledger) in (session / 'seed-contract.md').read_text()
                assert not any(n['kind'] == 'fact' for n in ledger['nodes']), ledger['nodes']
            elif name in ('confirmation hint', 'confirmation Enter'):
                run.wait('Seed draft r1')
                if name == 'confirmation hint':
                    run.wait('a line saying you confirm, then F3')
                run.wait('Confirming the seed is not approval to build.')
                before = files(cwd)
                run.send(b'\r', 'F3 confirms the seed; Enter does not add a line.')
                run.wait('a line saying you confirm, then F3')
                run.type('TEST I confirm r1.')
                assert 'a line saying you confirm, then F3' not in run.flat(), run.screen.text()
                run.send(b'\r' + F1, 'Read the seed, type your confirming words on one line')
                run.send(b'\x1b', 'your words TEST I confirm r1.')
                # F3 reaches Seed Me, not the multiline refusal: neither Enter inserted a break.
                run.send(F3, 'Seed Me refused:')
                assert 'Put your confirming words on one line' not in run.flat(), run.screen.text()
                assert files(cwd) == before, 'Hint, Enter or refused F3 wrote a file'
                assert run.calls() == [], 'Confirmation guidance called a model'
            elif name == 'multiline confirmation':
                run.wait('Seed draft r1')
                before = files(cwd)
                # Pasted LF/CR/CRLF still refuse without normalizing the words.
                for ending in ('\n', '\r', '\r\n'):
                    run.send(b'\x1b[200~' + ('I confirm' + ending + 'r1.').encode() + b'\x1b[201~', 'r1.')
                    run.send(F3, 'Put your confirming words on one line; nothing was confirmed.')
                    assert files(cwd) == before, 'Multiline F3 wrote a file'
                    assert run.calls() == [], 'Multiline F3 called a model'
                    run.wait('your words I confirm r1.')
                    # CRLF is one editor newline, like CR and LF.
                    run.send(b'\x7f' * 13 + F1, 'Read the seed, type your confirming words on one line')
                    run.send(b'\x1b', 'your words')
                assert not (session / 'seed-confirmation.json').exists()
            elif name == 'practice confirmation':
                run.wait('Seed draft r1')
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
                    'seed secret': 'A finding appears to contain a secret, so no seed was drafted',
                }[name]
                run.send(F2, expected)
                assert run.flat().count('Next:') == 1, run.screen.text()
                ledger = helper.load(session)
                if name == 'zero budget':
                    run.wait('Next: start Tinkery with --shape-budget CALLS')
                    run.wait('your answer Local.')
                    assert files(sessions) == before and run.calls() == [], 'Zero budget wrote or called'
                elif name in ('model writes decisions', 'seed secret'):
                    assert not (session / 'seed-contract.md').exists(), 'A model-written decisions block or a secret was saved'
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


if __name__ == '__main__':
    for size in [(100, 30), (80, 24)]:
        full_journey(*size)
    for name in ['zero budget', 'stale', 'model failure', 'stop', 'helper refusal', 'model tries to confirm',
                 'model writes decisions', 'seed secret', 'practice confirmation', 'unfound citation', 'nothing found',
                 'end the interview', 'assumptions', 'hard to undo']:
        case(name)
    case('zero budget', 80, 24)
    case('assumptions', 80, 24)
    case('hard to undo', 80, 24)
    for size in [(100, 30), (80, 24)]:
        case('multiline confirmation', *size)
        os.environ['NO_COLOR'] = '1'
        case('confirmation hint', *size)
        case('confirmation Enter', *size)
