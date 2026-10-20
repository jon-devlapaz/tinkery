"""Shape answering journeys (#150, #159): a stub Pi, real Seed Me helper sessions in TEST roots, no model calls.

The full journey confirms a goal in Think, opens Shape from tend, sees which folder the step reads and
picks another, looks into the first question, answers it, sees the second step build on the saved
finding, and confirms the drafted seed with Seed Me's canonical sections. Short cases cover the zero budget, a stale version, a
model failure, stopping a step, a helper refusal, a reply that tries to confirm or to write the decisions, a
seed finding that looks like a secret, a practice session's confirmation, a citation that isn't in the folder,
and steps that found nothing."""
import fcntl, hashlib, importlib.util, json, os, select, signal, struct, subprocess, tempfile, termios, time
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
F11, F12 = b'\x1b[23~', b'\x1b[24~'

# Think's requests get a reading with one question; Shape's steps pop the next scripted reply.
HOST = r'''#!/usr/bin/env python3
import json, pathlib, sys
import os
r = json.load(sys.stdin)
home = pathlib.Path(os.environ['TINKERY_TEST_DIR'])
if 'ledger' in r or r.get('kind') == 'teach-back':
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
            'found': [{'id': 'read-local', 'claim': {'type': 'observation', 'scope': 'README.md:3'},
                       'checked': 'README.md:3', 'observed': 'says local only'}],
            'supported_by': ['read-local'],
            'suggestion': 'Local, matching the README.'}
def completion_review(checks):
    return {'contradictions': [], 'decided': ['TEST agreed scope, not completed work'], 'done': [],
            'failure_cases': [{'statement': c['statement'], 'case': 'TEST wrong output must fail'} for c in checks]}


def seed_fields(body):
    """Replay historical Markdown fixtures as prose fields; production never parses model headings."""
    fields = {}
    name, lines = 'You are confirming', []
    for line in body.splitlines():
        if line.startswith('## '):
            if lines:
                fields[name] = '\n'.join(lines).strip()
            name, lines = line[3:], []
        else:
            lines.append(line)
    if lines:
        fields[name] = '\n'.join(lines).strip()
    return fields


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
SEED = {'next': 'seed', 'confirming': seed_fields(SEED_BODY),
        'checks': [{'statement': 'Refusals never show private text.', 'kind': 'judge',
                    'source': 'TEST goal', 'how_to_judge': 'Inspect refusal output for private text.'}],
        'found': [{'id': 'read-refusal', 'claim': {'type': 'observation', 'scope': 'src/main.rs:12'},
                   'checked': 'src/main.rs:12', 'observed': 'the refusal path; the change touches it'}],
        'decisions': '- storage [user] MODEL-INVENTED synced (source: the model)'}
SEED['review'] = completion_review(SEED['checks'])
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
            run.wait("Shape hasn't asked its next question yet.")
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
            run.wait('F2 save + look')
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
            recorded = second['input']['found_so_far']
            assert len(recorded) == 1 and recorded[0]['id'] == 'read-local', recorded
            assert recorded[0]['claim'] == QUESTION['found'][0]['claim'], recorded
            # The seed's grounding is the saved findings, word for word, before the decisions.
            date = next(n for n in ledger['nodes'] if n['id'] == 'read-local')['evidence'][0]['at'][:10]
            today = ("## Evidence and uncertainty\n"
                     "Grounded in:\n- README.md\n\n"
                     "Provenance unavailable: reported reads do not verify filesystem origins or unrecorded reads. "
                     "Some recorded findings have no attributable read path.\n\n"
                     f"Checked by read-only file tools in {' '.join(str(target).split())}:\n"
                     f"- README.md:3: says local only (observation; scope: README.md:3) (observed {date})\n"
                     f"Checked by read-only file tools in {' '.join(str(target).split())}; "
                     "citation not found: src/main.rs isn't in the folder:\n"
                     f"- src/main.rs:12: the refusal path; the change touches it (observation; scope: src/main.rs:12) (observed {date})\n\n")
            assert today + '## Suggested first slice\n' in seed, seed
            # Both question and seed findings are facts; prose appears once in seed evidence.
            assert len([n for n in ledger['nodes'] if n['kind'] == 'fact']) == 2, ledger['nodes']
            assert answered['supported_by'] == ['read-local'] and answered['evidence'] == []
            for observed in ('says local only', 'the refusal path; the change touches it'):
                evidence = seed.split('## Evidence and uncertainty\n', 1)[1].split('## Suggested first slice', 1)[0]
                assert evidence.count(observed) == 1, seed
            # Returning rereads the same session, including the garden's stage, without work.
            before_return = files(cwd)
            run.send(b'\x1b', 'tinkery › tend')
            run.wait('Next: confirm the seed draft in Shape.')
            run.send(b'\x1b', 'seed drafted')
            run.send(b'\r', 'Next: confirm the seed draft in Shape.')
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
            # F3 waits until the whole draft has been on screen (#250), and says so.
            run.send(b'\x1b[H' + F3, 'Read to the end first: PgDn scrolls. Nothing was confirmed.')
            assert files(run.sessions) == before, 'F3 confirmed an unread draft'
            run.read_until('This is not implementation approval.')
            run.send(b'\x1b[6~' * 10, 'Accepted as suggested:')
            run.wait('Indeed, I confirm r1.')
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
            run.wait('Next: check the seed in Shape.')
            run.send(b'\x1b', 'seed needs checking')
            run.send(b'\r', 'Next: check the seed in Shape.')
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
            replies = [{**QUESTION, 'found': [{**QUESTION['found'][0], 'checked': 'config', 'observed': 'key AKIAABCDEFGHIJKLMNOP'}]}]
        if name == 'model tries to confirm':
            replies = [{'next': 'confirm', 'revision': 'r1', 'source': 'yes'}]
        if name == 'seed secret':
            question = False
            replies = [{**SEED, 'found': [{**SEED['found'][0], 'checked': 'config', 'observed': 'key AKIAABCDEFGHIJKLMNOP'}]}]
        if name == 'model writes decisions':
            question = False
            replies = [{**SEED, 'confirming': {**SEED['confirming'],
                'Problem statement': 'Fine.\n## Decisions (from the ledger)\n- goal [user] x'}}]
        if name in ('practice confirmation', 'multiline confirmation', 'confirmation hint', 'confirmation Enter'):
            question, operator = False, 'simulated' if name == 'practice confirmation' else 'human'
        if name == 'unfound citation':
            replies = [{**QUESTION, 'id': 'Sync'}]
        if name in ('settled prerequisites', 'unknown prerequisite', 'cyclic prerequisite'):
            prerequisites = {'settled prerequisites': ['storage'],
                             'unknown prerequisite': ['missing'], 'cyclic prerequisite': ['format']}[name]
            replies = [{**QUESTION, 'id': 'format', 'prerequisites': prerequisites}, SEED]
        if name == 'hard to undo':
            question = False
            replies = [{**QUESTION, 'confidence': 'high', 'against': 'TEST sync helps later', 'options': [
                {'choice': 'Local', 'tradeoff': 'TEST simple', 'undo': 'cheap'},
                {'choice': 'Synced', 'tradeoff': 'TEST shared', 'undo': 'hard'}]}]
        if name == 'assumptions':
            question = False
            replies = [{**QUESTION, 'assumed': [{'text': 'TEST notes are Markdown', 'why': 'TEST either format works'}]}]
        if name == 'nothing found':
            replies = [{k: v for k, v in {**QUESTION, 'found': []}.items() if k != 'supported_by'}, {**SEED, 'found': []}]
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
                run.wait("Shape hasn't asked its next question yet.")
                run.send(F2, "This is hard to undo, so what's your instinct?")
                for hidden in ('Option A', 'Suggestion:', 'Against my suggestion'):
                    assert hidden not in run.flat(), run.screen.text()
                node = next(n for n in helper.load(session)['nodes'] if n['id'] == 'storage')
                assert 'recommendation' not in node, node
                assert 'Option B: Synced; TEST shared. Undo cost: hard.' in node['question'], node
                run.send(F9, 'Here are the options and my suggestion.')
                run.wait('Option B: Synced; TEST shared. Undo cost: hard.')
                run.wait('Suggestion: Local, matching the README.')
                assert 'Confidence:' not in run.screen.text(), run.screen.text()
                node = next(n for n in helper.load(session)['nodes'] if n['id'] == 'storage')
                assert node['recommendation'] == 'Local, matching the README.', node
                assert 'F9' not in run.screen.text().splitlines()[-1], run.screen.text()
                assert len(run.calls()) == 1, 'Showing the suggestion called a model'
            elif name == 'assumptions':
                run.wait("Shape hasn't asked its next question yet.")
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
            elif name == 'settled prerequisites':
                run.wait('Keep records local or synced?')
                run.type('TEST local')
                run.send(F2, 'Suggestion: Local, matching the README.')
                saved = helper.load(session)
                assert saved['current_question'] == 'format'
                assert next(n for n in saved['nodes'] if n['id'] == 'format')['prerequisites'] == ['storage']
                assert len(run.calls()) == 1
            elif name in ('unknown prerequisite', 'cyclic prerequisite'):
                run.wait('Keep records local or synced?')
                run.type('TEST local')
                run.send(F2, "isn't recorded" if name == 'unknown prerequisite' else 'in a circle')
                run.wait('retry the step')
                saved = helper.load(session)
                assert saved['current_question'] is None
                assert next(n for n in saved['nodes'] if n['id'] == 'storage')['answer'] == 'TEST local'
                assert not any(n['id'] in ('format', 'read-local') for n in saved['nodes']), saved
                assert not saved['assumed'] and len(run.calls()) == 1
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
                # F3 needs the whole draft read first (#250); these cases are about the words.
                os.write(run.master, b'\x1b[6~' * 10)
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
                run.send(F3, 'Tinkery refused:')
                assert 'Put your confirming words on one line' not in run.flat(), run.screen.text()
                assert files(cwd) == before, 'Hint, Enter or refused F3 wrote a file'
                assert run.calls() == [], 'Confirmation guidance called a model'
            elif name == 'multiline confirmation':
                run.wait('Seed draft r1')
                os.write(run.master, b'\x1b[6~' * 10)
                before = files(cwd)
                # Pasted LF/CR/CRLF still refuse without normalizing the words.
                for ending in ('\n', '\r', '\r\n'):
                    run.send(b'\x1b[200~' + ('I confirm' + ending + 'r1.').encode() + b'\x1b[201~', 'r1.')
                    run.send(F3, 'Put your confirming words on one line; nothing was confirmed.')
                    assert files(cwd) == before, 'Multiline F3 wrote a file'
                    assert run.calls() == [], 'Multiline F3 called a model'
                    run.wait('your words I confirm r1.')
                    # CRLF is one editor newline, like CR and LF.
                    # Changed words clear the refusal about the old ones (#249).
                    run.send(b'\x7f' * 13, 'a line saying you confirm, then F3')
                    assert 'Put your confirming words on one line' not in run.flat(), run.screen.text()
                    run.send(F1, 'Read the seed, type your confirming words on one line')
                    run.send(b'\x1b', 'your words')
                assert not (session / 'seed-confirmation.json').exists()
            elif name == 'practice confirmation':
                run.wait('Seed draft r1')
                run.type('I confirm r1.')
                before = files(sessions)
                run.send(F3, 'Tinkery refused: a simulated session cannot be confirmed')
                assert files(sessions) == before, 'A practice seed was confirmed'
            else:
                if question:
                    run.wait('Keep records local or synced?')
                    run.type('Local.')
                else:
                    run.wait("Shape hasn't asked its next question yet.")
                before = files(sessions)
                expected = {
                    'zero budget': "No model calls are left: this run's budget is 0.",
                    'model failure': 'Your answer is saved. Looking into the next question failed: Pi exited',
                    'helper refusal': 'Your answer is saved. Tinkery refused: receipt appears to contain a secret',
                    'model tries to confirm': "Looking into the next question failed: The model's reply wasn't a question or a seed draft.",
                    'model writes decisions': "field confirming / Problem statement, line 2: ## Decisions (from the ledger)",
                    'seed secret': 'Tinkery refused: receipt appears to contain a secret',
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


def prerequisite_questions(width, height):
    """Prepared A/B ledger: recorded ready questions still precede any model step (#286).

    Adjusted from #286 (Claude's call, keeping the documented order): a stub cannot propose B
    while ready A is unanswered, because that existing scheduling rule correctly asks A before calling Pi.
    """
    with tempfile.TemporaryDirectory(prefix='tinkery-TEST-prerequisites-') as directory:
        cwd = Path(directory).resolve()
        session = make(cwd / 'TEST-sessions')
        old = helper.load(session)
        state = helper.editable(old)
        state['current_question'] = None
        state['nodes'].append(node('b', status='unresolved', prerequisites=['storage'],
            label='TEST second choice', question='TEST which format after storage?'))
        helper.publish(session, state, old['version'], 'TEST prepared prerequisite questions')
        run = Run(cwd, width, height, budget=1, replies=[SEED])
        try:
            run.open_shape()
            run.wait('F2 look into the next question')
            before = files(run.sessions)
            run.send(b'\x1b[15~', 'Open questions')
            run.read_until('TEST second choice — waits for: Where records live')
            assert files(run.sessions) == before and run.calls() == []
            run.send(b'\x1b[15~', 'F2 look into the next question')
            run.send(F2, 'Keep records local or synced?')
            assert helper.load(session)['current_question'] == 'storage'
            assert run.calls() == [], 'Promoting A called a model'
            run.type('TEST A stays local')
            run.send(F2, 'TEST which format after storage?')
            saved = helper.load(session)
            assert saved['current_question'] == 'b'
            assert next(n for n in saved['nodes'] if n['id'] == 'storage')['answer'] == 'TEST A stays local'
            assert next(n for n in saved['nodes'] if n['id'] == 'b')['prerequisites'] == ['storage']
            assert run.calls() == [], 'Promoting B called a model'
            run.send(b'\x1b[15~', 'Open questions')
            assert 'waits for:' not in run.flat(), run.screen.text()
            run.send(b'\x1b[15~', 'TEST which format after storage?')
            run.type('TEST B is Markdown')
            run.send(F2, 'Seed draft r1')
            assert len(run.calls()) == 1 and helper.load(session)['goal'] == GOAL
            print(f'Shape prerequisites passed at {width}x{height}: prepared TEST A/B ledger, '
                  'F5 names the held prerequisite, A then B without model calls, 1 final stub reply')
        finally:
            run.close()


def typed_findings_review(width, height):
    """Two stub investigations, then an operator-only review before confirmation (#282)."""
    with tempfile.TemporaryDirectory(prefix='tinkery-TEST-typed-findings-') as directory:
        cwd = Path(directory).resolve()
        (cwd / 'README.md').write_text('# TEST\n\nLocal only.\nA later observation conflicts.\n')
        inferred = {'id': 'infer-local', 'claim': {'type': 'inference', 'scope': 'TEST storage path',
                    'limits': 'TEST not runtime verification'}, 'checked': 'README.md:3',
                    'observed': 'TEST the documented path is local', 'supported_by': ['read-local']}
        question = {**QUESTION, 'found': [*QUESTION['found'], inferred],
                    'supported_by': ['read-local', 'infer-local']}
        conflict = {'id': 'conflict', 'claim': {'type': 'observation', 'scope': 'TEST later lines'},
                    'checked': 'README.md:4', 'observed': 'TEST later evidence conflicts with storage',
                    'contradicts': 'storage'}
        seed = {**SEED, 'found': [conflict]}
        session = make(cwd / 'TEST-sessions', question=False)
        run = Run(cwd, width, height, budget=2, replies=[question, seed])
        try:
            run.open_shape()
            run.wait('F2 look into the next question')
            run.send(F2, 'Keep records local or synced?')
            ledger = helper.load(session)
            current = next(n for n in ledger['nodes'] if n['id'] == 'storage')
            assert current['supported_by'] == ['read-local', 'infer-local'], current
            assert current['status'] == 'unresolved' and 'answer' not in current
            facts = [n for n in ledger['nodes'] if n['kind'] == 'fact']
            assert [n['claim']['type'] for n in facts] == ['observation', 'inference'], facts
            assert facts[1]['supported_by'] == ['read-local']
            assert all(n['authority'] == 'evidence' and n['evidence'] for n in facts)
            run.send(b'\x1b[15~', 'TEST the documented path is local')
            run.send(b'\x1b[15~', 'your answer')
            run.type('TEST keep local')
            run.send(F2, 'Current question: review flagged answer')
            run.wait('Keep Where records live?')
            run.wait('Settled answer: TEST keep local')
            run.wait('F6 keep with reason')
            ledger = helper.load(session)
            assert helper.review_flags(ledger['nodes']), ledger
            seed_text = (session / 'seed-contract.md').read_text()
            for text in ['observation; scope:', 'inference; scope: TEST storage path',
                         'limits: TEST not runtime verification', 'supported by: read-local']:
                assert text in seed_text, seed_text
            evidence = seed_text.split('## Evidence and uncertainty\n', 1)[1].split('## Suggested first slice', 1)[0]
            assert evidence.count('TEST the documented path is local') == 1, seed_text
            before = files(run.sessions)
            run.send(F3, 'Review the flagged answer first; nothing was confirmed.')
            run.send(F6, "Type why you're keeping this answer first; nothing was saved.")
            assert files(run.sessions) == before and len(run.calls()) == 2
            run.type('TEST the scope still fits')
            # A held keep key is not an operator review; typing and help also save nothing.
            run.send(b'\x1b[17;1:2~' + F1, 'F6 keeps it unchanged with your reason; no model runs.')
            run.send(b'\x1b', 'your reason TEST the scope still fits')
            assert files(run.sessions) == before and len(run.calls()) == 2
            run.send(F6, 'Seed draft r1')
            run.wait('Answer kept with your reason. No model ran.')
            ledger = helper.load(session)
            kept = next(n for n in ledger['nodes'] if n['id'] == 'storage')
            assert kept['answer'] == 'TEST keep local' and kept['authority'] == 'user', kept
            assert kept['history'][-1]['reason'] == 'TEST the scope still fits', kept
            assert not helper.review_flags(ledger['nodes']), ledger
            run.type('TEST confirm only r1')
            run.read_until('This is not implementation approval.')
            # The handoff paragraph is not the end: confirmation rows follow it.
            run.send(b'\x1b[6~' * 10, 'Accepted as suggested: 0; chosen otherwise: 2.')
            run.send(F3, 'Seed ready: you confirmed r1, and it still matches.')
            assert helper.status(session)['handoff']['verified'] is True
            assert len(run.calls()) == 2 and helper.load(session)['goal'] == GOAL
            print(f'Shape typed findings and contradiction review passed at {width}x{height}: '
                  '2 stub calls, F3 held, F6 keep-with-reason, unchanged answer, verified seed')
        finally:
            run.close()


def answer_actions(width, height):
    """Explicit deferral, grounded delegation and cascading revision, with stub replies only."""
    for action in ['defer', 'delegate', 'no facts', 'no suggestion', 'revise', 'revise draft', 'revise zero budget']:
        with tempfile.TemporaryDirectory(prefix='tinkery-TEST-answer-actions-') as directory:
            cwd = Path(directory).resolve()
            (cwd / 'README.md').write_text('# TEST\n\nLocal only.\n')
            session = make(cwd / 'TEST-sessions')
            old = helper.load(session)
            state = helper.editable(old)
            current = state['nodes'][1]
            if action in ['delegate', 'no facts', 'no suggestion']:
                if action != 'no suggestion':
                    current['recommendation'] = 'TEST choose local from the evidence.'
                if action != 'no facts':
                    state['nodes'].append({'id': 'test-grounding', 'kind': 'fact', 'status': 'settled',
                        'prerequisites': [], 'claim': {'type': 'observation', 'scope': 'TEST README'},
                        'answer': 'TEST local only', 'authority': 'evidence',
                        'authority_source': 'TEST fixture', 'evidence': ['TEST README says local only']})
                    current['supported_by'] = ['test-grounding']
            if action.startswith('revise'):
                current.update(status='settled', answer='TEST old local', authority='user',
                               authority_source='TEST fixture, not operator approval')
                state['current_question'] = None
                dependents = [('child', 'storage'), ('grandchild', 'child')] if action == 'revise' else []
                for identifier, parent in dependents:
                    state['nodes'].append(node(identifier, prerequisites=[parent], status='settled',
                        question=f'TEST answer {identifier} again?', answer=f'TEST old {identifier}',
                        authority='user', authority_source='TEST fixture, not operator approval'))
                state['nodes'].append({'id': 'test-conflict', 'kind': 'fact', 'status': 'settled',
                    'prerequisites': [], 'claim': {'type': 'observation', 'scope': 'TEST conflict'},
                    'answer': 'TEST conflicts with old storage', 'authority': 'evidence',
                    'authority_source': 'TEST fixture', 'evidence': ['TEST conflicting observation'],
                    'contradicts': 'storage'})
            helper.publish(session, state, old['version'], 'TEST action fixture')
            if action == 'revise draft':
                (session / 'seed-contract.md').write_text('status: draft\n# TEST old draft\nrevision: r1\n\n'
                    + helper.decisions_block(helper.load(session)))
            replies = [SEED] if action.startswith('revise') else [QUESTION, SEED]
            run = Run(cwd, width, height, budget=0 if action == 'revise zero budget' else 2, replies=replies)
            try:
                run.open_shape()
                run.wait('F2 revise' if action.startswith('revise') else 'F11 defer')
                before = files(run.sessions)
                if action == 'defer':
                    run.type('TEST untouched answer')
                    run.send(F11, 'reason to defer')
                    run.wait('Gate: TEST choice')
                    run.type('TEST wait for scope')
                    run.send(F11, 'revisit condition')
                    run.send(F11, 'deferral requires reason and revisit condition')
                    assert files(run.sessions) == before and run.calls() == []
                    run.send(b'\t', 'reason to defer TEST wait for scope')
                    run.send(b'\t', 'revisit condition')
                    run.type('TEST before implementation')
                    run.send(F11, 'Suggestion: Local, matching the README.')
                    saved = helper.load(session)
                    deferred = next(n for n in saved['nodes'] if n['id'] == 'storage')
                    assert deferred['status'] == 'deferred', deferred
                    assert deferred['defer_reason'] == 'TEST wait for scope'
                    assert deferred['revisit_condition'] == 'TEST before implementation'
                    assert deferred.get('answer') is None and not (session / 'seed-confirmation.json').exists()
                    assert len(run.calls()) == 1
                elif action == 'delegate':
                    run.send(F12, 'Suggestion: Local, matching the README.')
                    delegated = next(n for n in helper.load(session)['nodes'] if n['id'] == 'storage')
                    assert delegated['authority'] == 'delegated' and delegated['answer'] == 'TEST choose local from the evidence.', delegated
                    assert delegated['recommendation'] == current['recommendation'], delegated
                    assert delegated['evidence'] == ['TEST README says local only'], delegated
                    assert "'F12 delegate suggestion'" in delegated['authority_source'], delegated
                    assert len(run.calls()) == 1
                elif action in ['no facts', 'no suggestion']:
                    run.type('TEST retain my answer')
                    run.send(F12, 'no supporting facts' if action == 'no facts' else 'no suggestion')
                    run.wait('your answer TEST retain my answer')
                    assert files(run.sessions) == before and run.calls() == []
                elif action in ['revise draft', 'revise zero budget']:
                    run.send(F2, 'F2 save revision and look')
                    run.type('TEST revised synced only with permission')
                    if action == 'revise zero budget':
                        run.send(F2, 'No model calls are left')
                        run.wait('TEST revised synced only with permission')
                        assert files(run.sessions) == before and run.calls() == []
                    else:
                        run.send(F2, 'Seed draft r2')
                        saved = helper.load(session)
                        assert saved['goal'] == GOAL and saved['current_question'] is None
                        seed = (session / 'seed-contract.md').read_text()
                        assert helper.decisions_block(saved) in seed and 'TEST revised synced only with permission' in seed
                        assert len(run.calls()) == 1, 'Revision without dependents did not take the normal next step'
                else:
                    run.wait('F2 revise')
                    run.send(F2, 'F2 save revision and look')
                    run.type('TEST revised synced only with permission')
                    run.send(F2, 'TEST answer child again?')
                    saved = helper.load(session)
                    assert saved['goal'] == GOAL and saved['current_question'] == 'child'
                    for identifier in ['child', 'grandchild']:
                        reopened = next(n for n in saved['nodes'] if n['id'] == identifier)
                        assert reopened['status'] == 'unresolved' and reopened.get('answer') is None, reopened
                        assert 'storage' in reopened['reopen_reason'], reopened
                    assert run.calls() == [], 'Revision called a model'
                    run.type('TEST new child')
                    run.send(F2, 'TEST answer grandchild again?')
                    assert run.calls() == [], 'Asking a reopened dependent called a model'
                    run.type('TEST new grandchild')
                    run.send(F2, 'Seed draft r1')
                    assert len(run.calls()) == 1
                if action in ['defer', 'delegate']:
                    run.type('TEST answer the next question')
                    run.send(F2, 'Seed draft r1')
                    seed = (session / 'seed-contract.md').read_text()
                    decisions = helper.decisions_block(helper.load(session))
                    assert decisions in seed, seed
                    assert ('storage deferred:' if action == 'defer' else '[delegated]') in decisions, decisions
                    assert len(run.calls()) == 2
                print(f'Shape {action} passed at {width}x{height}: TEST roots, {len(run.calls())} stub calls')
            finally:
                run.close()


def closing_summary(width, height):
    """Ledger risks appear above confirmation, and F3 records an exact skip without a call."""
    with tempfile.TemporaryDirectory(prefix='tinkery-TEST-closing-summary-') as directory:
        cwd = Path(directory).resolve()
        session = make(cwd / 'TEST-sessions', question=False)
        old = helper.load(session)
        state = helper.editable(old)
        for identifier, answer, recommendation, authority in [
            ('changed', 'TEST local', None, 'user'),
            ('delegated', 'TEST suggested', 'TEST suggested', 'delegated'),
            ('accepted', 'TEST yes', 'TEST yes', 'user'),
            ('chosen', 'TEST yes ', 'TEST yes', 'user')]:
            choice = node(identifier, status='settled', answer=answer, authority=authority,
                          authority_source='TEST fixture only', label=identifier,
                          evidence=['TEST grounding'] if authority == 'delegated' else [])
            if recommendation:
                choice['recommendation'] = recommendation
            state['nodes'].append(choice)
        state['assumed'] = [{'text': 'TEST Markdown', 'why': 'TEST either format works'}]
        helper.publish(session, state, old['version'], 'TEST choices')
        old = helper.load(session)
        state = helper.editable(old)
        next(n for n in state['nodes'] if n['id'] == 'changed')['answer'] = 'TEST remote'
        helper.publish(session, state, old['version'], 'TEST reversed choice')
        run = Run(cwd, width, height, budget=1, replies=[{**SEED, 'found': []}])
        try:
            run.open_shape()
            run.wait('F2 look into the next question')
            assert 'F7 teach-back' not in run.flat()
            run.send(F2, 'Seed draft r1')
            for line in ['Before you confirm', 'Accepted as suggested: 2; chosen otherwise: 3.',
                         'changed: TEST remote — reverses an earlier answer',
                         'delegated: TEST suggested — delegated; accepted exactly as suggested',
                         'Assumed: TEST Markdown — TEST either format works',
                         'accepted: TEST yes — accepted exactly as suggested']:
                run.read_until(line)
            run.wait('F7 teach-back')
            assert len(run.calls()) == 1
            argv = run.calls()[0]['argv']
            prompt = argv[argv.index('--system-prompt') + 1]
            for rule in ['Before drafting in this same step', 'Separate decided', 'one case that must fail']:
                assert rule in prompt, rule
            run.type('TEST confirm only this seed')
            run.send(F3, 'Seed ready: you confirmed r1, and it still matches.')
            saved = helper.load(session)
            origin = next(n for n in saved['nodes'] if n['id'] == saved['origin'])
            assert origin['evidence'][-1]['observed'] == 'Skipped teach-back.'
            assert origin['evidence'][-1]['artifact'] == 'tinkery:teach-back/skipped'
            assert saved['goal'] == GOAL and len(run.calls()) == 1
            assert helper.status(session)['handoff']['verified'] is True
            print(f'Shape confirmation summary + skip passed at {width}x{height}: 1 drafting stub call, no confirmation call')
        finally:
            run.close()


def completion_review_display(width, height, done=False):
    """Drafting review survives saving and appears as guesses above F3, without extra calls."""
    with tempfile.TemporaryDirectory(prefix='tinkery-TEST-review-display-') as directory:
        cwd = Path(directory).resolve()
        session = make(cwd / 'TEST-sessions', question=False)
        review = completion_review(SEED['checks'])
        review['decided'] = ['TEST café scope.\r\nKeep words.  ']
        review['done'] = ['TEST README observed.\r\nNo checks ran.  '] if done else []
        review['failure_cases'][0]['case'] = 'TEST private text appears'
        reply = {**SEED, 'found': [], 'review': review}
        run = Run(cwd, width, height, budget=1, replies=[reply])
        try:
            run.open_shape()
            run.wait('F2 look into the next question')
            run.send(F2, 'Seed draft r1')
            seed = (session / 'seed-contract.md').read_bytes()
            saved = helper.load(session)
            origin = next(n for n in saved['nodes'] if n['id'] == saved['origin'])
            receipt = next(r for r in origin['evidence'] if r.get('artifact') == 'tinkery:review/completed')
            assert json.loads(receipt['observed']) == review, receipt
            assert receipt['checked'] == 'Review r1 · ' + hashlib.sha256(seed).hexdigest(), receipt
            for line in [
                'Drafting step guess — decided: TEST café scope. Keep words.',
                ('Drafting step guess — observed done: TEST README observed. No checks ran.' if done
                 else 'Drafting step guess: Nothing observed done.'),
                'Drafting step guess: If TEST private text appears, Refusals never show private text. must fail.',
            ]:
                run.read_until(line)
                assert run.flat().index(line) < run.flat().index('F3 confirm seed r1'), run.screen.text()
            assert len(run.calls()) == 1 and saved['goal'] == GOAL
            run.read_until('This is not implementation approval.')
            run.type('TEST confirm review')
            run.send(F3, 'Seed ready: you confirmed r1, and it still matches.')
            assert helper.status(session)['handoff']['verified'] is True
            print(f'Shape completion review display {"done" if done else "empty done"} passed at {width}x{height}: 1 stub call')
        finally:
            run.close()


def completion_reviews(width, height):
    """Missing or incomplete review refuses before any seed write; a blocker is a question."""
    for kind in ['missing', 'untyped', 'missing case', 'blocker', 'question']:
        with tempfile.TemporaryDirectory(prefix='tinkery-TEST-completion-review-') as directory:
            cwd = Path(directory).resolve()
            session = make(cwd / 'TEST-sessions', question=False)
            reply = {**SEED, 'found': [], 'review': completion_review(SEED['checks'])}
            if kind == 'missing':
                del reply['review']
            elif kind == 'untyped':
                reply['review'] = 'TEST reviewed it'
            elif kind == 'missing case':
                reply['review']['failure_cases'] = []
            elif kind == 'blocker':
                reply['review']['contradictions'] = ['TEST choices conflict']
            else:
                reply = {**QUESTION, 'question': 'TEST choices conflict; local or synced?'}
            run = Run(cwd, width, height, budget=2, replies=[reply, {**SEED, 'found': []}])
            try:
                run.open_shape()
                run.wait('F2 look into the next question')
                before = files(run.sessions)
                expected = {'missing': 'needs a typed completion review', 'untyped': 'needs a typed completion review',
                            'missing case': 'exactly one failure case', 'blocker': 'Return it as a question',
                            'question': 'TEST choices conflict; local or synced?'}[kind]
                run.send(F2, expected)
                assert len(run.calls()) == 1 and not (session / 'seed-contract.md').exists()
                if kind == 'question':
                    run.type('TEST local')
                    run.send(F2, 'Seed draft r1')
                    assert len(run.calls()) == 2, 'Completion review added a call per draft'
                else:
                    assert files(run.sessions) == before, 'Refused review saved part of a seed'
                print(f'Shape completion review {kind} passed at {width}x{height}: stub replies only')
            finally:
                run.close()


def teach_back(width, height, exhausted=False):
    """Capture is explicit and budget-gated; even reported mismatches leave F3 available."""
    with tempfile.TemporaryDirectory(prefix='tinkery-TEST-teach-back-') as directory:
        cwd = Path(directory).resolve()
        session = make(cwd / 'TEST-sessions', question=False)
        target = cwd / 'TEST-repository'
        target.mkdir()
        mismatch = 'TEST sync is outside the draft.\r\nKeep café mismatch words.  '
        run = Run(cwd, width, height, budget=1 if exhausted else 2,
                  replies=[{**SEED, 'found': []}, {'mismatches': [mismatch]}])
        try:
            run.open_shape()
            run.wait('F2 look into the next question')
            run.send(b'\x1bOS', 'Next: F4 uses this folder')
            run.send(b'/TEST-repository\x1bOS', 'The step reads files in TEST-repository.')
            run.send(F2, 'Seed draft r1')
            run.wait('F7 teach-back')
            run.type('TEST I confirm r1')
            before = files(run.sessions)
            run.send(F7, 'your teach-back')
            run.send(b'cancel me\x1b', 'Teach-back cancelled. Nothing was saved.')
            run.wait('TEST I confirm r1')
            assert files(run.sessions) == before and len(run.calls()) == 1
            run.send(F7, 'your teach-back')
            words = 'TEST build sync.\r\nNo deletes.  '
            run.send(b'\x1b[200~' + words.encode() + b'\x1b[201~', 'No deletes.')
            run.send(F1, 'F7 saves and compares (one model call)')
            run.send(b'\x1b', 'your teach-back')
            assert files(run.sessions) == before and len(run.calls()) == 1
            if exhausted:
                run.send(F7, 'No model calls are left')
                assert files(run.sessions) == before and len(run.calls()) == 1
                run.send(b'\x1b', 'Teach-back cancelled.')
            else:
                run.send(b'\x1b[18;1:2~' + F7, 'Teach-back compared. Mismatches never block F3.')
                run.wait('F3 confirm seed r1')
                run.wait('TEST I confirm r1')
                run.read_until('Teach-back mismatch (guess): TEST sync is outside the draft.')
                saved = helper.load(session)
                origin = next(n for n in saved['nodes'] if n['id'] == saved['origin'])
                assert [r['observed'] for r in origin['evidence'] if r.get('artifact', '').startswith('tinkery:teach-back/')] == [words, mismatch, 'Teach-back compared.']
                assert len(run.calls()) == 2
                assert '--no-tools' in run.calls()[1]['argv'] and '--tools' not in run.calls()[1]['argv']
                assert run.calls()[0]['cwd'] == str(target), run.calls()
                assert run.calls()[1]['cwd'] == str(cwd), run.calls()
                assert run.calls()[1]['input'] == {'kind': 'teach-back',
                    'draft': (session / 'seed-contract.md').read_text(), 'operator_words': words}
            run.read_until('Accepted as suggested: 0; chosen otherwise: 1.')
            os.write(run.master, b'\x1b[6~' * 10)
            run.send(F3, 'Seed ready: you confirmed r1, and it still matches.')
            saved = helper.load(session)
            origin = next(n for n in saved['nodes'] if n['id'] == saved['origin'])
            if exhausted:
                assert origin['evidence'][-1]['observed'] == 'Skipped teach-back.'
            else:
                assert [r['observed'] for r in origin['evidence'] if r.get('artifact', '').startswith('tinkery:teach-back/')] == [words, mismatch, 'Teach-back compared.']
            assert saved['goal'] == GOAL and helper.status(session)['handoff']['verified'] is True
            assert len(run.calls()) == (1 if exhausted else 2)
            assert 'F7 teach-back' not in run.flat()
            print(f'Shape teach-back {"zero budget" if exhausted else "mismatch + F3"} passed at {width}x{height}')
        finally:
            run.close()


def teach_back_retry(width, height):
    """A malformed comparison spends one call, keeps words, and retries only on another key."""
    with tempfile.TemporaryDirectory(prefix='tinkery-TEST-teach-back-retry-') as directory:
        cwd = Path(directory).resolve()
        session = make(cwd / 'TEST-sessions', question=False)
        run = Run(cwd, width, height, budget=3,
                  replies=[{**SEED, 'found': []}, {'mismatches': [42]}, {'mismatches': []}])
        try:
            run.open_shape()
            run.wait('F2 look into the next question')
            run.send(F2, 'Seed draft r1')
            run.type('TEST confirm r1')
            run.send(F7, 'your teach-back')
            run.type('TEST first teach-back')
            run.send(F7, 'The teach-back reply needs a mismatches list.')
            saved = helper.load(session)
            origin = next(n for n in saved['nodes'] if n['id'] == saved['origin'])
            assert [r['observed'] for r in origin['evidence'] if r.get('artifact', '').startswith('tinkery:teach-back/')] == ['TEST first teach-back']
            assert len(run.calls()) == 2
            for parser_text in ['invalid type', 'integer', 'line 1 column']:
                assert parser_text not in run.flat(), run.screen.text()
            run.send(F1, 'Model calls left this run: 1 of 3.')
            run.send(b'\x1b', 'F7 teach-back')
            assert len(run.calls()) == 2, 'A failed comparison retried on its own'
            run.send(F7, 'your teach-back')
            run.type('TEST corrected teach-back')
            run.send(F7, 'Teach-back compared. Mismatches never block F3.')
            run.read_until('Teach-back guess: no mismatches reported.')
            run.read_until('Accepted as suggested: 0; chosen otherwise: 1.')
            os.write(run.master, b'\x1b[6~' * 10)
            run.send(F3, 'Seed ready: you confirmed r1, and it still matches.')
            saved = helper.load(session)
            origin = next(n for n in saved['nodes'] if n['id'] == saved['origin'])
            assert [r['observed'] for r in origin['evidence'] if r.get('artifact', '').startswith('tinkery:teach-back/')] == [
                'TEST first teach-back', 'TEST corrected teach-back', 'Teach-back compared.']
            assert len(run.calls()) == 3 and saved['goal'] == GOAL
            print(f'Shape teach-back failure + explicit retry passed at {width}x{height}: exactly 3 stub calls')
        finally:
            run.close()


if __name__ == '__main__':
    for size in [(100, 30), (80, 24)]:
        completion_review_display(*size)
        completion_review_display(*size, done=True)
        teach_back_retry(*size)
        closing_summary(*size)
        completion_reviews(*size)
        teach_back(*size)
        teach_back(*size, exhausted=True)
        prerequisite_questions(*size)
    for name in ['settled prerequisites', 'unknown prerequisite', 'cyclic prerequisite']:
        case(name)
    for size in [(100, 30), (80, 24)]:
        answer_actions(*size)
    for size in [(100, 30), (80, 24)]:
        typed_findings_review(*size)
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
