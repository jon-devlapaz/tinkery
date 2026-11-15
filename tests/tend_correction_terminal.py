"""Saved correction transfer and format compatibility through the public terminal flow."""
import fcntl
import hashlib
import json
import os
from pathlib import Path
import shutil
import tempfile
import time
import test_launch as subprocess
import tend_connection_terminal as flow

EVIDENCE = Path(os.environ['TINKERY_CORRECTION_EVIDENCE']) if os.environ.get('TINKERY_CORRECTION_EVIDENCE') else None

def capture(term, name):
    settle(term)
    if EVIDENCE:
        EVIDENCE.mkdir(parents=True, exist_ok=True)
        (EVIDENCE/(name+'.txt')).write_text(term.screen.text())

WORDS = 'For support work, preserve the failed lookup before suggesting documentation.\nKeep exact words: café.'


def exploration(root):
    return next(p for p in (root/'links').glob('connection-*.json') if not p.name.endswith('.correction.json'))


def correction(root):
    return next((root/'links').glob('*.correction.json'))


def settle(term):
    for _ in flow.wait_output(term.master, term.process, term.screen.feed, time.monotonic()+.15):
        pass


def edit(term, words, expected='F2 save correction only'):
    flow.send(term,b'f',expected)
    os.write(term.master,b'\x01\x0b')
    os.write(term.master,b'\x1b[200~'+words.encode()+b'\x1b[201~')


def save_correction(term, words=WORDS):
    edit(term,words)
    flow.send(term,flow.F2,'F2 find one connection')


def discover(root, repo, size=(100,30)):
    term=flow.launch(root,repo,*size,budget=10)
    flow.wait(term,'F2 find one connection')
    flow.send(term,flow.F2,'p prepare this goal')
    return term


def journey(size):
    with tempfile.TemporaryDirectory(prefix='tinkery-TEST-correction-') as directory:
        root=Path(directory).resolve();repo=flow.fixture(root)
        originals={p:p.read_bytes() for p in (root/'sessions').glob('*/ledger.json')}
        term=discover(root,repo,size)
        try:
            save_correction(term)
            stored=json.loads(correction(root).read_text());assert stored['correction']['text']==WORDS
            assert correction(root).stat().st_mode & 0o777 == 0o600
            capture(term,f'{size[0]}x{size[1]}-correction-disclosure')
            assert len(flow.requests(root))==1
        finally: term.close()
        term=flow.launch(root,repo,*size,budget=10)
        try:
            flow.wait(term,'F2 find one connection')
            assert len(flow.requests(root))==1
            flow.send(term,flow.F2,'p prepare this goal')
            request=flow.requests(root)[-1]['input']
            assert set(request)=={'repo','sources','model','thinking','harness','prompt','correction'}
            assert request['correction']==stored['correction']
            assert '--no-tools' in flow.requests(root)[-1]['args']
            settle(term)
            assert 'operator guidance' in term.screen.text(),term.screen.text()
            lines=term.screen.text().splitlines()
            assert any(line.strip()=='Keep exact words: café.' for line in lines),term.screen.text()
            assert term.screen.text().count('Keep exact words: café.')==1
            capture(term,f'{size[0]}x{size[1]}-proposal-with-correction')
            os.write(term.master,b'\x1b[6~');settle(term)
            capture(term,f'{size[0]}x{size[1]}-proposal-scrolled')
        finally: term.close()
        # A different goal set receives the same exact guidance, without a selected pair.
        for i in range(2): flow.update_goal(root,f'goal-{i}',goal=f'TEST later support task {i}')
        (root/'mode').write_text('none')
        term=flow.launch(root,repo,*size,budget=10)
        try:
            flow.wait(term,'F2 find one connection');flow.send(term,flow.F2,'No useful connection found')
            assert flow.requests(root)[-1]['input']['correction']==stored['correction']
            assert [s['goal'] for s in flow.requests(root)[-1]['input']['sources']]==['TEST later support task 0','TEST later support task 1']
            assert json.loads(correction(root).read_text())==stored
        finally: term.close()
        # Restore source words, then save a proposal with its frozen correction.
        for p,data in originals.items(): p.write_bytes(data)
        for i in range(2): flow.update_goal(root,f'goal-{i}',goal=json.loads(originals[root/'sessions'/f'goal-{i}'/'ledger.json'])['goal'])
        (root/'mode').write_text('good')
        term=flow.launch(root,repo,*size,budget=10)
        try:
            flow.wait(term,'F2 find one connection');flow.send(term,flow.F2,'p prepare this goal')
            flow.send(term,b'p','F2 save exact words');flow.send(term,flow.F2,'tinkery › shape')
            flow.wait(term,"Shape hasn't asked its next question yet.")
            capture(term,f'{size[0]}x{size[1]}-shape')
        finally: term.close()
        record=json.loads(exploration(root).read_text());dest=Path(record['saved'])
        ledger=(dest/'ledger.json').read_bytes()
        origin=record['pending']['origin']
        assert origin['schema']==2 and origin['correction']==stored['correction']
        entries=json.loads(ledger)['nodes'][0]['evidence']
        evidence=next(e for e in entries if e.get('artifact')=='tinkery:origin/tend-v2')
        assert json.loads(evidence['observed'])==origin
        term=flow.launch(root,repo,*size,budget=10)
        try:
            flow.wait(term,'o open saved goal')
            flow.send(term,b'x','F2 clear correction only');flow.send(term,flow.F2,'o open saved goal')
            settle(term);assert json.loads(correction(root).read_text())['correction'] is None
            flow.send(term,b'o','tinkery › shape')
        finally: term.close()
        assert (dest/'ledger.json').read_bytes()==ledger
        assert len(flow.requests(root))==4
        assert all(p.read_bytes()==data for p,data in originals.items())
        print(f'{size[0]}x{size[1]}: save/restart, exact disclosure, different-goal no-result, v2 handoff, clear/reopen; four fake calls')


