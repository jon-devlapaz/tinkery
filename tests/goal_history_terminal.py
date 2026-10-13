"""Two confirmed goals in one real CLI process; fake Pi and isolated TEST roots only."""
import fcntl,json,os,select,signal,struct,subprocess,tempfile,termios,time
from pathlib import Path
from vt_screen import Screen
REPO=Path(__file__).resolve().parents[1]
BINARY=REPO/'target/debug/tinkery'
SKILL=Path(os.environ['SEED_ME_TEST_SKILL']).resolve()
HELPER=SKILL.parent/'scripts/session.py'
HOST='''#!/usr/bin/env python3
import json,sys,pathlib
r=json.load(sys.stdin)
with pathlib.Path('calls.jsonl').open('a') as f:f.write(json.dumps(r)+'\\n')
if r.get('kind')=='decision-review':
 decisions=r.get('decisions',[])
 if not decisions:print('{"decisions":[]}');sys.exit(0)
 reply=[]
 for i,d in enumerate(decisions):
  reply.append(dict(part=d['part'],text=d['text'],answers=d['answers'],question=d.get('question','May refusals show private text?'),covers=[i]))
 print(json.dumps(dict(decisions=reply)));sys.exit(0)
marker=r['sources'][0]['text']
choices=[]
if len(r['sources'])==1:
 choices=[dict(part='must_not',text='Private text',answers=['omit','include'],question='May refusals show private text?')]
 if marker=='BRAVO':choices.append(dict(part='must_not',text='Uploads',answers=['local','cloud'],question='May logs upload?'))
print(json.dumps(dict(parts=dict(who='I',outcome='finish '+marker,why='the work is stuck',done_when='tests pass for '+marker),decisions=choices,questions=[])))
'''
def helper(*args):
 return json.loads(subprocess.check_output(['python3',str(HELPER),*map(str,args)],text=True))
def journey(w,h,home):
 with tempfile.TemporaryDirectory(prefix='tinkery-TEST-goal-history-') as directory:
  cwd=Path(directory);root=cwd/'TEST-sessions';records=cwd/'TEST-records';host=cwd/'pi';host.write_text(HOST);host.chmod(0o755)
  master,slave=os.openpty();fcntl.ioctl(slave,termios.TIOCSWINSZ,struct.pack('HHHH',h,w,0,0));screen=Screen(w,h)
  p=subprocess.Popen([str(BINARY),*(['--home'] if home else []),'--shape-pi','--model','test/model','--seed-me',str(SKILL),'--pi-command',str(host),'--seed-session-root',str(root),'--record-root',str(records)],stdin=slave,stdout=slave,stderr=slave,cwd=cwd,env={**os.environ,'TERM':'xterm-256color'})
  def drain(until,timeout=20):
   end=time.monotonic()+timeout
   while time.monotonic()<end:
    if select.select([master],[],[],.02)[0]:
     try:screen.feed(os.read(master,65536))
     except OSError:break
    if not screen.synchronized and until(screen.text()):return
    if p.poll() is not None:break
   # The binary can exit between the check above and poll(); look once more before failing.
   if not screen.synchronized and until(screen.text()):return
   raise AssertionError(screen.text())
  def send(keys,text):os.write(master,keys);drain(lambda s:text in s)
  def right():return ' '.join(' '.join(l[w*48//100+2:].split()) for l in screen.text().splitlines())
  def confirm():
   send(b'\x1bOR','type confirm');send(b'confirm\r','Saved.')
  try:
   if home:send(b'1',"What's on your mind?")
   else:drain(lambda s:"What's on your mind?" in s)
   send(b'ALPHA','ALPHA');send(b'\x1bOQ','May refusals show private text?')
   send(b'omit','omit');send(b'\x1bOQ',"Nothing I'm unsure about")
   confirm()
   alpha_file=next(records.iterdir());alpha_bytes=alpha_file.read_bytes();alpha=json.loads(alpha_bytes)
   assert sum(c['check']=='structure-spans-history' for c in alpha['checks'])==2,alpha['checks']
   send(b'\x1b[21~','1  think');send(b'1',"What's on your mind?")
   send(b'BRAVO','BRAVO');send(b'\x1bOQ','May refusals show private text?')
   send(b'\x1bOR','type confirm')
   assert right().count('May refusals show private text?')==1,right()
   assert right().count('Open decision: Private text')==0,right()
   assert 'Open decision: Uploads' in right(),right()
   send(b'confirm\r','Saved.')
   found=list(records.iterdir());assert len(found)==2,'Second visit lost record setting'
   bravo_file=next(f for f in found if f!=alpha_file);bravo_text=bravo_file.read_text();bravo=json.loads(bravo_text)
   assert 'ALPHA' not in bravo_text,'Prior goal leaked into the second think record'
   assert sum(c['check']=='structure-spans-history' for c in bravo['checks'])==1,bravo['checks']
   assert bravo['turns'][0]['text']=='BRAVO'
   ledger=helper('read',root/bravo['seed_me_session'])
   origin=next(n for n in ledger['nodes'] if n['id']==ledger['origin'])
   context=json.loads(origin['evidence'][-1]['observed'])
   assert ledger['goal']==bravo['goal']
   assert len(context['decisions'])==2 and len(context['unresolved'])==1
   assert context['unresolved_notes']==['Open decision: Uploads (local / cloud)'],context
   assert not (root/bravo['seed_me_session']/'seed-contract.md').exists()
   send(b'\x1b[21~','1  think');os.write(master,b'q');drain(lambda s:p.poll() is not None);assert p.returncode==0
   assert alpha_file.read_bytes()==alpha_bytes,'Starting a new goal changed the older record'
   for session in root.iterdir():assert helper('status',session)['viewer']=='started earlier, not running now'
   calls=[json.loads(l) for l in (cwd/'calls.jsonl').read_text().splitlines()]
   assert len(calls)==6,'Navigation or confirmation launched extra provider calls'
   print(json.dumps(dict(width=w,height=h,entry='home' if home else 'standalone',provider='stub',journey='two goals/first refined/record isolation/linked decision once/distinct same-part decision retained/record folder/viewer stop',confirmed_goals=2)))
  finally:
   if p.poll() is None:p.kill();p.wait()
   if root.exists():
    for session in root.iterdir():
     record=session/'viewer.json'
     if record.exists():
      pid=json.loads(record.read_text()).get('pid')
      if pid:
       command=subprocess.run(['ps','-p',str(pid),'-o','command='],capture_output=True,text=True)
       if str(SKILL.parent/'scripts/viewer.py') in command.stdout and str(session.resolve()) in command.stdout:
        try:os.kill(pid,signal.SIGTERM)
        except ProcessLookupError:pass
   os.close(master);os.close(slave)
for home in [False,True]:
 for size in [(80,24),(100,30),(160,40),(320,40)]:journey(*size,home)
