#!/usr/bin/env python3
"""One approved rerun: maximum three shape and three log-only advisory invocations."""
import fcntl
import json
import os
from pathlib import Path
import shutil
import sys
import time

root=Path(os.environ['TINKERY_EVAL_TRACE'])
root.mkdir(exist_ok=True)
args=sys.argv[1:]
if '--thinking' in args or args[args.index('--model')+1]!='openai-codex/gpt-5.6-luna':
    raise SystemExit('Refuse unapproved model or thinking override')
if not all(flag in args for flag in ('--no-session','--no-tools','--no-extensions','--no-mcp','--no-approve')):
    raise SystemExit('Refuse authority-capable eval invocation')
pi=shutil.which('pi')
if not pi:
    raise SystemExit('Pi unavailable; no provider request')
data=sys.stdin.buffer.read()
kind=json.loads(data).get('kind','shape')
if kind not in ('shape','meaning-preservation'):
    raise SystemExit('Refuse unapproved eval call kind')
with (root/'budget.lock').open('a+') as lock:
    fcntl.flock(lock,fcntl.LOCK_EX)
    calls=root/'calls.jsonl'
    previous=[json.loads(line) for line in calls.read_text().splitlines()] if calls.exists() else []
    if len(previous)>=6 or sum(call['kind']==kind for call in previous)>=3:
        raise SystemExit('Approved three-shape/three-advisory budget exhausted; no Pi invocation')
    n=len(previous)
    (root/f'{n:02d}-input.json').write_bytes(data)
    (root/f'{n:02d}-argv.json').write_text(json.dumps(args,indent=2)+'\n')
    with calls.open('a') as log:
        log.write(json.dumps(dict(index=n,kind=kind,started_ns=time.time_ns(),actor='assistant-driven TEST',thinking_override=False))+'\n')
original=(root/f'{n:02d}-input.json').open('rb')
os.dup2(original.fileno(),0)
os.execv(pi,[pi,*args])