def edits_and_conflicts():
    with tempfile.TemporaryDirectory(prefix='tinkery-TEST-correction-edit-') as directory:
        root=Path(directory).resolve();repo=flow.fixture(root)
        term=discover(root,repo)
        try:
            edit(term,'TEST cancelled');flow.send(term,b'\x1b','p prepare this goal')
            assert not list((root/'links').glob('*.correction.json'))
            edit(term,'x'*2049);flow.send(term,flow.F2,'1–2048 bytes')
            capture(term,'oversized-edit-retained')
            assert not list((root/'links').glob('*.correction.json'))
            # Refusal retains the draft; correct it and save without another call.
            os.write(term.master,b'\x01\x0bTEST corrected');flow.send(term,flow.F2,'F2 find one connection')
            before=correction(root).read_bytes()
            assert json.loads(before)['correction']['text']=='TEST corrected'
            edit(term,'TEST stale edit')
            other=flow.launch(root,repo,100,30,budget=0)
            try:
                flow.wait(other,'F2 find one connection');save_correction(other,'TEST other writer')
            finally: other.close()
            flow.send(term,flow.F2,'correction changed')
            assert json.loads(correction(root).read_text())['correction']['text']=='TEST other writer'
            flow.send(term,b'\x1b','F2 find one connection')
            # Holding the same repository lock refuses mutation without changing bytes.
            lock=exploration(root).with_suffix('.lock').open('w');fcntl.flock(lock,fcntl.LOCK_EX|fcntl.LOCK_NB)
            before=correction(root).read_bytes()
            edit(term,'TEST locked');flow.send(term,flow.F2,'Another connection pass')
            assert correction(root).read_bytes()==before
            lock.close()
            flow.send(term,b'\x1b','F2 find one connection')
            flow.send(term,b'x','F2 clear correction only')
            value=json.loads(before);value['correction']['revision']='a'*48;correction(root).write_text(json.dumps(value))
            flow.send(term,flow.F2,'correction changed')
            assert json.loads(correction(root).read_text())==value
            assert len(flow.requests(root))==1
        finally: term.close()
        print('cancel, oversized edit recovery, stale Replace/Clear and active writer refusal: one fake call')


