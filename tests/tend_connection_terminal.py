"""One explicit connection and durable Shape handoff, using real PTYs and a fake provider."""
import json
import os
from pathlib import Path
import sys
import tempfile
import time
import test_launch as subprocess
from terminal_process import TerminalProcess, wait_output

REPO = Path(__file__).resolve().parents[1]
F2 = b'\x1bOQ'
F10 = b'\x1b[21~'


def wait(term, text):
    for code in wait_output(term.master, term.process, term.screen.feed, time.monotonic()+15):
        if not term.screen.synchronized and text in term.screen.text():
            return term.screen.text()
        if code is not None:
            break
    raise AssertionError(f'Missing {text!r}:\n{term.screen.text()}')


def send(term, keys, text):
    os.write(term.master, keys)
    return wait(term, text)


def fixture(root):
    repo = root/'repo'
    (repo/'.git').mkdir(parents=True)
    records = root/'records'; records.mkdir()
    sessions = root/'sessions'; sessions.mkdir()
    for index, goal in enumerate(['TEST find repeated support requests\nKeep their original words.', 'TEST make documentation easier to find\nMeasure missed answers.']):
        name = f'goal-{index}'
        folder = sessions/name; folder.mkdir()
        (folder/'ledger.json').write_text(json.dumps(dict(schema_version=1, session_id=name, status='active', goal=goal, updated_at='2026-11-14T00:00:00Z')))
        (records/f'{name}.json').write_text(json.dumps(dict(schema=3, tinkery='TEST', origin='simulated',
            model=None, thinking=None, prompt_hash=None, started='2026-11-14T00:00:00Z', confirmed='2026-11-14T00:00:00Z',
            seed_me_session=name, seed_me_session_path=str(folder), seed_me_session_id=name,
            turns=[], checks=[], goal=goal, repo=str(repo))))
    provider = root/'fake-pi'
    provider.write_text(f'''#!{sys.executable}
import json,sys,time
from pathlib import Path
root=Path({str(root)!r})
value=json.load(sys.stdin)
with (root/'requests').open('a') as f: f.write(json.dumps(dict(input=value,args=sys.argv))+'\\n')
mode=(root/'mode').read_text() if (root/'mode').exists() else 'good'
if mode=='slow': time.sleep(30)
sources=value['sources'][:2]
proposal=dict(sources=[dict(id=s['id'],quote=s['goal']) for s in sources], relationship='TEST support gaps can guide documentation.',
 goal='TEST connect support gaps to documentation.', outcome='TEST one linked support example.', question='TEST was an answer easier to find?', assumptions=['TEST both goals concern the same users.'])
if mode=='bad': proposal['sources'][0]['quote']='not an exact source quote'
print(json.dumps(dict(connection=None if mode=='none' else proposal)))
''')
    provider.chmod(0o755)
    return repo


def launch(root, repo, width, height, budget=2):
    env=subprocess.environment({**os.environ, 'TERM':'xterm-256color','NO_COLOR':'1','TINKERY_NODE':'/tinkery-TEST-no-node'})
    term=TerminalProcess([str(REPO/'target/debug/tinkery'),'--operator','agent','--home','--shape-pi','--model','test/model',
        '--shape-budget',str(budget),'--pi-command',str(root/'fake-pi'),'--seed-session-root',str(root/'sessions'),
        '--record-root',str(root/'records'),'--link-root',str(root/'links')],cwd=repo,env=env,width=width,height=height)
    wait(term,'1  think');send(term,b'2','this repo');send(term,b'f','Find connections')
    return term


def requests(root):
    return [json.loads(line) for line in (root/'requests').read_text().splitlines()] if (root/'requests').exists() else []


def journey(width,height):
    with tempfile.TemporaryDirectory(prefix='tinkery-TEST-connection-') as directory:
        root=Path(directory).resolve();repo=fixture(root)
        originals={p:p.read_bytes() for p in (root/'sessions').glob('*/ledger.json')}
        term=launch(root,repo,width,height)
        try:
            wait(term,'F2 find one connection')
            assert not requests(root)
            send(term,F2,'p prepare this goal')
            assert len(requests(root))==1
            assert '--no-tools' in requests(root)[0]['args']
            send(term,b'\x1b','this repo')
        finally: term.close()
        # Kill and relaunch the application. The result is durable, not a second request.
        term=launch(root,repo,width,height)
        try:
            wait(term,'p prepare this goal');assert len(requests(root))==1
            send(term,b'p','F2 save exact words')
            os.write(term.master,b' EDITED')
            send(term,F2,'tinkery › shape')
            assert len(requests(root))==1
        finally: term.close()
        stored=list((root/'links').glob('connection-*.json'));assert len(stored)==1
        record=json.loads(stored[0].read_text());saved=Path(record['saved'])
        ledger=json.loads((saved/'ledger.json').read_text())
        assert 'EDITED' in ledger['goal']
        assert record['pending']['origin']['goal']==ledger['goal']
        assert all(p.read_bytes()==b for p,b in originals.items())
        # Emulate loss of only the final exploration update after real publication.
        record['saved']=None;stored[0].write_text(json.dumps(record))
        term=launch(root,repo,width,height)
        try:
            wait(term,'F2 resume saving')
            send(term,F2,'tinkery › shape')
            assert len(requests(root))==1
            assert len(list((root/'sessions').iterdir()))==3
        finally: term.close()
        term=launch(root,repo,width,height)
        try:
            wait(term,'F2 find a fresh connection')
            send(term,F2,'p prepare this goal')
            assert len(requests(root))==2
        finally: term.close()


