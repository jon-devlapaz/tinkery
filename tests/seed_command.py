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
        # The second reply drops the goal's Done when line; the third and fourth keep it.
        dropped = {**SEED, 'checks': [{**SEED['checks'][0], 'statement': 'Refusals are calm.'}]}
        again = {**SEED, 'found': [{**SEED['found'][0], 'id': 'read-refusal-again'}]}
        run = Run(directory, [QUESTION, dropped, SEED, again], agent=agent)
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
        assert shown['must_keep'] == ['Refusals never show private text.'], shown['must_keep']

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
        # A draft that drops the goal's Done when line is no draft; the next look keeps it.
        run.refused('keep this complete Done when line unchanged', 'step', session, 'delegate')
        assert not (Path(session) / 'seed-contract.md').exists(), 'A draft without the goal check was saved'
        run.refused('does not fit', 'step', session, 'revise', 'TEST too early')
        first = json.loads(run.ok('step', session, 'look'))['now']
        assert first['stage'] == 'draft' and first['revision'] == 'r1'

        # Revising records the words and redrafts; it never confirms, and the old bytes stop counting.
        before = run.files()
        run.refused('The words are empty', 'step', session, 'revise', ' ')
        run.refused('This step needs drafting', 'step', session, 'revise', 'TEST change', drafting=False)
        run.drafting[-1] = '0'
        run.refused('No drafting runs are left', 'step', session, 'revise', 'TEST change')
        run.drafting[-1] = '4'
        assert run.files() == before, 'A refused revision wrote to the session'
        draft = json.loads(run.ok('step', session, 'revise', 'TEST name the refusal path'))
        assert draft['must_keep'] == shown['must_keep']
        draft = draft['now']
        assert draft['stage'] == 'draft' and draft['revision'] == 'r2', draft
        change = next(n for n in helper.load(Path(session))['nodes'] if n['id'] == 'change-to-r1')
        assert change['answer'] == 'TEST name the refusal path' and (change['authority'] == 'user') != agent
        seed = (Path(session) / 'seed-contract.md').read_text()
        assert draft['text'] == seed and draft['sha256'] == hashlib.sha256(seed.encode()).hexdigest()
        assert 'Refusals never show private text.' in seed
        assert len(run.calls()) == 4

        # Confirming names the bytes that were read; anything else confirms nothing.
        before = run.files()
        run.refused('not the current draft', 'step', session, 'confirm', 'a' * 64)
        run.refused('usage:', 'step', session, 'confirm')
        run.refused('does not fit', 'step', session, 'look')
        assert run.files() == before, 'A refused command wrote to the session'
        # Confirming calls no model, so it needs no drafting options; the record names the command.
        ready = json.loads(run.ok('step', session, 'confirm', draft['sha256'], drafting=False))['now']
        assert ready == {'stage': 'ready', 'revision': 'r2', 'ended': False}
        confirmed = json.loads((Path(session) / 'seed-confirmation.json').read_text())
        assert confirmed['source'] == f'tinkery seed step confirm {draft["sha256"]}', confirmed
        assert helper.status(Path(session))['handoff']['verified']
        assert len(run.calls()) == 4, 'Confirming called a model'
        print(f'seed command journey passed: {"agent" if agent else "human"}, 9 commands, 4 stub calls, verified seed')


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
        created = run.seed('new', 'goal.txt', drafting=False)
        assert created.returncode == 0 and not created.stderr, created.stderr.decode()
        session = created.stdout.decode().strip()
        # A goal with no Done when lines still saves, and says the checks will be the model's.
        (run.cwd / 'plain.txt').write_text('Fix: TEST one sentence.')
        plain = run.seed('new', 'plain.txt', drafting=False)
        assert plain.returncode == 0 and b'states no Done when lines' in plain.stderr, plain.stderr.decode()
        assert json.loads(run.ok('show', plain.stdout.decode().strip(), drafting=False))['must_keep'] == []
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
        run.refused('The words are empty', 'step', session, 'answer', ' ')
        assert run.files() == before and run.calls() == [], 'A refused step wrote or called a model'
        print('seed command refusals passed: empty goal, undeclared operator, no drafting, zero budget, empty answer; zero stub calls')


if __name__ == '__main__':
    for agent in [False, True]:
        journey(agent)
    refusals()
