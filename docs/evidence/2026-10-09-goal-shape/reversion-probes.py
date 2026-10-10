#!/usr/bin/env python3
"""Independent scratch-copy regressions. No provider or real helper/session work."""
import json,os,pathlib,shutil,subprocess,tempfile
REPO=pathlib.Path(__file__).resolve().parents[3]
OUTPUT=REPO/'docs/evidence/2026-10-09-goal-shape/probes'
OUTPUT.mkdir(exist_ok=True)
PROBES=[
 ('reasked-skipped-part','skipped_target_blocks_a_renamed_and_reworded_model_question',[('src/shaping/brain_dump/board.rs','request.skipped.iter().any(|s| s.target == Some(p))\n                ||','false\n                ||')]),
 ('extra-sentences','full_person_voice_and_multiple_clauses_keep_words_without_duplicate_prefixes',[('src/shaping/brain_dump/goal.rs',"out.push(';');","out.push(c);")]),
 ('comma-inside-data','trailing_commas_normalize_only_outside_strings_and_preserve_claimed_span_guards',[('src/shaping/brain_dump/board.rs',"if !quoted\n            && byte == b','", "if byte == b','")]),
 ('omit-boundaries','complete_composition_has_two_sentences_and_preserves_criteria_and_boundaries',[('src/shaping/brain_dump/goal.rs','if let Some(boundaries) = clause(Part::Boundaries)', 'if let Some(boundaries) = None::<&str>')]),
 ('rewrite-criteria','complete_composition_has_two_sentences_and_preserves_criteria_and_boundaries',[('src/shaping/brain_dump/goal.rs','sentences.join(" ")','sentences.join(" ").replace("way off", "unusually high")')]),
 ('empty-queue-ready','readiness_is_all_five_present_and_not_vague_skipped_or_pending_scope',[('src/shaping/brain_dump.rs','&& self.open_parts().is_empty()','&& true')]),
 ('third-ask','at_most_two_asks_then_the_part_stays_open_for_seed',[('src/shaping/brain_dump/board.rs','unwrap_or(0) < 2','unwrap_or(0) < 255'),('src/shaping/brain_dump/board.rs','unwrap_or(0) >= 2','unwrap_or(0) >= 255')]),
 ('stale-counts','delayed_results_cannot_bypass_current_ask_counts_or_skipped_targets',[('src/shaping/brain_dump.rs','request.ask_counts = self.ask_counts.clone();\n        request.skipped','request.skipped')]),
 ('independent-goal','quiet_review_freezes_present_parts_and_identical_goal_without_missing_lines',[('src/shaping/brain_dump/goal_view.rs','Affirmation{goal,outcome:','Affirmation{goal:g.parts.outcome.clone().unwrap_or_default(),outcome:')]),
]
with tempfile.TemporaryDirectory(prefix='tinkery-TEST-C16-probes-') as temp:
 work=pathlib.Path(temp)/'repo';shutil.copytree(REPO,work,ignore=shutil.ignore_patterns('.git','target','__pycache__','2026-10-09-goal-shape'))
 env={**os.environ,'CARGO_TARGET_DIR':str(pathlib.Path(temp)/'target')};results=[]
 def run(label,test):
  cmd=['cargo','test','--lib','--locked','--offline','shaping::brain_dump::c16_tests::'+test,'--','--exact'];p=subprocess.run(cmd,cwd=work,env=env,text=True,capture_output=True);(OUTPUT/(label+'.log')).write_text(p.stdout+p.stderr);return p,cmd
 for label,test,edits in PROBES:
  original={p:(work/p).read_text() for p,_,_ in edits}
  baseline,cmd=run(label+'-baseline',test);assert baseline.returncode==0,(label,'baseline failed',baseline.stderr)
  for path,old,new in edits:
   text=(work/path).read_text();assert text.count(old)==1,(label,path,old,text.count(old));(work/path).write_text(text.replace(old,new))
  mutant,cmd=run(label,test);assert mutant.returncode!=0 and 'test result: FAILED' in mutant.stdout,(label,'mutation did not fail the test',mutant.stdout,mutant.stderr)
  results.append({'probe':label,'baseline_exit':baseline.returncode,'mutant_exit':mutant.returncode,'command':cmd,'protection_failed_when_reverted':True});print(label,'baseline passed; mutant failed',flush=True)
  for path,text in original.items():(work/path).write_text(text)
 (OUTPUT/'results.json').write_text(json.dumps(results,indent=2)+'\n')
