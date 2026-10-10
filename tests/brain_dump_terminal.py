"""Plain POSIX PTY: no Herdr routing/pacing, no actual provider charges."""
import fcntl, json, os, select, struct, subprocess, tempfile, termios, time
from pathlib import Path
from vt_screen import Screen
BINARY=Path(__file__).resolve().parents[1]/'target/debug/tinkery'
HOST='''#!/usr/bin/env python3
import json,sys,time
assert all(f in sys.argv for f in ['--no-session','--no-tools','--no-context-files','--no-extensions','--no-mcp'])
r=json.load(sys.stdin)
if r.get('kind')=='meaning-preservation':
 print(json.dumps(dict(missing=[],false_choice=False)));sys.exit(0)
if r.get('kind')=='question-continuity':
 print(json.dumps(dict(repeated=[])));sys.exit(0)
with open('requests.jsonl','a') as f: f.write(json.dumps(r)+'\\n')
if 'FAIL' in r['sources'][-1]['text']: print('bad JSON');sys.exit(0)
if any('WAIT' in s['text'] for s in r['sources']): time.sleep(10)
anchors=[dict(source=s['id'],quote=s['text'],occurrence=0) for s in r['sources']]
q='context' if 'meaning' in r['answered'] else 'meaning'
print(json.dumps(dict(uncertain=False,framings=[dict(text='Keep review evidence understandable between handovers.',supports=anchors)],outcome='You can distinguish observed evidence from an unreviewed suggestion in a PR thread.',misfits=[],questions=[dict(id=q,text='What would make a handover trustworthy?' if q=='context' else 'Which evidence gets confused?'),dict(id='boundary',text='Who needs this distinction?')],alternatives=[dict(label=l,benefit='Clearer handovers.',cost='Check the workflow.',undo_cost='Unknown.') for l in ['A small receipt / candidate','Link existing evidence / candidate']])))
'''
def journey(width,height):
 with tempfile.TemporaryDirectory(prefix='tinkery-brain-pty-') as directory:
  cwd=Path(directory);host=cwd/'pi';host.write_text(HOST);host.chmod(0o755);skill=cwd/'SKILL.md';skill.write_text('# Seed Me\n### Shape the working draft\nUnderstand intent.\n### Size gate\n')
  master,slave=os.openpty();fcntl.ioctl(slave,termios.TIOCSWINSZ,struct.pack('HHHH',height,width,0,0));original=termios.tcgetattr(slave);screen=Screen(width,height);out=bytearray()
  process=subprocess.Popen([str(BINARY),'--shape-pi','--model','test/model','--seed-me',str(skill),'--pi-command',str(host)],stdin=slave,stdout=slave,stderr=slave,cwd=cwd,env={**os.environ,'TERM':'xterm-256color'})
  def await_text(text,timeout=5):
   deadline=time.monotonic()+timeout
   while time.monotonic()<deadline:
    if select.select([master],[],[],0.02)[0]:
     chunk=os.read(master,65536);out.extend(chunk);screen.feed(chunk)
    if text in screen.text() and not screen.synchronized:
     assert screen.cells[1][2:9]==list('tinkery') and screen.cells[1][width-3]=='?',screen.text()
     return
    if process.poll() is not None:break
   raise AssertionError(f'Missing {text!r}:\n{screen.text()}')
  def absent(text):
   deadline=time.monotonic()+5
   while time.monotonic()<deadline:
    if select.select([master],[],[],0.02)[0]:screen.feed(os.read(master,65536))
    if text not in screen.text():return
   raise AssertionError(f'Stale help still visible: {text}')
  def send(data,text):os.write(master,data);await_text(text)
  def notice(data,text):
   os.write(master,data)
   if data==b'\x1b':await_text("Reading couldn't be updated.")
   os.write(master,b'\x1bOP');await_text('Right now')
   send(b'h','How it works')
   os.write(master,b'\x1b[6~'*100);await_text(text)
   os.write(master,b'\x1bOP');absent('Last result')
   if 'Keep review evidence' not in screen.text():os.write(master,b'\x1b[6~')
   await_text('Keep review evidence')
  def requests():return [json.loads(line) for line in (cwd/'requests.jsonl').read_text().splitlines()] if (cwd/'requests.jsonl').exists() else []
  try:
   await_text("What's on your mind?");assert 'I think this is about' not in screen.text();assert not requests()
   text='Typed evidence should appear promptly without waiting for a provider call.'
   start=time.monotonic();send(text.encode(),'call.');burst=time.monotonic()-start;assert burst<1.0,(width,burst)
   assert not requests();send(b'\x1bOQ','Which evidence gets confused?');assert len(requests())==1
   assert requests()[0]['sources'][0]['text']==text
   assert not requests()[0]['fragments'];await_text('Typed evidence');await_text('Keep review evidence');assert 'Ctrl-O originals' not in screen.text()
   send(b'\x04','Desired experience');assert len(requests())==1
   send(b'\x04','Keep review evidence');assert len(requests())==1
   # Input and F2 submit; the result must not replace text entered during the request.
   answer='A real model capture is different from a stub or a human review.'
   start=time.monotonic();send(answer.encode(),'review.');answer_burst=time.monotonic()-start;assert answer_burst<1.0
   send(b'\x1bOQ','What would make a handover');await_text('Reading');assert len(requests())==2;assert requests()[1]['sources'][1]['in_reply_to']=='meaning'
   send(b'\x1b[15~','Desired experience');send(b'\x1b[15~','Keep review evidence');assert len(requests())==2
   # A deliberate source selection extracts one card; neither action calls the provider.
   send(b'\x1b[5;5~','Typed evidence');os.write(master,b'\x1b[<0;3;5M\x1b[<0;3;5m');time.sleep(.1);notice(b'\x1b[1;2C'*5+b'\x05','Extracted by you');assert len(requests())==2
   send(b'e','Typed');send(b'\r','Typed evidence');assert len(requests())==2
   notice(b'y','Copy sent');assert screen.clipboards[-1].startswith('# Tinkery / provisional board');assert text in screen.clipboards[-1] and answer in screen.clipboards[-1]
   send(b'\x1bOS','Original 1');send(b'\x04','Desired experience');send(b'\x04','Keep review evidence');send(b'\x1bOS','Original 1');send(b'\x1b','Keep review evidence')
   # Character-by-character injection, with 1ms pacing (not a paste event).
   slow=' Streamed letters stay responsive.';start=time.monotonic()
   for c in slow:os.write(master,c.encode());time.sleep(0.001)
   await_text('responsive.');stream=time.monotonic()-start;assert stream<1.0,(width,stream)
   send(b'\x15FAIL','FAIL');send(b'\x1bOQ',"Reading couldn't be updated.");notice(b'', 'Reshape failed');assert requests()[2]['fragments'][0]['text']=='Typed';assert requests()[2]['fragments'][0]['start']==0 and requests()[2]['fragments'][0]['end']==5;assert 'Keep review evidence' in screen.text();send(b'\x0f','Original 3');send(b'\x1b','Keep review evidence')
   send(b'\x0eWAIT','WAIT');send(b'\x1bOQ','Should these added words');send(b'include','include');send(b'\x1bOQ','Thinking…');notice(b'\x1b','Cancelled; original');assert 'Keep review evidence' in screen.text()
   send(b'\x1b[21~','Back to menu and lose this draft? y / n');assert process.poll() is None
   send(b'n',"Reading couldn't be updated.");absent('lose this draft?')
   send(b'\x03',"Reading couldn't be updated.");absent('lose this draft');send(b'\x1b[21~','Back to menu and lose this draft? y / n');os.write(master,b'\x1b[21~');process.wait(timeout=5);assert process.returncode==0
   while select.select([master],[],[],0.02)[0]:
    chunk=os.read(master,65536)
    if not chunk:break
    out.extend(chunk)
   assert b'\x1b[?2004l' in out and b'\x1b[?1006l' in out and b'\x1b[>0s' in out
   after=termios.tcgetattr(slave);original[3]&=~getattr(termios,'PENDIN',0);after[3]&=~getattr(termios,'PENDIN',0);assert after==original
   assert {p.name for p in cwd.iterdir()}=={'pi','SKILL.md','requests.jsonl'},'Unexpected persistence'
   print(json.dumps(dict(width=width,height=height,burst_seconds=round(burst,4),answer_burst_seconds=round(answer_burst,4),paced_1ms_seconds=round(stream,4),journey='submit/answer/original/copy/failure/cancel/restore',provider='stub')))
  finally:
   if process.poll() is None:process.kill();process.wait()
   os.close(master);os.close(slave)
for size in [(80,24),(100,30),(160,40),(320,40)]:journey(*size)
