"""Actual binary + official Seed Me helper, isolated TEST roots; no provider charge/browser."""
import fcntl,json,os,select,struct,subprocess,tempfile,termios,time
from pathlib import Path
from vt_screen import Screen
REPO=Path(__file__).resolve().parents[1]
BINARY=REPO/'target/debug/tinkery'
SKILL=Path(os.environ['SEED_ME_TEST_SKILL']).resolve()
HELPER=SKILL.parent/'scripts/session.py'
HOST='''#!/usr/bin/env python3
import json,sys
r=json.load(sys.stdin)
if r.get('kind')=='meaning-preservation': print(json.dumps(dict(missing=[],false_choice=False)));sys.exit(0)
if r.get('kind')=='question-continuity':print('{"repeated":[]}');sys.exit(0)
print(json.dumps(dict(uncertain=False,framings=[dict(text='A PR ready for human review, with the restaurateur judging the finished result while both the harness and codebase compound.',supports=[dict(source=s['id'],quote=s['text'],occurrence=0) for s in r['sources']])],outcome='A PR ready for human review.',misfits=[],questions=[dict(id='open',text='Which UI or consequential design checkpoints remain unresolved?')],alternatives=[])))
'''
def helper(*args):
 p=subprocess.run(['python3',str(HELPER),*map(str,args)],capture_output=True,text=True,check=True);return json.loads(p.stdout)
def journey(width,height):
 with tempfile.TemporaryDirectory(prefix='tinkery-TEST-goal-') as directory:
  cwd=Path(directory);root=cwd/'TEST-sessions';host=cwd/'pi';host.write_text(HOST);host.chmod(0o755)
  master,slave=os.openpty();fcntl.ioctl(slave,termios.TIOCSWINSZ,struct.pack('HHHH',height,width,0,0));screen=Screen(width,height)
  process=subprocess.Popen([str(BINARY),'--shape-pi','--model','test/model','--seed-me',str(SKILL),'--seed-session-root',str(root),'--pi-command',str(host)],stdin=slave,stdout=slave,stderr=slave,cwd=cwd,env={**os.environ,'TERM':'xterm-256color'})
  def await_text(text,timeout=30):
   deadline=time.monotonic()+timeout
   while time.monotonic()<deadline:
    if select.select([master],[],[],0.02)[0]:screen.feed(os.read(master,65536))
    if text in screen.text():return
    if process.poll() is not None:break
   raise AssertionError(f'Missing {text!r}:\n{screen.text()}')
  def absent(text):
   deadline=time.monotonic()+5
   while time.monotonic()<deadline:
    if select.select([master],[],[],0.02)[0]:screen.feed(os.read(master,65536))
    if text not in screen.text():return
   raise AssertionError(f'Stale modal still visible: {text}')
  def send(data,text):os.write(master,data);await_text(text)
  try:
   await_text("What's on your mind?");assert not root.exists()
   dump='  PR ready for human review. restaurateur, not cook. Café 👩‍💻; harness and codebase both compound.  '
   os.write(master,b'\x1b[200~'+dump.encode()+b'\x1b[201~');await_text('compound.')
   send(b'\x1bOQ','Which UI');assert not root.exists()
   send(b'\x07','type confirm');assert 'Creates a Seed Me session' in screen.text();assert not root.exists()
   os.write(master,b'\r\x1bOQ');time.sleep(.1);assert not root.exists(),'Opening/Enter/F2 implied consent'
   os.write(master,b'\x1b');absent('type confirm');assert not root.exists()
   send(b'\x07','type confirm')
   os.write(master,b'\x1b[6~'*5);await_text('remain unresolved?')
   send(b'confirm\r','Goal confirmed — seed not written yet');await_text('Continue with Seed Me in any harness')
   sessions=list(root.iterdir());assert len(sessions)==1
   session=sessions[0];ledger=helper('read',session);status=helper('status',session)
   assert ledger['status']=='active' and ledger['operator']=='human';assert status['snapshot_current']
   assert status['viewer'].startswith('live http://127.0.0.1:'),json.dumps(dict(status=status,screen=screen.text(),viewer_record=json.loads((session/'viewer.json').read_text()) if (session/'viewer.json').exists() else None))
   origin=next(n for n in ledger['nodes'] if n['id']==ledger['origin'])
   assert origin['status']=='settled' and origin['kind']=='decision' and origin['authority']=='user'
   assert origin['answer']==ledger['goal']=='A PR ready for human review, with the restaurateur judging the finished result while both the harness and codebase compound.'
   assert any(e.get('observed')==dump and e.get('checked')=='Tinkery original 1' for e in origin['evidence']);assert 'unresolved' in origin['evidence'][-1]['observed']
   assert not (session/'seed-contract.md').exists()
   failed=subprocess.run(['python3',str(HELPER),'end',str(session),'--status','completed','--reason','TEST must refuse incomplete seed'],capture_output=True,text=True)
   assert failed.returncode!=0 and 'seed-contract.md' in failed.stderr
   assert helper('read',session)['status']=='active'
   os.write(master,b'\x07confirm\r\x1bOQ');time.sleep(.1);assert len(list(root.iterdir()))==1
   os.write(master,b'q');deadline=time.monotonic()+10
   while process.poll() is None and time.monotonic()<deadline:
    if select.select([master],[],[],.02)[0]:screen.feed(os.read(master,65536))
   assert process.poll()==0,screen.text()
   assert helper('status',session)['viewer']=='started earlier, not running now'
   assert helper('read',session)['status']=='active';assert not (session/'seed-contract.md').exists()
   print(json.dumps(dict(width=width,height=height,provider='stub',helper='official',test_confirmation='AUTOMATED TEST, NOT OPERATOR',goal_only=True,viewer_stopped=True,session_root='isolated temporary directory')))
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
       if str(SKILL.parent/'scripts/viewer.py') in command.stdout and str(s) in command.stdout:
        try:os.kill(pid,15)
        except ProcessLookupError:pass
   os.close(master);os.close(slave)
for size in [(80,24),(100,30),(160,40),(320,40)]:journey(*size)
