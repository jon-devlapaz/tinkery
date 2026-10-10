"""Actual binary + official Seed Me helper, isolated TEST roots; no provider charge/browser."""
import base64,fcntl,json,os,re,select,struct,subprocess,sys,tempfile,termios,time
from pathlib import Path
from vt_screen import Screen
REPO=Path(__file__).resolve().parents[1]
BINARY=REPO/'target/debug/tinkery'
SKILL=Path(os.environ['SEED_ME_TEST_SKILL']).resolve()
HELPER=SKILL.parent/'scripts/session.py'
HOST='''#!/usr/bin/env python3
import json,sys
from pathlib import Path
with Path('provider-calls').open('a') as log: log.write('call\\n')
r=json.load(sys.stdin)
if r.get('kind')=='meaning-preservation': print(json.dumps(dict(missing=[],false_choice=False)));sys.exit(0)
if r.get('kind')=='question-continuity':print('{"repeated":[]}');sys.exit(0)
print(json.dumps(dict(parts=dict(who='I',outcome='judge a PR ready for human review as restaurateur while both the harness and codebase compound',why='I retain final judgment',done_when='a PR is ready for human review',must=['human taste'],must_not=['cook','show cost unless it is way off','build a dashboard']),open=['why'],supports=[dict(source=s['id'],quote=s['text'],occurrence=0) for s in r['sources']],misfits=[],questions=[dict(target='why',id='open',text='Which UI or consequential design checkpoints remain unresolved?')],alternatives=[])))
'''
def helper(*args):
 p=subprocess.run(['python3',str(HELPER),*map(str,args)],capture_output=True,text=True,check=True);return json.loads(p.stdout)
