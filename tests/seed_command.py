"""`tinkery seed new|show|step` (#845): a goal to a verified seed with no screen and no keystrokes.

A stub Pi, real Rust ledger sessions in TEST roots, no model calls. One journey runs human and agent
declared; short cases cover the refusals that keep each command one explicit, budgeted action.
"""
import hashlib, json, os, tempfile
from pathlib import Path

import session_fixture as helper
import test_launch as subprocess
from shape_answer_terminal import BINARY, GOAL, HOST, QUESTION, SEED


class Run:
    def __init__(self, directory, replies, budget=4, agent=False):
        self.cwd = Path(directory).resolve()
        host = self.cwd / 'pi'
        host.write_text(HOST)
        host.chmod(0o755)
        # The stub's citations: README.md:3 for the question, src/main.rs:12 for the seed.
        (self.cwd / 'README.md').write_text('# TEST\n\nLocal only.\n')
        (self.cwd / 'src').mkdir()
        (self.cwd / 'src/main.rs').write_text('\n' * 11 + 'TEST refusal path\n')
        (self.cwd / 'shape-replies.json').write_text(json.dumps(list(replies)))
        (self.cwd / 'goal.txt').write_text(GOAL)
        self.sessions = self.cwd / 'TEST-sessions'
        self.options = ['--seed-session-root', str(self.sessions), '--record-root', str(self.cwd / 'TEST-records')]
        self.drafting = ['--shape-pi', '--model', 'test/model', '--pi-command', str(host), '--shape-budget', str(budget)]
        self.options += ['--operator', 'agent' if agent else 'human']

    def seed(self, *args, drafting=True, options=None, home=None):
        """One command with no terminal: stdin is closed and both outputs are pipes."""
        env = subprocess.environment({**os.environ, 'TINKERY_TEST_DIR': str(self.cwd)})
        if home:
            env['HOME'] = str(home)
        options = self.options if options is None else options
        return subprocess.run([str(BINARY), *options, *(self.drafting if drafting else []), 'seed', *args],
                              cwd=self.cwd, env=env, stdin=subprocess.DEVNULL, stdout=subprocess.PIPE,
                              stderr=subprocess.PIPE, timeout=60)

    def ok(self, *args, **kwargs):
        result = self.seed(*args, **kwargs)
        assert result.returncode == 0, (args, result.stderr.decode())
        return result.stdout.decode()

    def refused(self, why, *args, **kwargs):
        result = self.seed(*args, **kwargs)
        assert result.returncode != 0 and not result.stdout, (args, result.stdout.decode())
        assert why in result.stderr.decode(), result.stderr.decode()

    def calls(self):
        path = self.cwd / 'shape-calls.jsonl'
        return [json.loads(line) for line in path.read_text().splitlines()] if path.exists() else []

    def files(self):
        return {str(p): p.read_bytes() for p in self.sessions.rglob('*') if p.is_file() and p.name != '.writer.lock'}


