"""`tinkery seed trace` reads one goal's records from Think to Taste, in order, and writes nothing.

Synthetic TEST records in a temporary root: a real Rust ledger session, a stub Think record, a stub
run folder and a stub Pi session file. No model calls. Three journeys: one full run, a retry of it,
and a run whose transcripts are missing. The reader must see the operator's words byte for byte,
transcripts only as a path and SHA-256, and each missing record as an `unknown` line.
"""
import hashlib
import json
from pathlib import Path
import tempfile

import session_fixture as helper
import test_launch as subprocess
from shape_terminal import make, seed

REPO = Path(__file__).resolve().parents[1]
BINARY = REPO / 'target/debug/tinkery'
GOAL = 'TEST one trace per goal.\nDone when: TEST a reader sees it in order.'
WORDS = '  TEST café\tkeep THIS exactly  \n'
VERDICT = 'Reject: TEST the sounds are still missing  '
SECRET = 'sk-TEST-0000-never-print-this-secret'
RUN_ID = '00000000-0000-4000-8000-000000000701'
HEAD = 'c' * 40


def tree(root):
    return {str(p): p.read_bytes() for p in root.rglob('*') if p.is_file()}


def think_record(records, session, checkout):
    ledger = helper.load(session)
    records.mkdir(parents=True, exist_ok=True)
    record = {'schema': 3, 'tinkery': 'TEST', 'origin': 'simulated', 'harness': None, 'model': None,
              'thinking': None, 'prompt_hash': None, 'started': '2026-11-28T01:00:00Z',
              'confirmed': '2026-11-28T01:05:00Z', 'seed_me_session': session.name,
              'seed_me_session_path': str(session), 'seed_me_session_id': ledger['session_id'],
              'repo': str(checkout), 'checks': [], 'goal': ledger['goal'], 'turns': [
                  {'at': '2026-11-28T01:00:00Z', 'kind': 'words', 'source': 1, 'text': WORDS},
                  {'at': '2026-11-28T01:01:00Z', 'kind': 'asked', 'id': 'q1', 'part': None,
                   'question': 'TEST who reads it?'},
                  {'at': '2026-11-28T01:02:00Z', 'kind': 'answered', 'id': 'q1', 'question': 'TEST who reads it?',
                   'source': 2, 'answer': 'TEST agents first'}]}
    (records / f'2026-11-28T01-05-00Z_{session.name}.json').write_text(json.dumps(record))


def run_folder(checkout, session, transcripts, slug='TEST-run', run_id=RUN_ID, failed=True):
    """A run built from `session`: a gate, a relayed start, one worker step, verify, review, PR, Taste."""
    run = checkout / 'runs' / slug
    (run / 'decisions').mkdir(parents=True)
    source = session / 'seed-contract.md'
    (run / 'run.json').write_text(json.dumps({'schema': 1, 'run_id': run_id, 'seed_contract': {
        'path': f'runs/{slug}/seed-contract.md', 'source': str(source),
        'sha256': hashlib.sha256(source.read_bytes()).hexdigest()}}))
    (run / 'decisions/3-1764291600000000000-aa.json').write_text(json.dumps({
        'decision': 'approved', 'reviewer': 'TEST reviewer', 'source': 'TEST gate', 'reason': 'TEST reason'}))
    (run / 'worker-authorization.jsonl').write_text(json.dumps({
        'schema': 1, 'operator': 'agent', 'authority': 'operator, relayed', 'authorization': {'words': 'TEST go'},
        'action': 'start', 'run_id': run_id, 'step': 'step-01', 'checkout': str(checkout)}) + '\n')
    steps = run / 'steps'
    steps.mkdir()
    (steps / 'step-01.json').write_text(json.dumps({
        'schema': 1, 'run_id': run_id, 'state': 'finished', 'model': 'test/model', 'session': 'TEST-pi-session',
        'prepared_ns': 1764291700000000000, 'max_responses': 40}))
    if transcripts:
        pi = checkout.parent / 'TEST-pi-sessions'
        pi.mkdir(exist_ok=True)
        (pi / '2026-11-28_TEST-pi-session.jsonl').write_text(
            json.dumps({'type': 'session', 'id': 'TEST-pi-session'}) + '\n'
            + json.dumps({'type': 'message', 'message': {'role': 'tool', 'content': f'export KEY={SECRET}'}}) + '\n')
        # Bytes that are not UTF-8 after the header: the reader reads only the first line.
        with (pi / '2026-11-28_TEST-pi-session.jsonl').open('ab') as transcript:
            transcript.write(b'\xff\xfe TEST binary tool output\n')
        (steps / 'step-01.session-dir').write_text(str(pi))
    output = run / '04-test/output'
    output.mkdir(parents=True)
    (output / 'verification.json').write_text(json.dumps(
        {'result': 'failed', 'error': 'TEST settled seed check failed', 'candidate': {'head': HEAD},
         'seed_checks': {'keep-local': 'pending'}} if failed else
        {'result': 'passed', 'candidate': {'head': HEAD}, 'seed_checks': {'keep-local': 'attested'}}))
    (output / 'test-log.md').write_text('TEST log\n')
    (run / '05-deploy/output').mkdir(parents=True)
    (run / '05-deploy/output/REVIEW-findings.md').write_text('Round: 1\nTEST finding\n')
    (run / 'pr.json').write_text(json.dumps({'url': 'https://github.com/TEST/repo/pull/1', 'candidate': HEAD,
                                             'time_ns': 1764291800000000000}))
    (run / 'labels.jsonl').write_text(json.dumps({
        'kind': 'rejected', 'words': VERDICT, 'reviewer': 'TEST operator', 'source': f'taste@{HEAD}',
        'consent': 'TEST', 'time_ns': 1764291900000000000}, ensure_ascii=False) + '\n')
    return run