def journey(width,height,home=False,menu_only=False):
 with tempfile.TemporaryDirectory(prefix='tinkery-TEST-goal-') as directory:
  cwd=Path(directory);root=cwd/'TEST-sessions';host=cwd/'pi';host.write_text(HOST);host.chmod(0o755)
  # Observe even short-lived helper/provider Python launches without changing the official helper.
  python=cwd/'python3'
  python.write_text(f'#!{sys.executable}\nimport os,sys\nwith open({str(cwd/"python-launches")!r}, "a") as log: log.write(repr(sys.argv[1:])+"\\n")\nos.execv({sys.executable!r}, [{sys.executable!r}, *sys.argv[1:]])\n')
  python.chmod(0o755)
  master,slave=os.openpty();fcntl.ioctl(slave,termios.TIOCSWINSZ,struct.pack('HHHH',height,width,0,0));screen=Screen(width,height)
  process=subprocess.Popen([str(BINARY),*(['--home'] if home else []),'--shape-pi','--model','test/model','--seed-me',str(SKILL),'--seed-session-root',str(root),'--pi-command',str(host)],stdin=slave,stdout=slave,stderr=slave,cwd=cwd,env={**os.environ,'TERM':'xterm-256color','PATH':str(cwd)+os.pathsep+os.environ['PATH']})
  raw=bytearray()
  def feed():
   data=os.read(master,65536);raw.extend(data);screen.feed(data)
  def right():
   return ' '.join(' '.join(line[width*48//100+2:].strip() for line in screen.text().splitlines()).split())
  def descendants():
   rows=subprocess.run(['ps','-axo','pid=,ppid=,command='],capture_output=True,text=True,check=True).stdout.splitlines()
   entries=[line.strip().split(None,2) for line in rows]
   found={process.pid};children=set()
   while True:
    added={int(pid) for pid,ppid,*_ in entries if int(ppid) in found}-found
    if not added: return children
    found.update(added);children.update(added)
  def await_text(text,timeout=30):
   deadline=time.monotonic()+timeout
   while time.monotonic()<deadline:
    if select.select([master],[],[],0.02)[0]:feed()
    if text in screen.text() and not screen.synchronized:
     assert screen.cells[1][2:9]==list('tinkery') and screen.cells[1][width-3]=='?',screen.text()
     return
    if process.poll() is not None:break
   raise AssertionError(f'Missing {text!r}:\n{screen.text()}')
  def absent(text):
   deadline=time.monotonic()+5
   while time.monotonic()<deadline:
    if select.select([master],[],[],0.02)[0]:feed()
    if text not in screen.text():return
   raise AssertionError(f'Stale modal still visible: {text}')
  def send(data,text):os.write(master,data);await_text(text)
  try:
   if home: await_text('think');send(b'1',"What's on your mind?")
   else: await_text("What's on your mind?")
   assert not root.exists()
   dump='  PR ready for human review. restaurateur, not cook. Café 👩‍💻; harness and codebase both compound. Cost only when it\'s way off; no dashboard.  '
   os.write(master,b'\x1b[200~'+dump.encode()+b'\x1b[201~');await_text('compound.')
   send(b'\x1bOQ','Which UI');assert not root.exists()
   send(b'\x07','type confirm');assert "type confirm to save. You can't edit it after." in ' '.join(l[width*48//100:].strip() for l in screen.text().splitlines()),screen.text();assert not root.exists()
   os.write(master,b'\r\x1bOQ');time.sleep(.1);assert not root.exists(),'Opening/Enter/F2 implied consent'
   os.write(master,b'\x1b');absent('type confirm');assert not root.exists()
   send(b'\x07','type confirm')
   os.write(master,b'\x1b[6~'*30);await_text('remain unresolved?')
   send(b'confirm\r','Saved.');await_text('Next:')
   assert 'Next: shape it into a seed with Seed Me.' in right(),screen.text()
   assert screen.text().splitlines()[-1].strip()=='c copy prompt   s open in shape   10 menu'
   assert str(root) not in screen.text() and 'seed not written yet' not in screen.text()
   first_goal=right()
   sessions=list(root.iterdir());assert len(sessions)==1
   session=sessions[0].resolve();ledger=helper('read',session);status=helper('status',session)
   assert ledger['status']=='active' and ledger['operator']=='human';assert status['snapshot_current']
   assert status['viewer'].startswith('live http://127.0.0.1:'),json.dumps(dict(status=status,screen=screen.text(),viewer_record=json.loads((session/'viewer.json').read_text()) if (session/'viewer.json').exists() else None))
   origin=next(n for n in ledger['nodes'] if n['id']==ledger['origin'])
   assert origin['status']=='settled' and origin['kind']=='decision' and origin['authority']=='user'
   assert origin['answer']==ledger['goal']=='I judge a PR ready for human review as restaurateur while both the harness and codebase compound.\nWhy: I retain final judgment.\nDone when: a PR is ready for human review.\nMust: human taste.\nMust not: cook.\nMust not: show cost unless it is way off.\nMust not: build a dashboard.'
   context=json.loads(origin['evidence'][-1]['observed'])
   assert ledger['draft']['outcome']==context['success']==context['goal_parts']['done_when']=='a PR is ready for human review'
   assert context['constraints']==dict(must=['human taste'],must_not=['cook','show cost unless it is way off','build a dashboard'])
   assert context['problem']==context['goal_parts']['why']=='I retain final judgment'
   assert any(e.get('observed')==dump and e.get('checked')=='Tinkery original 1' for e in origin['evidence']);assert 'unresolved' in origin['evidence'][-1]['observed']
   assert not (session/'seed-contract.md').exists()
   failed=subprocess.run(['python3',str(HELPER),'end',str(session),'--status','completed','--reason','TEST must refuse incomplete seed'],capture_output=True,text=True)
   assert failed.returncode!=0 and 'seed-contract.md' in failed.stderr
   assert helper('read',session)['status']=='active'
   saved=(session/'ledger.json').read_bytes()
   calls=(cwd/'provider-calls').read_bytes()
   launches=(cwd/'python-launches').read_bytes()
   owned=descendants()
   assert len(owned)==1,owned  # Only the already-started official viewer.
   os.write(master,b'\x07confirm\r\x1bOQ\x1bOR\x1bOS\x1b[15~\x1b[17~\x1b[19~\x1b[20~\x0e\x0f\x04q')
   time.sleep(.2)
   send(b'c','Copied.')
   matches=re.findall(rb'\x1b\]52;c;([^\x07\x1b]+)(?:\x07|\x1b\\)',bytes(raw))
   assert matches,bytes(raw)[-3000:]
   expected=(f'Use the seed-me skill at "{SKILL}" to continue the existing Seed Me session at "{session}". I want to continue this session and shape its confirmed goal into a seed contract.\n\n'
    "Read the session with the skill's scripts/session.py read and status commands before changing anything. Do not initialize a new session or replace the confirmed goal. Preserve recorded decisions, constraints, exclusions and unresolved context. Tinkery's answered and skipped questions are context, not additional settled Seed Me decisions. If the session is not active, stop and report its status.\n\n"
    "Follow the skill's interview and viewer lifecycle. Goal confirmation is not seed confirmation or permission to implement. Save the draft as this session's seed-contract.md; ask me to confirm the displayed revision before using session.py seed confirm. Do not start implementation.")
   assert base64.b64decode(matches[-1]).decode()==expected,(base64.b64decode(matches[-1]).decode(),expected)
   assert descendants()==owned and (cwd/'provider-calls').read_bytes()==calls
   assert (cwd/'python-launches').read_bytes()==launches
   assert (session/'ledger.json').read_bytes()==saved and len(list(root.iterdir()))==1
   os.write(master,b'\x1b[6~'*30);await_text('Must not: build a dashboard.')
   assert 'Next: shape it into a seed with Seed Me.' in right(),screen.text()
   for line in ledger['goal'].splitlines(): assert line in first_goal or line in right(),(line,first_goal,screen.text())
   send(b'\x1bOP','Right now');send(b'h','How it works')
   os.write(master,b'\x1b[6~');await_text('Session:')
   os.write(master,b'\x1b');await_text('Right now');send(b'\x1bOP','Every key')
   assert 'send' not in right() and 'speak' not in right()
   os.write(master,b'\x1b');await_text('Saved.')
   assert str(root) not in screen.text()
   if menu_only:
    send(b'\x1b[21~','think')
    assert process.poll() is None and 'shape' in screen.text()
   else:
    # Make the target non-first with the official helper, not hand-written canonical files.
    newer=Path(helper('init','--root',root)['session'])
    send(b's','tinkery › shape')
    assert 'Continue with Seed Me in any harness' not in screen.text()
    send(b'\r','Continue with Seed Me in any harness')
    assert session.name in screen.text().replace(' ','').replace('\n',''),screen.text()
    assert newer.name not in screen.text()
    send(b'\x1b','tinkery › shape');send(b'\x1b','think')
   assert (session/'ledger.json').read_bytes()==saved
   assert helper('status',session)['viewer']=='started earlier, not running now'
   assert not descendants() and (cwd/'provider-calls').read_bytes()==calls
   assert (cwd/'python-launches').read_bytes()==launches
   os.write(master,b'\x1b[21~');deadline=time.monotonic()+10
   while process.poll() is None and time.monotonic()<deadline:
    if select.select([master],[],[],.02)[0]:feed()
   assert process.poll()==0,screen.text()
   assert helper('status',session)['viewer']=='started earlier, not running now'
   assert helper('read',session)['status']=='active';assert not (session/'seed-contract.md').exists()
   print(json.dumps(dict(width=width,height=height,provider='stub',helper='official',test_confirmation='AUTOMATED TEST, NOT OPERATOR',goal_only=True,receipt_keys=['c','s','F10'],entry='home' if home else 'standalone',navigation='F10' if menu_only else 's',viewer_stopped=True,session_root='isolated temporary directory')))
  finally:
   if process.poll() is None:process.kill();process.wait()
   # If an assertion failed, stop only the viewer recorded by this TEST session.
   if root.exists():
    for s in root.iterdir():
     record=s/'viewer.json'
     if record.exists():
      pid=json.loads(record.read_text()).get('pid')
      if pid:
       command=subprocess.run(['ps','-p',str(pid),'-o','command='],capture_output=True,text=True)
       if str(SKILL.parent/'scripts/viewer.py') in command.stdout and str(s.resolve()) in command.stdout:
        try:os.kill(pid,15)
        except ProcessLookupError:pass
   os.close(master);os.close(slave)
for home in [False,True]:
 for size in [(80,24),(100,30),(160,40),(320,40)]:journey(*size,home=home)
 journey(80,24,home=home,menu_only=True)
