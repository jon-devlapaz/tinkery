#!/usr/bin/env python3
"""Approved Phase 2 eval; copied prior 14-call trace retains the shared ceiling."""
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
 if len(prior)>=40 or sum(c.get('phase')=='after' for c in prior)>=26 or sum(c.get('phase')=='after' and c['kind']==kind for c in prior)>=16:raise SystemExit('Approved after-eval/shared budget exhausted; no Pi invocation')
 n=len(prior);(root/f'{n:02}-input.json').write_bytes(data);(root/f'{n:02}-argv.json').write_text(json.dumps(a,indent=2)+'\n')
 with log.open('a') as f:f.write(json.dumps({'index':n,'phase':'after','kind':kind,'started_ns':time.time_ns(),'model':'openai-codex/gpt-5.6-luna','thinking':'low','actor':'assistant-driven TEST after eval'})+'\n')
input_file=(root/f'{n:02}-input.json').open('rb');os.dup2(input_file.fileno(),0);os.execv(pi,[pi,*a])