def journey(agent):
    with tempfile.TemporaryDirectory(prefix='tinkery-TEST-seed-command-') as directory:
        # #931: the draft chose a default instead of asking, so the operator must see it listed.
        assumed = {'text': 'TEST refusals use plain text', 'why': 'TEST either format shows the same refusal'}
        run = Run(directory, [QUESTION, {**SEED, 'assumed': [assumed]}], agent=agent)
        session = run.ok('new', 'goal.txt', 'TEST issue 845', drafting=False).strip()
        ledger = helper.load(Path(session))
        goal = next(node for node in ledger['nodes'] if node['id'] == 'goal')
        assert ledger['goal'] == GOAL and goal['answer'] == GOAL, 'The goal words changed'
        assert any(e['observed'] == GOAL for e in goal['evidence']), 'The original words were not kept'
        assert 'TEST issue 845' in goal['authority_source']
        assert 'nobody typed confirm' in goal['authority_source'], 'A command save claimed a typed confirm'
        assert (goal['authority'] == 'user') != agent, 'An agent save was recorded as the operator'
        assert ledger['draft']['outcome'] == 'Refusals never show private text.'

        shown = json.loads(run.ok('show', session, drafting=False))
        assert shown['goal'] == GOAL and shown['now']['stage'] == 'waiting'

        # Both probes are asked without a model; neither has a suggestion to delegate.
        asked = json.loads(run.ok('step', session, 'look'))['now']
        assert asked['stage'] == 'question' and asked['id'] == 'probe-real-sample' and asked['suggestion'] is None
        run.refused('no suggestion to delegate', 'step', session, 'delegate')
        asked = json.loads(run.ok('step', session, 'answer', 'TEST no sample: fixture'))['now']
        assert asked['id'] == 'probe-pre-mortem'
        assert run.calls() == [], 'A probe called a model'

        # The first model step asks a grounded question; delegation accepts its suggestion.
        asked = json.loads(run.ok('step', session, 'answer', 'TEST skip'))['now']
        assert asked['question'] == QUESTION['question'] and asked['suggestion'] == QUESTION['suggestion']
        assert asked['found'], 'The finding behind the question is missing'
        draft = json.loads(run.ok('step', session, 'delegate'))['now']
        assert draft['stage'] == 'draft' and draft['revision'] == 'r1'
        seed = (Path(session) / 'seed-contract.md').read_text()
        assert draft['text'] == seed and draft['sha256'] == hashlib.sha256(seed.encode()).hexdigest()
        assert f"- {assumed['text']} (why: {assumed['why']})" in seed, 'The drafted default is not in the Tink'
        listed = json.loads(run.ok('show', session, drafting=False))
        assert listed['open'] == [], 'A draft still has a gray item open'
        decided = listed['decided_without_you']
        assert {'what': assumed['text'], 'why': assumed['why'], 'authority': 'assumed'} in decided, decided
        delegated = [d for d in decided if d['what'] == QUESTION['label']]
        assert delegated and delegated[0]['authority'] == 'delegated', decided
        assert all(d['authority'] != 'user' for d in decided), 'An operator answer was listed as decided without them'
        # #931: a question Shape asked and nobody but the operator may settle stays gray.
        assert [g['label'] for g in listed['gray']] == [QUESTION['label']], listed['gray']
        shown = listed['now']
        assert shown['risks'] == draft['risks']
        cases = ['Drafting step guess:', 'Must fail: TEST wrong output must fail',
                 '  Check: Refusals never show private text.']
        start = shown['risks'].index(cases[0])
        assert shown['risks'][start:start + len(cases)] == cases
        assert len(run.calls()) == 2

        # Confirming names the bytes that were read; anything else confirms nothing.
        before = run.files()
        run.refused('not the current draft', 'step', session, 'confirm', 'a' * 64)
        run.refused('usage:', 'step', session, 'confirm')
        run.refused('does not fit', 'step', session, 'look')
        assert run.files() == before, 'A refused command wrote to the session'
        # Confirming calls no model, so it needs no drafting options; the record names the command.
        ready = json.loads(run.ok('step', session, 'confirm', draft['sha256'], drafting=False))['now']
        assert ready == {'stage': 'ready', 'revision': 'r1', 'ended': False}
        confirmed = json.loads((Path(session) / 'seed-confirmation.json').read_text())
        assert confirmed['source'] == f'tinkery seed step confirm {draft["sha256"]}', confirmed
        assert helper.status(Path(session))['handoff']['verified']
        assert len(run.calls()) == 2, 'Confirming called a model'
        print(f'seed command journey passed: {"agent" if agent else "human"}, 6 commands, 2 stub calls, decided-without-you listed, verified seed')


def no_gray():
    """A draft that asked no model question has nothing gray; the fixed probes never count."""
    with tempfile.TemporaryDirectory(prefix='tinkery-TEST-seed-command-') as directory:
        run = Run(directory, [SEED], agent=True)
        session = run.ok('new', 'goal.txt', drafting=False).strip()
        run.ok('step', session, 'look')
        run.ok('step', session, 'answer', 'TEST no sample: fixture')
        draft = json.loads(run.ok('step', session, 'answer', 'TEST skip'))
        assert draft['now']['stage'] == 'draft', draft['now']
        assert draft['gray'] == [], draft['gray']
        assert len(draft['decided_without_you']) == 2, 'The two probe answers are not listed'
    with tempfile.TemporaryDirectory(prefix='tinkery-TEST-seed-command-') as directory:
        # A question the operator answered in their own words is settled, so it is not gray.
        run = Run(directory, [QUESTION, SEED])
        session = run.ok('new', 'goal.txt', drafting=False).strip()
        run.ok('step', session, 'look')
        run.ok('step', session, 'answer', 'TEST no sample: fixture')
        assert json.loads(run.ok('step', session, 'answer', 'TEST skip'))['now']['stage'] == 'question'
        draft = json.loads(run.ok('step', session, 'answer', 'TEST plain text refusals'))
        assert draft['now']['stage'] == 'draft', draft['now']
        assert draft['gray'] == [], draft['gray']
        print('seed command no-gray passed: probes only and an operator answer, 1 and 2 stub calls, empty gray')


