"""Concern text is a synthetic regression, not a new operator dump or live model run."""
import fcntl,json,os,select,struct,subprocess,tempfile,termios,time
from pathlib import Path
from vt_screen import Screen
BINARY=Path(__file__).resolve().parents[1]/'target/debug/tinkery'
HOST='''#!/usr/bin/env python3
import json,sys
r=json.load(sys.stdin)
if r.get('kind')=='meaning-preservation':print('{"missing":[],"false_choice":false}');sys.exit(0)
print(json.dumps(dict(parts=dict(who='I',outcome='understand whether its complexity earns its keep'),open=['done_when'],supports=[dict(source=1,quote='i am worried that this thing has been overengineered',occurrence=0)],misfits=[],questions=[dict(id='complexity',text='Which complexity makes you worry?')],alternatives=[])))
'''
def journey(w,h):
 with tempfile.TemporaryDirectory(prefix='tinkery-TEST-help-exit-') as d:
  cwd=Path(d);host=cwd/'pi';host.write_text(HOST);host.chmod(0o755);skill=cwd/'SKILL.md';skill.write_text('# Seed Me\n### Shape the working draft\nUnderstand intent.\n### Size gate\n')
  master,slave=os.openpty();fcntl.ioctl(slave,termios.TIOCSWINSZ,struct.pack('HHHH',h,w,0,0));original=termios.tcgetattr(slave);screen=Screen(w,h);raw=bytearray()
  process=subprocess.Popen([str(BINARY),'--shape-pi','--model','test/model','--seed-me',str(skill),'--pi-command',str(host)],stdin=slave,stdout=slave,stderr=slave,cwd=cwd,env={**os.environ,'TERM':'xterm-256color'})
  def await_condition(condition):
   deadline=time.monotonic()+5
   while time.monotonic()<deadline:
    if select.select([master],[],[],.02)[0]:
     chunk=os.read(master,65536);raw.extend(chunk);screen.feed(chunk)
    if condition(screen.text()) and not screen.synchronized:
     assert screen.cells[1][2:9]==list('tinkery') and screen.cells[1][screen.width-3]=='?',screen.text()
     return
    if process.poll() is not None:break
   raise AssertionError(screen.text())
  def wait(text):await_condition(lambda s:text in s)
  def send(data,text):os.write(master,data);wait(text)
  try:
   wait("What's on your mind?");send(b'\x1bOP','Type anything');send(b'h','How it works');send(b'\x1bOP',"What's on your mind?")
   dump='i am worried that this thing has been overengineered\n'+''.join(f'line-{i:02d} preserves an exact concern and its context.\n' for i in range(36))+'LAST-ORIGINAL-LINE'
   send(b'\x1b[200~'+dump.encode()+b'\x1b[201~','LAST-ORIGINAL-LINE')
   send(b'\x1b[21~','Back to menu and lose this draft? y / n');assert process.poll() is None
   os.write(master,b'n');await_condition(lambda s:'lose this draft?' not in s);wait('LAST-ORIGINAL-LINE')
   send(b'\x1bOQ','Which complexity');send(b'\x07','type confirm')
   # Source starts at its beginning; review never paints over the top header.
   assert ''.join(screen.cells[4][2:20])=='i am worried that ',screen.text()
   os.write(master,b'\x1b[<65;4;6M'*30);wait('LAST-ORIGINAL-LINE')
   send(b'\x1b[21~','Back to menu and lose this draft? y / n');os.write(master,b'n');await_condition(lambda s:'lose this draft?' not in s);wait('type confirm')
   send(b'\x1bOP','Type confirm');send(b'h','How it works')
   # Resize the same live alternate screen and force a complete repaint.
   nw,nh=(100,30) if w==80 else (80,24)
   fcntl.ioctl(slave,termios.TIOCSWINSZ,struct.pack('HHHH',nh,nw,0,0));screen=Screen(nw,nh);os.write(master,b'\x0c');wait('How it works')
   send(b'\x1b[21~','Back to menu and lose this draft? y / n');os.write(master,b'y');process.wait(timeout=5);assert process.returncode==0
   while select.select([master],[],[],.02)[0]:raw.extend(os.read(master,65536))
   assert b'\x1b[?2004l' in raw and b'\x1b[?1006l' in raw and b'\x1b[>0s' in raw
   after=termios.tcgetattr(slave);original[3]&=~getattr(termios,'PENDIN',0);after[3]&=~getattr(termios,'PENDIN',0);assert after==original
   assert {p.name for p in cwd.iterdir()}=={'pi','SKILL.md'},'Unexpected persistence'
   print(json.dumps(dict(width=w,height=h,provider='stub',journey='contextual help/source wheel/review/header/live resize/guarded exit/restore',test_source='synthetic concern; not operator recognition')))
  finally:
   if process.poll() is None:process.kill();process.wait()
   os.close(master);os.close(slave)
for size in [(80,24),(100,30),(160,40),(320,40)]:journey(*size)
