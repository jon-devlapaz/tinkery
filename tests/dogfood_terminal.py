"""Synthetic PTY regression, not live-model or clipboard-permission evidence."""
import fcntl, json, os, select, signal, struct, tempfile, termios, time
import test_launch as subprocess
from pathlib import Path
from vt_screen import Screen
BINARY=Path(__file__).resolve().parents[1]/'target/debug/tinkery'
HOST='''#!/usr/bin/env python3
import json,sys,pathlib,time,os
root=pathlib.Path(__file__).parent
r=json.load(sys.stdin)
with (root/'calls.jsonl').open('a') as f:f.write(json.dumps(r)+'\\n')
if not r.get('kind') and r['sources'][0]['text'].startswith('Slow'):
 (root/'shaping.pid').write_text(str(os.getpid()))
 time.sleep(15)
print(json.dumps(dict(parts=dict(who='I',outcome='remember books and notes',why='I forget what I read',done_when='I can find a book and its note'),open=['outcome'],supports=[dict(source=1,quote=r['sources'][0]['text'],occurrence=0)],misfits=[],questions=[dict(id='book-notes',text='What would you want to remember about each book?')],alternatives=[])))
'''
def journey(w,h):
 with tempfile.TemporaryDirectory(prefix='tinkery-TEST-dogfood-') as d:
  cwd=Path(d);host=cwd/'pi';host.write_text(HOST);host.chmod(0o755)
  master,slave=os.openpty();fcntl.ioctl(slave,termios.TIOCSWINSZ,struct.pack('HHHH',h,w,0,0));original=termios.tcgetattr(slave);screen=Screen(w,h);raw=bytearray()
  process=subprocess.Popen([str(BINARY),'--shape-pi', '--shape-budget', '64','--record-root',str(cwd/'TEST-records'),'--model','test/model','--pi-command',str(host),'--seed-session-root',str(cwd/'TEST-sessions')],stdin=slave,stdout=slave,stderr=slave,cwd=cwd,env=subprocess.environment({**os.environ,'TERM':'xterm-256color'}))
  def await_condition(check):
   end=time.monotonic()+6
   while time.monotonic()<end:
    if select.select([master],[],[],.02)[0]:
     chunk=os.read(master,65536);raw.extend(chunk);screen.feed(chunk)
    if not screen.synchronized and check(screen.text()):
     assert screen.cells[1][2:9]==list('tinkery') and screen.cells[1][screen.width-3]=='?',screen.text()
     return
    if process.poll() is not None:break
   raise AssertionError(screen.text())
  def send(keys,text):os.write(master,keys);await_condition(lambda s:text in s)
  try:
   await_condition(lambda s:"What's on your mind?" in s)
   send(b'\x1bOQ','Write something, then F2 sends it.')
   assert not (cwd/'calls.jsonl').exists(),'Empty send called the provider'
   assert not (cwd/'TEST-sessions').exists() and not (cwd/'TEST-records').exists()
   send(b'Books read this year.','Books read this year.')
   assert 'Write something, then F2 sends it.' not in screen.text(),screen.text()
   send(b'\x1bOQ','What would you want')
   send(b'\x1bOR','type confirm');assert process.poll() is None
   send(b'\x1b','What would you want')
   os.write(master,b'\x1b[17~');time.sleep(.2);assert 'board controls' not in screen.text()
   send(b'\x1b[19~','Question skipped')
   send(b'\x1a','Skip undone')
   send(b'\x1b[20~','Copy sent');assert b'\x1b]52;' in raw
   send(b'\x1b[15~','Why');send(b'\x1b','What would you want')
   send(b'\x1bOS','Original 1 / dump');send(b'\x1b','What would you want')
   assert 'F10 menu' in screen.text().splitlines()[-1]
   send(b'\x1bOP','Right now');os.write(master,b'b');os.write(master,b'\x1b')
   # The reading is visible behind help; wait for Escape to close help before another key.
   await_condition(lambda s:'Right now' not in s and 'I think you mean' in s)
   assert 'F10 menu' not in screen.text().splitlines()[-1]
   send(b'\x1b[1;2Q','add more')
   send(b'\x1b[200~Also fix a welcome-email typo.\x1b[201~','welcome-email typo')
   send(b'\x1bOQ','Should these added words')
   assert 'Also fix a welcome-email typo.' in screen.text(),screen.text()
   calls=[json.loads(line) for line in (cwd/'calls.jsonl').read_text().splitlines()]
   assert len([r for r in calls if not r.get('kind')])==1,'Addition was sent before scope answer'
   send(b'\x1bOR','type confirm');assert 'Still open' in screen.text()
   # Click the quiet Back action, never Enter/confirm.
   col=w*48//100+3;row=h-2
   send(f'\x1b[<0;{col};{row}M'.encode(),'Should these added words')
   send(b'\x1b[21~','Back to menu and lose this draft? y / n');assert process.poll() is None
   os.write(master,b'n');await_condition(lambda s:'lose this draft?' not in s)
   send(b'\x1b[21~','Back to menu and lose this draft? y / n');os.write(master,b'\x1b[21~');process.wait(timeout=5);assert process.returncode==0
   while select.select([master],[],[],.02)[0]:raw.extend(os.read(master,65536))
   assert not (cwd/'TEST-sessions').exists(),'Implicit handoff'
   assert b'\x1b[?2004l' in raw and b'\x1b[?1006l' in raw
   after=termios.tcgetattr(slave);original[3]&=~getattr(termios,'PENDIN',0);after[3]&=~getattr(termios,'PENDIN',0);assert after==original
   print(json.dumps(dict(width=w,height=h,provider='stub',journey='empty send and recovery/one-frame uncertain reading/F1-F10/Shift-F2/skip undo/copy/focus/keybar/scope/back/guard/restore',confirmation=False)))
  finally:
   if process.poll() is None:process.kill();process.wait()
   os.close(master);os.close(slave)
