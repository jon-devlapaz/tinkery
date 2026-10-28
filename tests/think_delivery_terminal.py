"""Replay #401's recorded omission and correction through keys; zero real model calls."""
import json
from datetime import datetime, timezone
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
            run.read_until(READY)  # The numbered rows come first; the state follows.
            assert "Nothing I'm unsure" not in run.flat(), run.screen.text()
            run.send(b'\x1b', READY)
            run.choose('Review the goal', 'You are confirming')
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

            run.choose('Review the goal', 'You are confirming')
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
            events = record['reading_context']['events']
            assert [event['kind'] for event in events] == [
                'submitted', 'presented', 'submitted', 'presented']
            assert events[0]['source'] == 1 and events[2]['source'] == 2
            assert events[1]['board']['other_goals'] == fixture['readings'][0]['other_goals']
            assert events[3]['board']['other_goals'] == []
            assert events[1]['board']['framings'][0]['text'] == first_goal
            assert events[3]['board']['framings'][0]['text'] == corrected_goal
            for event in events:
                if event['kind'] == 'presented':
                    assert event['at'].endswith('Z')
            assert 'tinkery_source' in record  # Unknown for an ordinary unstamped build.
            assert record['goal_confirmation']['declaration'] == 'declared: agent'
            assert ledger['operator'] == 'agent'
            assert not (session / 'seed-contract.md').exists()
            assert requests() == sent, 'Confirmation must not start another model call.'
            run.send(ESC, '1  think')
            before_history = {str(p): (p.read_bytes(), p.stat().st_mtime_ns)
                              for p in root.rglob('*') if p.is_file()}
            run.send(b'2', 'enter open')
            run.send(b'\r', 's shape')
            run.send(b'r', 'Recorded sequence')
            first_history = run.screen.text()
            sequence = ['Original words 1', 'Reading 1 shown',
                        'Answer 2 · after reading 1', 'Reading 2 shown']
            positions = [first_history.index(label) for label in sequence]
            assert positions == sorted(positions), first_history
            for event in events:
                if event['kind'] == 'presented':
                    shown = datetime.fromisoformat(event['at'].replace('Z', '+00:00')).astimezone(timezone.utc)
                    assert shown.strftime('%H:%M:%S') in first_history
                    assert event['at'] not in first_history
            assert 'Shown means it reached the screen, not that it was read.' in first_history
            assert 'framings' not in first_history and 'structure-spans-history' not in first_history
            # Walk one row at a time so the full answer is checked through the real viewport.
            visible = '\n'.join(first_history.splitlines()[5:height - 1])
            previous = first_history
            for _ in range(180):
                os.write(run.master, b'\x1b[B')
                deadline = time.monotonic() + .5
                while time.monotonic() < deadline:
                    if select.select([run.master], [], [], .02)[0]:
                        run.screen.feed(os.read(run.master, 65536))
                    if not run.screen.synchronized and run.screen.text() != previous:
                        break
                current = run.screen.text()
                if current == previous:
                    break
                visible += current.splitlines()[height - 2]
                previous = current
            else:
                raise AssertionError('History did not reach its end')
            compact = lambda text: ''.join(text.split())
            assert 'Answer 2' in visible
            headers = ['Answer 2', "Readings · Tinkery's guesses",
                       'Reading 1 · shown ', 'Reading 2 · shown ', 'Confirmed goal']
            assert [visible.index(h) for h in headers] == sorted(visible.index(h) for h in headers)
            first_reading = visible.split('Reading 1 · shown ', 1)[1].split(
                'Reading 2 · shown ', 1)[0]
            second_reading = visible.split('Reading 2 · shown ', 1)[1].split('Confirmed goal', 1)[0]
            first_main, other_goals = first_reading.split('Also in your dump', 1)
            assert compact(first_goal) in compact(first_main), 'Reading 1 wording changed'
            for item in events[1]['board']['other_goals']:
                assert compact(item) in compact(other_goals), 'Reading 1 grouping missing'
            assert compact(corrected_goal) in compact(second_reading), 'Reading 2 wording changed'
            assert 'Also in your dump' not in second_reading
            for item in added:
                assert compact('Must: ' + item) not in compact(first_main)
                assert compact('Must: ' + item) in compact(second_reading)
            for item in ['Change operator sessions.', 'Change tool configuration.']:
                assert compact('Must not: ' + item) in compact(second_reading)
            for text in [fixture['words'], fixture['correction'], corrected_goal]:
                assert compact(text) in compact(visible), 'History omitted submitted words or goal'
            run.send(F1, 'Right now')
            run.send(b'h', 'How it works')
            run.send(ESC, 'Right now')
            run.send(ESC, 'Think history')
            run.send(b'\x1b[H', 'Recorded sequence')
            assert run.screen.text() == first_history
            assert requests() == sent, 'History navigation must not start a stub request'
            after_history = {str(p): (p.read_bytes(), p.stat().st_mtime_ns)
                             for p in root.rglob('*') if p.is_file()}
            assert before_history == after_history, 'History navigation changed saved files'
            print(json.dumps(dict(width=width, height=height, no_color=no_color,
                                  provider='frozen candidates; zero real calls',
                                  tinkery_source=record['tinkery_source'],
                                  journey='omission/review/correction/save/sequence/exact reading text and groupings/read-only',
                                  acceptance='automated replay, not human or model-quality acceptance')))
        finally:
            run.close()


if __name__ == '__main__':
    for size in [(80, 24), (140, 42)]:
        for mono in [False, True]:
            journey(*size, mono)
