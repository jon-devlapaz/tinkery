#!/usr/bin/env python3
"""Operator-approved revision: revision 2: hard 24 new calls; no extension without a new Claude-session message."""
import fcntl,json,os,pathlib,shutil,sys,time
root=pathlib.Path(os.environ['TINKERY_C16_TRACE']);root.mkdir(exist_ok=True)
a=sys.argv[1:]
if a[a.index('--model')+1]!='openai-codex/gpt-5.6-luna' or a[a.index('--thinking')+1]!='low':raise SystemExit('Unapproved model/thinking; no Pi invocation')
if not all(s in a for s in ['--no-session','--no-tools','--no-extensions','--no-mcp','--no-approve']):raise SystemExit('Missing isolation flags; no Pi invocation')
data=sys.stdin.buffer.read();kind=json.loads(data).get('kind','shape')
if kind not in ['shape','meaning-preservation']:raise SystemExit('Unapproved call kind; no Pi invocation')
pi=shutil.which('pi')
if not pi:raise SystemExit('Pi unavailable; no provider invocation')
with (root/'budget.lock').open('a+') as lock:
 fcntl.flock(lock,fcntl.LOCK_EX)
 log=root/'calls.jsonl';prior=[json.loads(s) for s in log.read_text().splitlines()] if log.exists() else []
 revision=[c for c in prior if c.get('phase')=='revised-2']
 if len(prior)<59:raise SystemExit('Prior cumulative trace missing; no Pi invocation')
 if len(prior)>=83 or len(revision)>=24 or sum(c['kind']==kind for c in revision)>=24:raise SystemExit('Approved revision/shared budget exhausted; no Pi invocation')
 n=len(prior);(root/f'{n:02}-input.json').write_bytes(data);(root/f'{n:02}-argv.json').write_text(json.dumps(a,indent=2)+'\n')
 with log.open('a') as f:f.write(json.dumps({'index':n,'phase':'revised-2','kind':kind,'started_ns':time.time_ns(),'model':'openai-codex/gpt-5.6-luna','thinking':'low','actor':'assistant-driven TEST revised eval'})+'\n')
input_file=(root/f'{n:02}-input.json').open('rb');os.dup2(input_file.fileno(),0);os.execv(pi,[pi,*a])