def malformed_and_legacy():
    with tempfile.TemporaryDirectory(prefix='tinkery-TEST-correction-formats-') as directory:
        root=Path(directory).resolve();repo=flow.fixture(root)
        term=discover(root,repo)
        try:
            flow.send(term,b'p','F2 save exact words');flow.send(term,flow.F2,'tinkery › shape')
        finally: term.close()
        path=exploration(root);record=json.loads(path.read_text());saved=Path(record['saved'])
        assert record['schema']==2 and record['input']['correction'] is None
        assert record['pending']['origin']['schema']==1
        # Reconstruct the #861 wire format and its actual old prompt hash.
        record['schema']=1;del record['input']['correction']
        old_prompt=subprocess.check_output(['git','show','1d14afc:src/tend/connection-prompt.txt'],cwd=flow.REPO,env=subprocess.environment())
        record['input']['prompt']=hashlib.sha256(old_prompt).hexdigest()
        record['saved']=None;shutil.rmtree(saved)
        path.write_text(json.dumps(record))
        term=flow.launch(root,repo,100,30,budget=0)
        try:
            flow.wait(term,'F2 resume saving')
            before=path.read_bytes();settle(term);assert path.read_bytes()==before
            flow.send(term,flow.F2,'tinkery › shape')
        finally: term.close()
        resumed=json.loads(path.read_text());assert resumed['schema']==1 and 'correction' not in resumed['input']
        assert resumed['pending']==record['pending'] and len(flow.requests(root))==1
        # Current bad feedback cannot prevent opening the verified frozen goal.
        cp=path.with_suffix('.correction.json');cp.write_text('{broken')
        term=flow.launch(root,repo,100,30,budget=0)
        try:
            flow.wait(term,'o open saved goal');flow.send(term,b'o','tinkery › shape')
        finally: term.close()
        term=flow.launch(root,repo,100,30,budget=0)
        try:
            flow.wait(term,'o open saved goal');flow.send(term,b'x','F2 clear correction only')
            flow.send(term,flow.F2,'o open saved goal');settle(term)
            assert json.loads(cp.read_text())['correction'] is None
        finally: term.close()
        cp.unlink();cp.symlink_to(root/'unsafe-target')
        term=flow.launch(root,repo,100,30,budget=0)
        try:
            flow.wait(term,'cannot be read safely');os.write(term.master,b'x');settle(term)
            assert not (root/'unsafe-target').exists() and cp.is_symlink()
        finally: term.close()
        print('legacy prompt pending recovery, malformed correction saved-goal opening/Clear and symlink refusal: one fake call')


def exact_trailing_newline():
    with tempfile.TemporaryDirectory(prefix='tinkery-TEST-correction-exact-end-') as directory:
        root=Path(directory).resolve();repo=flow.fixture(root)
        term=discover(root,repo)
        try:
            words='Exact guidance.\n'
            save_correction(term,words)
            assert json.loads(correction(root).read_text())['correction']['text']==words
            flow.send(term,b'f','F2 save correction only')
            flow.send(term,flow.F2,'F2 find one connection')
            assert json.loads(correction(root).read_text())['correction']['text']==words, 'Untouched replacement changed the exact correction'
            assert len(flow.requests(root))==1
        finally: term.close()
        print('one final newline survives untouched replacement without a call')


def capture_context_and_exact_replacement(size):
    with tempfile.TemporaryDirectory(prefix='tinkery-TEST-correction-capture-') as directory:
        root=Path(directory).resolve();repo=flow.fixture(root)
        term=discover(root,repo,size)
        try:
            flow.send(term,b'f','F2 save correction only')
            screen=term.screen.text()
            assert 'Reviewed proposal' in screen,screen
            compact=''.join(line.strip() for line in screen.splitlines())
            assert 'TEST connect support gaps to documentation.' in ' '.join(screen.split()),screen
            assert str(repo) in compact,screen
            capture(term,f'{size[0]}x{size[1]}-capture-context')
            words='Exact guidance.\n\n'
            os.write(term.master,b'\x1b[200~'+words.encode()+b'\x1b[201~')
            flow.send(term,b'\t','tab edit')
            # Scroll the complete proposal without changing or abandoning the draft.
            flow.send(term,b'\x1b[F','Assumption or conflict')
            capture(term,f'{size[0]}x{size[1]}-capture-context-scrolled')
            assert not list((root/'links').glob('*.correction.json'))
            assert len(flow.requests(root))==1
            flow.send(term,b'\t','tab context')
            assert 'Exact guidance.' in term.screen.text()
            flow.send(term,flow.F2,'F2 find one connection')
            assert json.loads(correction(root).read_text())['correction']['text']==words
            flow.send(term,b'f','F2 save correction only')
            capture(term,f'{size[0]}x{size[1]}-replacement-with-final-blanks')
            flow.send(term,flow.F2,'F2 find one connection')
            assert json.loads(correction(root).read_text())['correction']['text']==words
            assert len(flow.requests(root))==1
        finally: term.close()
        term=flow.launch(root,repo,*size,budget=0)
        try:
            flow.wait(term,'F2 find one connection')
            flow.send(term,b'f','F2 save correction only')
            flow.send(term,flow.F2,'F2 find one connection')
            assert json.loads(correction(root).read_text())['correction']['text']==words
            assert len(flow.requests(root))==1
        finally: term.close()
        print(f'{size[0]}x{size[1]}: capture shows repo/proposal; context scrolling retains draft; two final newlines survive untouched replacement/restart')


