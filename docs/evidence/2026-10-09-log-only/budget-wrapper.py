#!/usr/bin/env python3
import fcntl,json,os,pathlib,shutil,sys,time
root=pathlib.Path(os.environ['TINKERY_EVAL_TRACE'])
root.mkdir(exist_ok=True)
data=sys.stdin.buffer.read()
with (root/'budget.lock').open('a+') as lock:
 fcntl.flock(lock,fcntl.LOCK_EX)
 calls=root/'calls.jsonl'
 previous=calls.read_text().splitlines() if calls.exists() else []
 if len(previous)>=6: raise SystemExit('Approved six-call budget exhausted; no Pi invocation')
 n=len(previous)
 kind=json.loads(data).get('kind','shape')
 (root/f'{n:02d}-input.json').write_bytes(data)
 (root/f'{n:02d}-argv.json').write_text(json.dumps(sys.argv[1:],indent=2)+'\n')
 with calls.open('a') as log:log.write(json.dumps(dict(index=n,kind=kind,started_ns=time.time_ns(),actor='assistant-driven TEST',thinking_override='--thinking' in sys.argv))+'\n')
original=(root/f'{n:02d}-input.json').open('rb')
os.dup2(original.fileno(),0)
pi=shutil.which('pi')
if not pi:raise SystemExit('Pi unavailable; no provider request')
os.execv(pi,[pi,*sys.argv[1:]])