def exit_during_shaping(w,h):
 # Leaving while a shaping call is in flight must stop the owned provider process.
 with tempfile.TemporaryDirectory(prefix='tinkery-TEST-dogfood-') as d:
  cwd=Path(d);host=cwd/'pi';host.write_text(HOST);host.chmod(0o755)
  master,slave=os.openpty();fcntl.ioctl(slave,termios.TIOCSWINSZ,struct.pack('HHHH',h,w,0,0));screen=Screen(w,h)
  process=subprocess.Popen([str(BINARY),'--shape-pi', '--shape-budget', '64','--record-root',str(cwd/'TEST-records'),'--model','test/model','--pi-command',str(host),'--seed-session-root',str(cwd/'TEST-sessions')],stdin=slave,stdout=slave,stderr=slave,cwd=cwd,env=subprocess.environment({**os.environ,'TERM':'xterm-256color'}))
  pid=cwd/'shaping.pid'
  def await_condition(check):
   end=time.monotonic()+6
   while time.monotonic()<end:
    if select.select([master],[],[],.02)[0]:screen.feed(os.read(master,65536))
    if check():return
    if process.poll() is not None:break
   raise AssertionError(screen.text())
  try:
   await_condition(lambda:"What's on your mind?" in screen.text())
   os.write(master,b'Slow books read this year.');await_condition(lambda:'Slow books' in screen.text())
   os.write(master,b'\x1bOQ');await_condition(pid.exists)
   os.write(master,b'\x1b[21~');await_condition(lambda:'lose this draft?' in screen.text())
   os.write(master,b'\x1b[21~');process.wait(timeout=5);assert process.returncode==0
   try:os.kill(int(pid.read_text()),0)
   except ProcessLookupError:pass
   else:raise AssertionError('Owned shaping call survived exit')
   print(json.dumps(dict(width=w,height=h,provider='stub',journey='exit during shaping stops the owned call',confirmation=False)))
  finally:
   if process.poll() is None:process.kill();process.wait()
   if pid.exists():
    try:os.kill(int(pid.read_text()),signal.SIGTERM)
    except ProcessLookupError:pass
   os.close(master);os.close(slave)
for size in [(80,24),(100,30),(160,40),(320,40)]:journey(*size)
exit_during_shaping(100,30)
