"""Temporary protection reversions. Restore every source even if a probe fails."""
from pathlib import Path
import subprocess
import sys
repo=Path(__file__).resolve().parents[4]
probes=[
 ('consent','src/pi.rs','if used >= limit {','if false && used >= limit {',None),
 ('originals','src/seed_me/connection_save.rs','goal: self.origin.goal.clone(),\n                outcome:', 'goal: "changed words".into(),\n                outcome:', 'restart_reuses_published_goal_before_rechecking_changed_membership'),
 ('attribution','src/shape/step.rs','let creation = crate::seed_me::connection::read(session, goal)?;', 'let creation = crate::seed_me::connection::read(session, goal).ok().flatten();','tend_origin_has_honest_history_and_invalid_evidence_is_distinct'),
 ('duplicate-handoff','src/seed_me/connection_save.rs','if let Some(receipt) = self.inspect()? {\n            return Ok(receipt);\n        }','', 'restart_reuses_published_goal_before_rechecking_changed_membership'),
]
for name,filename,old,new,test in probes:
 path=repo/filename; original=path.read_text(); assert original.count(old)==1,(name,original.count(old))
 try:
  path.write_text(original.replace(old,new))
  if test:
   result=subprocess.run(['cargo','+stable','test','--lib',test,'--','--nocapture'],cwd=repo,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True)
  else:
   subprocess.run(['cargo','+stable','build','--locked','--quiet'],cwd=repo,check=True)
   result=subprocess.run([sys.executable,'-c',"import sys; sys.path.insert(0,'tests'); import tend_connection_terminal as t; t.refusal('good','run limit of 0 reached',budget=0)"],cwd=repo,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True)
  (Path(__file__).parent/f'reversion-{name}.log').write_text(result.stdout)
  assert result.returncode!=0,f'{name}: reverted protection unexpectedly passed'
  assert 'could not compile' not in result.stdout,f'{name}: compilation failure is not evidence'
  print(f'{name}: reverted protection failed as expected',flush=True)
 finally: path.write_text(original)