def trace(session, root, *options):
    result = subprocess.run([str(BINARY), *map(str, options), 'seed', 'trace', str(session)], cwd=root,
                            env=subprocess.environment(), capture_output=True, text=True)
    assert result.returncode == 0, result.stderr
    return result.stdout, [json.loads(line) for line in result.stdout.splitlines()]


def steps(lines):
    return [(line['kind'], line['stage'], line['name']) for line in lines]


def full_run():
    with tempfile.TemporaryDirectory(prefix='tinkery-TEST-trace-') as directory:
        root = Path(directory).resolve()
        checkout, records = root / 'TEST-checkout', root / 'TEST-records'
        session = make(root / 'TEST-sessions', 'ready', goal=GOAL, question=False)
        seed(session, confirm=True)
        think_record(records, session, checkout)
        run_folder(checkout, session, transcripts=True)
        before = tree(root)
        text, lines = trace(session, root, '--record-root', records)
        assert tree(root) == before, 'seed trace changed a file'
        assert {line['session'] for line in lines} == {helper.load(session)['session_id']}
        assert steps(lines) == [
            ('event', 'goal', 'session'),
            ('event', 'think', 'record'), ('event', 'think', 'words'), ('event', 'think', 'asked'),
            ('event', 'think', 'answered'), ('unknown', 'think', 'model transcript'),
            ('event', 'shape', 'ledger'), ('event', 'shape', 'node'), ('event', 'shape', 'node'),
            ('unknown', 'shape', 'model transcript'),
            ('event', 'confirm', 'seed confirmed'),
            ('event', 'run', 'run'), ('event', 'gate', 'decision'), ('event', 'worker', 'authorization'),
            ('event', 'worker', 'step'), ('event', 'worker', 'transcript'), ('unknown', 'worker', 'driver session'),
            ('event', 'verify', 'verification'), ('score', 'verify', 'check'),
            ('event', 'review', 'findings'), ('event', 'pr', 'pull request'), ('score', 'taste', 'verdict'),
        ], steps(lines)
        assert lines[0]['goal'] == GOAL and lines[0]['format'] == 'tinkery.trace/1'
        assert lines[2]['text'] == WORDS and lines[2]['actor'] == 'operator', lines[2]
        assert lines[4]['answer'] == 'TEST agents first' and lines[3]['actor'] == 'tinkery'
        verdict = lines[-1]
        assert (verdict['words'], verdict['value'], verdict['commit']) == (VERDICT, 'rejected', HEAD), verdict
        # Open coding: the first failure is readable from the trace alone.
        failure = next(line for line in lines if line.get('result') == 'failed')
        assert (failure['stage'], failure['error'], failure['run']) == (
            'verify', 'TEST settled seed check failed', 'TEST-run'), failure
        transcript = next(line for line in lines if line['name'] == 'transcript')
        path = Path(transcript['source']['path'])
        assert transcript['source']['sha256'] == hashlib.sha256(path.read_bytes()).hexdigest()
        assert SECRET.encode() in path.read_bytes() and SECRET not in text, 'a transcript reached the trace'
        assert next(line for line in lines if line['name'] == 'run')['seed_matches'] is True
        # The Think record's repository is the default checkout.
        assert trace(session, root, '--record-root', records, '--sdlc-root', checkout)[1] == lines
        # A changed seed after intake still joins by its path, and says the bytes changed.
        (session / 'seed-contract.md').write_text((session / 'seed-contract.md').read_text() + 'TEST edit\n')
        assert next(line for line in trace(session, root, '--record-root', records)[1]
                    if line['name'] == 'run')['seed_matches'] is False
        # An unreadable seed is an unknown match, never a silent false.
        seed_file = session / 'seed-contract.md'
        seed_file.unlink()
        seed_file.mkdir()
        unreadable = trace(session, root, '--record-root', records)[1]
        assert next(line for line in unreadable if line['name'] == 'run')['seed_matches'] is None
        assert [line['kind'] for line in unreadable if line['name'] == 'seed match'] == ['unknown'], unreadable
    print(f'seed trace full run: {len(lines)} lines in order, verbatim words, transcript by hash only')
    return lines