def relayed():
    """#914: the operator's words relayed by an agent are their own answer, so nothing stays gray."""
    with tempfile.TemporaryDirectory(prefix='tinkery-TEST-seed-command-') as directory:
        run = Run(directory, [QUESTION, SEED], agent=True)
        relay = run.cwd / 'TEST-relay.json'
        relay.write_text(json.dumps({'words': 'TEST plain text refusals', 'where': 'TEST project thread'}))
        bad = run.cwd / 'TEST-bad-relay.json'
        bad.write_text(json.dumps({'words': 'TEST plain text refusals'}))
        session = run.ok('new', 'goal.txt', drafting=False).strip()
        run.ok('step', session, 'look')
        run.ok('step', session, 'answer', 'TEST no sample: fixture')
        assert json.loads(run.ok('step', session, 'answer', 'TEST skip'))['now']['stage'] == 'question'
        before = run.files()
        run.refused('--relayed must name', 'step', session, 'answer', '--relayed', str(bad))
        assert run.files() == before, 'A malformed relay wrote to the session'
        draft = json.loads(run.ok('step', session, 'answer', '--relayed', str(relay)))
        assert draft['now']['stage'] == 'draft', draft['now']
        assert draft['gray'] == [], draft['gray']
        assert all(d['what'] != QUESTION['label'] for d in draft['decided_without_you']), draft['decided_without_you']
        node = next(n for n in helper.load(Path(session))['nodes'] if n.get('label') == QUESTION['label'])
        assert node['answer'] == 'TEST plain text refusals' and node['authority'] == 'user', node
        assert 'relayed by the agent from TEST project thread' in node['authority_source'], node
        assert 'origin unverified' in node['authority_source'], node
        digest = hashlib.sha256(relay.read_bytes()).hexdigest()
        assert any(digest in e.get('observed', '') for e in node['evidence']), 'The relay file is not named'
    with tempfile.TemporaryDirectory(prefix='tinkery-TEST-seed-command-') as directory:
        # A human at the terminal types their own words; a relay is only for an agent.
        run = Run(directory, [QUESTION, SEED])
        relay = run.cwd / 'TEST-relay.json'
        relay.write_text(json.dumps({'words': 'TEST words', 'where': 'TEST thread'}))
        session = run.ok('new', 'goal.txt', drafting=False).strip()
        run.ok('step', session, 'look')
        run.ok('step', session, 'answer', 'TEST no sample: fixture')
        run.ok('step', session, 'answer', 'TEST skip')
        before = run.files()
        run.refused('launch with --operator agent', 'step', session, 'answer', '--relayed', str(relay))
        assert run.files() == before, 'A human relay wrote to the session'
    print('seed command relayed passed: relayed words settle as the operator, clear gray, name the file; human and malformed relays refused')


def refusals():
    with tempfile.TemporaryDirectory(prefix='tinkery-TEST-seed-command-') as directory:
        run = Run(directory, [QUESTION], budget=0)
        (run.cwd / 'empty.txt').write_text(' \n')
        run.refused('The goal is empty', 'new', 'empty.txt', drafting=False)
        run.refused('Cannot read the goal file', 'new', 'missing.txt', drafting=False)
        # No screen shows who is acting, so an undeclared write is refused, and an option is no origin.
        undeclared = run.options[:-2]
        for args in [('new', 'goal.txt'), ('step', 'TEST-missing', 'look')]:
            result = run.seed(*args, options=undeclared)
            assert result.returncode != 0 and b'need --operator' in result.stderr, result.stderr.decode()
        run.refused('usage:', 'new', 'goal.txt', '--operator', drafting=False)
        assert not run.sessions.exists(), 'A refused goal created a session'
        session = run.ok('new', 'goal.txt', drafting=False).strip()
        run.refused('This step needs drafting', 'step', session, 'look', drafting=False)
        # `show` only reads: with no options and a legacy sessions folder in HOME, it moves nothing.
        legacy = run.cwd / 'TEST-home/.local/share/seed-me/sessions'
        legacy.mkdir(parents=True)
        result = run.seed('show', session, drafting=False, options=[], home=run.cwd / 'TEST-home')
        assert result.returncode == 0 and json.loads(result.stdout)['goal'] == GOAL, result.stderr.decode()
        assert legacy.is_dir() and not (run.cwd / 'TEST-home/.local/share/tinkery').exists(), 'show changed HOME'
        # The probes need no model, so a zero allowance still asks them, then stops at the first call.
        run.ok('step', session, 'look')
        before = run.files()
        run.refused('No drafting runs are left', 'step', session, 'answer', 'TEST no sample')
        run.refused('The answer is empty', 'step', session, 'answer', ' ')
        assert run.files() == before and run.calls() == [], 'A refused step wrote or called a model'
        print('seed command refusals passed: empty goal, undeclared operator, no drafting, zero budget, empty answer; zero stub calls')


if __name__ == '__main__':
    for agent in [False, True]:
        journey(agent)
    no_gray()
    relayed()
    refusals()