def long_repository_capture():
    with tempfile.TemporaryDirectory(prefix='tinkery-TEST-correction-long-repo-') as directory:
        root=Path(directory).resolve()
        for i in range(12): root=root/('repository-segment-'+str(i)+'-'+('x'*40))
        root.mkdir(parents=True);repo=flow.fixture(root)
        term=discover(root,repo,(80,24))
        try:
            flow.send(term,b'f','F2 save correction only')
            os.write(term.master,b'\x1b[200~TEST draft survives full repository review\x1b[201~')
            flow.send(term,b'\t','tab edit')
            observed=[]
            for step in range(20):
                observed.append(term.screen.text())
                if step in (0,5,10,15): capture(term,f'80x24-long-repository-context-{step}')
                os.write(term.master,b'\x1b[B');settle(term)
            # Each long path segment and the final repo are inspectable by scrolling.
            compact=''.join(line.strip() for frame in observed for line in frame.splitlines())
            for part in repo.parts:
                assert part in compact,(part,observed)
            assert 'Reviewed proposal' in compact
            capture(term,'80x24-long-repository-context')
            flow.send(term,b'\t','tab context')
            assert 'TEST draft survives full repository review' in term.screen.text()
            assert not list((root/'links').glob('*.correction.json'))
            flow.send(term,flow.F2,'F2 find one connection')
            assert json.loads(correction(root).read_text())['correction']['text']=='TEST draft survives full repository review'
            assert len(flow.requests(root))==1
        finally: term.close()
        print('long repository remains scrollable during capture; navigation retains draft and makes no call')


def disclosure_matches_request():
    with tempfile.TemporaryDirectory(prefix='tinkery-TEST-correction-disclosure-') as directory:
        root=Path(directory).resolve();repo=flow.fixture(root)
        term=discover(root,repo)
        try:
            save_correction(term,'TEST A reviewed words')
            flow.wait(term,'TEST A reviewed words')
            value=json.loads(correction(root).read_text())
            value['correction']['revision']='e'*48
            value['correction']['text']='TEST B current words'
            correction(root).write_text(json.dumps(value))
            flow.send(term,flow.F2,'goal inputs changed')
            assert len(flow.requests(root))==1
            capture(term,'stale-disclosure-refusal')
        finally: term.close()
        term=flow.launch(root,repo,100,30,budget=10)
        try:
            flow.wait(term,'TEST B current words')
            assert 'TEST A reviewed words' not in term.screen.text()
            flow.send(term,flow.F2,'p prepare this goal')
            assert flow.requests(root)[-1]['input']['correction']==value['correction']
            assert 'TEST B current words' in term.screen.text()
            capture(term,'disclosed-request-words')
        finally: term.close()
        assert len(flow.requests(root))==2
        print('stale disclosure refuses a call; reopening shows the exact correction sent')


def failed_write_keeps_words():
    if os.geteuid()==0:
        print('Permission refusal requires an ordinary user; skipped under root.')
        return
    with tempfile.TemporaryDirectory(prefix='tinkery-TEST-correction-write-') as directory:
        root=Path(directory).resolve();repo=flow.fixture(root)
        term=discover(root,repo)
        try:
            save_correction(term,'TEST original')
            before=correction(root).read_bytes()
            edit(term,'TEST retained after write failure')
            (root/'links').chmod(0o500)
            try:
                flow.send(term,flow.F2,'Permission denied')
                assert 'TEST retained after write failure' in term.screen.text()
                assert correction(root).read_bytes()==before
                capture(term,'failed-write-keeps-editor')
            finally: (root/'links').chmod(0o700)
            flow.send(term,flow.F2,'F2 find one connection')
            assert json.loads(correction(root).read_text())['correction']['text']=='TEST retained after write failure'
            assert len(flow.requests(root))==1
        finally: term.close()
        print('failed filesystem write preserves prior correction and exact editor words; retry makes no request')


