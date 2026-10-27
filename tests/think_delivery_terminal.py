"""Replay #401's recorded omission and correction through keys; zero real model calls."""
import json
import os
from pathlib import Path
import select
import tempfile
import time
from unittest.mock import patch

from shape_answer_terminal import Run, ESC, F1, F2

FIXTURE = Path(__file__).with_name('fixtures') / 'think-delivery-401.json'
READY = 'Ready for your review.'
HOST = '''#!/usr/bin/env python3
import json,pathlib,sys
root=pathlib.Path(__file__).parent
request=json.load(sys.stdin)
with (root/'requests.jsonl').open('a') as log: log.write(json.dumps(request)+'\\n')
if request.get('kind')=='decision-review':
    print('{"decisions":[]}')
else:
    fixture=json.loads((root/'fixture.json').read_text())
    print(json.dumps(fixture['readings'][len(request['sources'])-1]))
'''


def journey(width, height, no_color):
    fixture = json.loads(FIXTURE.read_text())
    added = [line for line in fixture['readings'][1]['parts']['must']
             if line not in fixture['readings'][0]['parts']['must']]
    assert len(added) == 5
    with tempfile.TemporaryDirectory(prefix='tinkery-TEST-delivery-401-') as directory:
        root = Path(directory)
        with patch.dict(os.environ):
            os.environ.pop('NO_COLOR', None)
            if no_color:
                os.environ['NO_COLOR'] = '1'
            run = Run(root, width, height, 4, agent=True)
        (root / 'pi').write_text(HOST)
        (root / 'fixture.json').write_text(FIXTURE.read_text())

        def requests():
            return [json.loads(line) for line in (root / 'requests.jsonl').read_text().splitlines()]

        def no_save():
            assert not run.sessions.exists() and not run.records.exists()

        def copy_board():
            count = len(run.screen.clipboards)
            run.send(F1, 'Copy the reading')
            os.write(run.master, run.screen.pick('Copy the reading'))
            deadline = time.monotonic() + 8
            while time.monotonic() < deadline:
                if select.select([run.master], [], [], .02)[0]:
                    run.screen.feed(os.read(run.master, 65536))
                if len(run.screen.clipboards) > count and not run.screen.synchronized:
                    paper = run.screen.clipboards[-1]
                    goal = paper.split('## I think this is about… / guess\n\n', 1)[1]
                    return paper, goal.split('\n\nSupporting words:', 1)[0]
            raise AssertionError(f'Copy did not arrive:\n{run.screen.text()}')

        def paste(words):
            os.write(run.master, b'\x1b[200~' + words.encode() + b'\x1b[201~')

        def read_right(text):
            """Read wrapped goal lines without interleaving the intact left original."""
            deadline = time.monotonic() + 8
            for _ in range(80):
                right = ' '.join(' '.join(line[width * 48 // 100 + 2:]
                                         for line in run.screen.text().splitlines()).split())
                if text in right and not run.screen.synchronized:
                    return
                before = run.screen.text()
                os.write(run.master, b'\x1b[6~')
                while time.monotonic() < deadline:
                    if select.select([run.master], [], [], .02)[0]:
                        run.screen.feed(os.read(run.master, 65536))
                    if not run.screen.synchronized and run.screen.text() != before:
                        break
                else:
                    break
            raise AssertionError(f'{text!r} not found in review:\n{run.screen.text()}')

        try:
            run.wait('1  think')
            run.send(b'1', "What's on your mind?")
            paste(fixture['words'])
            run.send(F2, READY)
            assert "Nothing I'm unsure about" not in run.flat()
            assert run.flat().count(READY) == 1
            paper, first_goal = copy_board()
            assert fixture['words'] in paper
            for line in added:
                assert line not in first_goal, 'The replay must retain the observed omission.'
            for other_goal in fixture['readings'][0]['other_goals']:
                assert f'also in your dump: {other_goal}' in paper
            first_requests = requests()
            assert len(first_requests) == 2
            assert first_requests[0]['sources'][0]['text'] == fixture['words']
            no_save()

            run.send(F1, 'Right now')
            assert READY in run.flat() and "Nothing I'm unsure" not in run.flat()
            run.send(b'\x1b', READY)
            run.choose('Review the goal', 'type confirm')
            assert "Operator's original request" in run.flat()
            read_right('Must not: Provide financial/legal advice.')
            no_save()
            run.send(b'\x1b', READY)
            assert requests() == first_requests, 'Help, copying and review must not call a model.'

            paste(fixture['correction'])
            run.send(F2, READY)
            read_right('Must not: Change tool configuration.')
            paper, corrected_goal = copy_board()
            assert fixture['words'] in paper and fixture['correction'] in paper
            for line in added:
                assert f'Must: {line}' in corrected_goal
            for line in fixture['readings'][0]['parts']['must_not']:
                assert f'Must not: {line}' in corrected_goal
            sent = requests()
            assert len(sent) == 4
            assert sent[2]['sources'][0]['text'] == fixture['words']
            assert sent[2]['sources'][1]['text'] == fixture['correction']
            no_save()

            run.choose('Review the goal', 'type confirm')
            read_right('Must not: Change tool configuration.')
            no_save()
            run.send(b'confirm\r', 'Saved.')
            record_files = list(run.records.glob('*.json'))
            assert len(record_files) == 1
            record = json.loads(record_files[0].read_text())
            session = run.sessions / record['seed_me_session']
            ledger = json.loads((session / 'ledger.json').read_text())
            assert record['turns'][0]['text'] == fixture['words']
            assert record['turns'][1]['answer'] == fixture['correction']
            assert record['goal'] == ledger['goal'] == corrected_goal
            assert record['goal_confirmation']['declaration'] == 'declared: agent'
            assert ledger['operator'] == 'agent'
            assert not (session / 'seed-contract.md').exists()
            assert requests() == sent, 'Confirmation must not start another model call.'
            run.send(ESC, '1  think')
            print(json.dumps(dict(width=width, height=height, no_color=no_color,
                                  provider='frozen candidates; zero real calls',
                                  journey='omitted constraints/review/correction/exact goal confirmation',
                                  acceptance='automated replay, not human or model-quality acceptance')))
        finally:
            run.close()


if __name__ == '__main__':
    for size in [(80, 24), (140, 42)]:
        for mono in [False, True]:
            journey(*size, mono)
