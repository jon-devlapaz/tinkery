#!/usr/bin/env python3
"""Fresh scratch-copy regressions; no providers, helpers or protected roots."""
import json,os,pathlib,shutil,subprocess,tempfile
REPO=pathlib.Path(__file__).resolve().parents[4]
OUTPUT=pathlib.Path(__file__).resolve().parent/'probes'
OUTPUT.mkdir(exist_ok=True)
C16='shaping::brain_dump::c16_tests::'
PROBES=[
 ('who-forced-to-i',C16+'beneficiary_and_labelled_lines_preserve_criteria_and_constraints',[('src/shaping/brain_dump/goal.rs','case_first(&who, true), outcome','"I", outcome')]),
 ('duplicate-can',C16+'supplied_can_is_not_duplicated_in_the_goal_line',[('src/shaping/brain_dump/goal.rs','.strip_prefix("can ")','.strip_prefix("unused connective ")'),('src/shaping/brain_dump/goal.rs','.strip_prefix("Can ")','.strip_prefix("Other unused connective ")')]),
 ('capital-after-semicolon',C16+'semicolon_items_are_lowercase_and_quotes_decimals_unicode_survive',[('src/shaping/brain_dump/goal.rs','case_first(&s, false)','case_first(&s, true)')]),
 ('eval-phrase-in-prompt','shaping::brain_dump::tests::board_process_uses_only_shaping_guidance_and_disabled_resources',[('src/shaping/brain_dump.rs','Return JSON: parts','Preserve way off. Return JSON: parts')]),
 ('omit-avoid',C16+'beneficiary_and_labelled_lines_preserve_criteria_and_constraints',[('src/shaping/brain_dump/goal.rs','if let Some(value) = clause(part)','if let Some(value) = if part == Part::Avoid { None } else { clause(part) }')]),
 ('rewrite-criteria',C16+'beneficiary_and_labelled_lines_preserve_criteria_and_constraints',[('src/shaping/brain_dump/goal.rs','lines.join("\\n")','lines.join("\\n").replace("way off", "unusually high")')]),
 ('empty-queue-ready',C16+'readiness_requires_four_parts_optional_constraints_only_block_when_open',[('src/shaping/brain_dump.rs','&& self.open_parts().is_empty()','&& true')]),
 ('third-ask',C16+'at_most_two_asks_then_the_part_stays_open_for_seed',[('src/shaping/brain_dump/board.rs','unwrap_or(0) < 2','unwrap_or(0) < 255'),('src/shaping/brain_dump/board.rs','unwrap_or(0) >= 2','unwrap_or(0) >= 255')]),
 ('stale-counts',C16+'delayed_results_use_current_counts_and_skipped_targets',[('src/shaping/brain_dump.rs','request.ask_counts = self.ask_counts.clone();\n        request.skipped','request.skipped')]),
 ('reasked-skipped-part',C16+'skipped_target_blocks_renamed_questions_and_undo_is_not_another_ask',[('src/shaping/brain_dump/board.rs','request.skipped.iter().any(|s| s.target == Some(p))\n                ||','false\n                ||')]),
 ('scope-instead-of-refinement',C16+'typing_after_ready_refines_goal_with_verbatim_reply_and_explicit_f2',[('src/shaping/brain_dump.rs','let refinement = !self.add_more && self.goal_ready_without_local_text();','let refinement = false;')]),
 ('independent-goal',C16+'quiet_review_freezes_exact_block_and_seed_mapping_without_duplicate_lines',[('src/shaping/brain_dump/goal_view.rs','Affirmation{goal,parts:','Affirmation{goal:g.parts.outcome.clone().unwrap_or_default(),parts:')]),
 ('wrong-outcome-mapping',C16+'quiet_review_freezes_exact_block_and_seed_mapping_without_duplicate_lines',[('src/shaping/brain_dump/goal_view.rs','outcome:g.parts.done_when.clone().unwrap_or_default()','outcome:g.parts.outcome.clone().unwrap_or_default()')]),
 ('comma-inside-data',C16+'trailing_commas_normalize_only_outside_strings_and_span_guards_remain',[('src/shaping/brain_dump/board.rs',"if !quoted\n            && byte == b','", "if byte == b','")]),
]
with tempfile.TemporaryDirectory(prefix='tinkery-TEST-C16-revised-probes-') as temp:
 work=pathlib.Path(temp)/'repo';shutil.copytree(REPO,work,ignore=shutil.ignore_patterns('.git','target','__pycache__','2026-10-09-goal-shape'))
 env={**os.environ,'CARGO_TARGET_DIR':str(pathlib.Path(temp)/'target')};results=[]
 def run(label,test):
  cmd=['cargo','+1.99.0','test','--lib','--locked','--offline',test,'--','--exact'];p=subprocess.run(cmd,cwd=work,env=env,text=True,capture_output=True);(OUTPUT/(label+'.log')).write_text(p.stdout+p.stderr);return p,cmd
 for label,test,edits in PROBES:
  original={p:(work/p).read_text() for p,_,_ in edits}
  baseline,cmd=run(label+'-baseline',test);assert baseline.returncode==0 and '1 passed' in baseline.stdout,(label,'baseline failed',baseline.stderr)
  for path,old,new in edits:
   text=(work/path).read_text();assert text.count(old)==1,(label,path,old,text.count(old));(work/path).write_text(text.replace(old,new))
  mutant,cmd=run(label,test);assert mutant.returncode!=0 and 'test result: FAILED' in mutant.stdout,(label,'mutation did not fail the test',mutant.stdout,mutant.stderr)
  results.append({'probe':label,'baseline_exit':baseline.returncode,'mutant_exit':mutant.returncode,'command':cmd,'protection_failed_when_reverted':True});print(label,'baseline passed; mutant failed',flush=True)
  for path,text in original.items():(work/path).write_text(text)
 (OUTPUT/'results.json').write_text(json.dumps(results,indent=2)+'\n')
