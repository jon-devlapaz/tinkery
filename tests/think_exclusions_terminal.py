"""#479 authored reply controls, not recorded replies or proof of prompt compliance."""
import json
import os
from pathlib import Path
import tempfile

from shape_answer_terminal import Run, F2

FIXTURE = Path(__file__).with_name('fixtures') / 'think-exclusions-479.json'
RULE = ('Preserve the strength of an exclusion: "no accounts" excludes including accounts, '
        'even optional ones; "accounts must not be required" excludes requiring them, not offering them.')
HOST = r'''#!/usr/bin/env python3
import json,pathlib,sys
root=pathlib.Path(__file__).parent
request=json.load(sys.stdin)
with (root/'requests.jsonl').open('a') as log:
    log.write(json.dumps(dict(input=request,argv=sys.argv))+'\n')
print('{"decisions":[]}' if request.get('kind')=='decision-review' else (root/'reply.json').read_text())
'''


def journey(case, weakened=False):
    # A deliberately weakened reply must stay weakened: there is no semantic repair heuristic.
    excluded = ['require accounts' if x == 'include accounts' else x for x in case['must_not']] if weakened else case['must_not']
    reply = dict(parts=dict(who='I', outcome='keep a local log', why='I forget past attempts',
                           done_when='I can find a recorded entry', must=case['must'], must_not=excluded),
                 deferred=case['deferred'], questions=[])
    with tempfile.TemporaryDirectory(prefix='tinkery-TEST-exclusions-479-') as directory:
        root = Path(directory)
        run = Run(root, 100, 30, 2, agent=True)
        (root/'pi').write_text(HOST)
        (root/'reply.json').write_text(json.dumps(reply))
        try:
            run.wait('1  think')
            run.send(b'1', "What's on your mind?")
            os.write(run.master, b'\x1b[200~' + case['words'].encode() + b'\x1b[201~')
            run.send(F2, 'Ready for your review.')
            # Read each expected line through the real reading pane, then review from its top.
            lines = [f'Must: {x}.' for x in case['must']] + [f'Must not: {x}.' for x in excluded]
            for line in lines:
                run.read_until(line)
            assert not run.sessions.exists() and not run.records.exists()
            run.choose('Review the goal', 'type confirm')
            for line in lines:
                run.read_until(line)
            for item in case['deferred']:
                run.read_until(item)
            os.write(run.master, b'\x1b[6~' * 30)
            run.wait('type confirm')
            run.send(b'confirm\r', 'Saved.')
            records = list(run.records.glob('*.json'))
            assert len(records) == 1
            record = json.loads(records[0].read_text())
            ledger = json.loads((run.sessions/record['seed_me_session']/'ledger.json').read_text())
            origin = next(n for n in ledger['nodes'] if n['id'] == ledger['origin'])
            context = json.loads(origin['evidence'][-1]['observed'])
            assert record['turns'][0]['text'] == case['words']
            assert context['constraints'] == dict(must=case['must'], must_not=excluded)
            assert context['deferred'] == case['deferred']
            goal = record['goal']
            assert goal == ledger['goal'] == origin['answer']
            assert goal.splitlines() == ['I keep a local log.', 'Why: I forget past attempts.',
                                         'Done when: I can find a recorded entry.', *lines]
            presented = [e['board']['framings'][0]['text'] for e in record['reading_context']['events']
                         if e['kind'] == 'presented']
            assert presented and all(text == goal for text in presented)
            calls = [json.loads(line) for line in (root/'requests.jsonl').read_text().splitlines()]
            assert len(calls) == 2, 'Review and confirmation must not call a model'
            assert calls[0]['input']['sources'][0]['text'] == case['words']
            argv = calls[0]['argv']
            assert RULE in argv[argv.index('--system-prompt')+1], 'Exclusion strength rule was not sent'
            print(json.dumps(dict(case=case['id'], weakened_control=weakened, width=100, height=30,
                                  execution='authored fake reply; zero real calls',
                                  quality='unknown; not model compliance or operator acceptance')))
        finally:
            run.close()


if __name__ == '__main__':
    cases = json.loads(FIXTURE.read_text())['cases']
    for case in cases:
        journey(case)
    for case in cases[:2]:
        journey(case, weakened=True)