def stale_goal_and_pending():
    with tempfile.TemporaryDirectory(prefix='tinkery-TEST-correction-freshness-') as directory:
        root=Path(directory).resolve();repo=flow.fixture(root)
        term=discover(root,repo)
        try:
            save_correction(term)
            flow.send(term,flow.F2,'p prepare this goal')
            flow.send(term,b'p','F2 save exact words')
            value=json.loads(correction(root).read_text())
            value['correction']['revision']='c'*48;value['correction']['text']='TEST changed guidance'
            correction(root).write_text(json.dumps(value))
            flow.send(term,flow.F2,'source inputs changed')
            assert not list((root/'sessions').glob('session-tend-*'))
            assert len(flow.requests(root))==2
        finally: term.close()
        term=flow.launch(root,repo,100,30,budget=10)
        try:
            flow.wait(term,'F2 find one connection');flow.send(term,flow.F2,'p prepare this goal')
            flow.send(term,b'p','F2 save exact words');flow.send(term,flow.F2,'tinkery › shape')
        finally: term.close()
        path=exploration(root);record=json.loads(path.read_text());saved=Path(record['saved'])
        record['saved']=None;path.write_text(json.dumps(record));shutil.rmtree(saved)
        before=correction(root).read_bytes()
        term=flow.launch(root,repo,100,30,budget=0)
        try:
            flow.wait(term,'F2 resume saving');os.write(term.master,b'x');settle(term)
            assert correction(root).read_bytes()==before
            # An external edit is detected before the pending goal can publish.
            value=json.loads(before);value['correction']['revision']='d'*48
            correction(root).write_text(json.dumps(value))
            flow.send(term,flow.F2,'source inputs changed before publication')
            assert not saved.exists()
            assert len(flow.requests(root))==3
        finally: term.close()
        print('correction freshness rejects stale edited goal and pending publication; no extra calls')


def input_limit_and_format_refusals():
    with tempfile.TemporaryDirectory(prefix='tinkery-TEST-correction-limit-') as directory:
        root=Path(directory).resolve();repo=flow.fixture(root)
        flow.update_goal(root,'goal-0',goal='TEST '+('x'*31000))
        term=discover(root,repo)
        try:
            edit(term,'y'*2048);flow.send(term,flow.F2,'32 KiB')
            assert len(flow.requests(root))==1
            assert len(json.loads(correction(root).read_text())['correction']['text'])==2048
            os.write(term.master,flow.F2);settle(term);assert len(flow.requests(root))==1
        finally: term.close()
    for case in ['v1-extra','v2-missing','unknown','pending-missing']:
        with tempfile.TemporaryDirectory(prefix='tinkery-TEST-correction-format-') as directory:
            root=Path(directory).resolve();repo=flow.fixture(root)
            term=discover(root,repo)
            try:
                save_correction(term);flow.send(term,flow.F2,'p prepare this goal')
                flow.send(term,b'p','F2 save exact words');flow.send(term,flow.F2,'tinkery › shape')
            finally: term.close()
            path=exploration(root);record=json.loads(path.read_text())
            if case=='v1-extra': record['schema']=1
            elif case=='v2-missing': del record['input']['correction']
            elif case=='unknown': record['schema']=99
            else: del record['pending']['origin']['correction']
            path.write_text(json.dumps(record));before=path.read_bytes()
            term=flow.launch(root,repo,100,30,budget=0)
            try:
                flow.wait(term,'latest exploration is malformed');settle(term)
                assert path.read_bytes()==before
                assert len(flow.requests(root))==2
            finally: term.close()
    print('complete input limit and four strict exploration/receipt format refusals passed')


if __name__=='__main__':
    subprocess.run(['cargo','+stable','build','--locked','--quiet'],cwd=flow.REPO,env=subprocess.environment(),check=True)
    for size in [(80,24),(100,30)]:
        journey(size)
        capture_context_and_exact_replacement(size)
    exact_trailing_newline()
    long_repository_capture()
    edits_and_conflicts()
    malformed_and_legacy()
    disclosure_matches_request()
    failed_write_keeps_words()
    stale_goal_and_pending()
    input_limit_and_format_refusals()
    print('Correction journeys passed. Fake responses prove transfer mechanics, not usefulness or relevance.')