def retry():
    with tempfile.TemporaryDirectory(prefix='tinkery-TEST-trace-retry-') as directory:
        root = Path(directory).resolve()
        checkout, records = root / 'TEST-checkout', root / 'TEST-records'
        first = make(root / 'TEST-sessions', 'ready', goal=GOAL, question=False)
        seed(first, confirm=True)
        run_folder(checkout, first, transcripts=True)
        for run_id, found in [(RUN_ID, True), ('00000000-0000-4000-8000-000000000799', False)]:
            second = make(root / 'TEST-sessions', 'ready', goal=GOAL, question=False)
            old = helper.load(second)
            state = helper.editable(old)
            state['nodes'][0]['evidence'] = [{'checked': 'Taste rejection', 'at': '2026-11-28T02:00:00+00:00',
                                              'observed': VERDICT, 'artifact': run_id}]
            helper.publish(second, state, old['version'], 'TEST retry fixture')
            _, lines = trace(second, root, '--record-root', records, '--sdlc-root', checkout)
            rejected = next(line for line in lines if line['name'] == 'rejected')
            assert (rejected['stage'], rejected['words'], rejected['run_id']) == ('retry', VERDICT, run_id), rejected
            prior = next(line for line in lines if line['name'] == 'prior session')
            if found:
                assert (prior['kind'], prior['prior_session']) == ('event', str(first)), prior
            else:
                assert prior['kind'] == 'unknown' and run_id in prior['reason'], prior
            assert ('unknown', 'run', 'runs') in steps(lines), 'a retry with no run of its own must say so'
    print('seed trace retry: the rejection words and the prior session, or an unknown prior run')


def missing_transcripts():
    with tempfile.TemporaryDirectory(prefix='tinkery-TEST-trace-missing-') as directory:
        root = Path(directory).resolve()
        checkout = root / 'TEST-checkout'
        session = make(root / 'TEST-sessions', 'ready', goal=GOAL, question=False)
        seed(session, confirm=True)
        run_folder(checkout, session, transcripts=False, failed=False)
        _, lines = trace(session, root, '--record-root', root / 'TEST-no-records', '--sdlc-root', checkout)
        unknown = [(line['stage'], line['name']) for line in lines if line['kind'] == 'unknown']
        assert unknown == [('think', 'record'), ('think', 'model transcript'), ('shape', 'model transcript'),
                           ('worker', 'transcript'), ('worker', 'driver session')], unknown
        assert all(line['reason'] for line in lines if line['kind'] == 'unknown')
        _, alone = trace(session, root, '--record-root', root / 'TEST-no-records')
        assert alone[-1]['kind'] == 'unknown' and '--sdlc-root' in alone[-1]['reason'], alone[-1]
        refused = subprocess.run([str(BINARY), 'seed', 'trace', str(session), '--sdlc-root'], cwd=root,
                                 env=subprocess.environment(), capture_output=True, text=True)
        assert refused.returncode != 0 and 'seed trace DIR' in refused.stderr, refused.stderr
    print('seed trace missing records: each missing transcript and record is an unknown line with its reason')
    return unknown


if __name__ == '__main__':
    full_run()
    retry()
    missing_transcripts()