def refusal(mode, expected, budget=2, unknown=False):
    with tempfile.TemporaryDirectory(prefix='tinkery-TEST-connection-refusal-') as directory:
        root=Path(directory).resolve();repo=fixture(root);(root/'mode').write_text(mode)
        if unknown: (root/'records'/'goal-1.json').unlink()
        term=launch(root,repo,100,30,budget)
        try:
            if unknown: wait(term,expected)
            else:
                wait(term,"F2 find one connection")
                send(term,F2,expected)
            assert len(requests(root))==(0 if unknown or budget==0 else 1)
            assert len(list((root/'sessions').iterdir()))==2
        finally: term.close()


def update_goal(root, name, **changes):
    path=root/'sessions'/name/'ledger.json'
    value=json.loads(path.read_text());value.update(changes);path.write_text(json.dumps(value))
    if 'goal' in changes:
        path=root/'records'/f'{name}.json'
        value=json.loads(path.read_text());value['goal']=changes['goal'];path.write_text(json.dumps(value))


def input_refusals():
    cases=[('one','at least two'),('many','at most 20'),('large','32 KiB'),('incomplete','incomplete'),('duplicate','unknown or invalid')]
    for case, expected in cases:
        with tempfile.TemporaryDirectory(prefix='tinkery-TEST-input-') as directory:
            root=Path(directory).resolve();repo=fixture(root)
            if case=='one': update_goal(root,'goal-1',status='stopped')
            elif case=='large': update_goal(root,'goal-0',goal='TEST '+('x'*33000))
            elif case=='incomplete': (root/'sessions'/'goal-1'/'ledger.json').write_text('broken')
            elif case=='duplicate': update_goal(root,'goal-1',session_id='goal-0')
            elif case=='many':
                for i in range(2,21):
                    name=f'goal-{i}';folder=root/'sessions'/name;folder.mkdir()
                    ledger=json.loads((root/'sessions'/'goal-0'/'ledger.json').read_text());ledger['session_id']=name
                    (folder/'ledger.json').write_text(json.dumps(ledger))
                    record=json.loads((root/'records'/'goal-0.json').read_text())
                    record.update(seed_me_session=name,seed_me_session_id=name,seed_me_session_path=str(folder))
                    (root/'records'/f'{name}.json').write_text(json.dumps(record))
            term=launch(root,repo,100,30)
            try:
                wait(term,expected);os.write(term.master,F2);time.sleep(.1)
                assert not requests(root)
            finally: term.close()


def cancellation_and_dismissal():
    with tempfile.TemporaryDirectory(prefix='tinkery-TEST-cancel-') as directory:
        root=Path(directory).resolve();repo=fixture(root);(root/'mode').write_text('slow')
        term=launch(root,repo,100,30)
        try:
            wait(term,'F2 find one connection');send(term,F2,'Working')
            deadline=time.monotonic()+5
            while not requests(root) and time.monotonic()<deadline: time.sleep(.01)
            send(term,b'\x1b','this repo');time.sleep(.2)
            assert len(requests(root))==1
            assert not list((root/'links').glob('connection-*.json'))
        finally: term.close()
        (root/'mode').write_text('good')
        term=launch(root,repo,100,30)
        try:
            wait(term,'F2 find one connection');send(term,F2,'p prepare this goal')
            send(term,b'd','Dismissed.')
        finally: term.close()
        term=launch(root,repo,100,30)
        try:
            wait(term,'Dismissed.');os.write(term.master,F2);time.sleep(.1)
            assert len(requests(root))==2
            assert len(list((root/'sessions').iterdir()))==2
        finally: term.close()


def freshness():
    for change in ['unrelated','goal','status','identity','repository','membership']:
        with tempfile.TemporaryDirectory(prefix='tinkery-TEST-freshness-') as directory:
            root=Path(directory).resolve();repo=fixture(root)
            term=launch(root,repo,100,30)
            try:
                wait(term,'F2 find one connection');send(term,F2,'p prepare this goal')
                if change=='unrelated': update_goal(root,'goal-0',version=99,revision=4,updated_at='2026-11-15T00:00:00Z')
                elif change=='goal': update_goal(root,'goal-0',goal='TEST changed goal')
                elif change=='status': update_goal(root,'goal-0',status='stopped')
                elif change=='identity': update_goal(root,'goal-0',session_id='changed-id')
                elif change=='repository':
                    path=root/'records'/'goal-0.json';value=json.loads(path.read_text());value['repo']=str(root/'other-repo');path.write_text(json.dumps(value))
                elif change=='membership':
                    folder=root/'sessions'/'unknown';folder.mkdir()
                    value=json.loads((root/'sessions'/'goal-0'/'ledger.json').read_text());value['session_id']='unknown'
                    (folder/'ledger.json').write_text(json.dumps(value))
                send(term,b'p','F2 save exact words')
                send(term,F2,'tinkery › shape' if change=='unrelated' else 'Find connections')
                if change!='unrelated':
                    time.sleep(.2)
                    assert len(list((root/'sessions').glob('session-tend-*')))==0
                assert len(requests(root))==1
            finally: term.close()


if __name__=='__main__':
    subprocess.run(['cargo','+stable','build','--locked','--quiet'],cwd=REPO,env=subprocess.environment(),check=True)
    for size in [(80,24),(100,30)]: journey(*size)
    refusal('bad','invalid or conflicts')
    refusal('good','run limit of 0 reached',budget=0)
    refusal('good','unknown or invalid',unknown=True)
    refusal('none','No useful connection found')
    input_refusals()
    cancellation_and_dismissal()
    freshness()
    print('Tend connection: 2 restart/handoff journeys, 4 refusal/no-result cases, 5 input refusals, cancellation/dismissal, 6 freshness cases; no real provider calls.')
