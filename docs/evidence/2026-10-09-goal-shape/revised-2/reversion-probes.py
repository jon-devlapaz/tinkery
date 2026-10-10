#!/usr/bin/env python3
"""Fresh scratch-copy regressions; no providers, helpers or protected roots."""
import json,os,pathlib,shutil,subprocess,tempfile
REPO=pathlib.Path(__file__).resolve().parents[4]
OUTPUT=pathlib.Path(__file__).resolve().parent/'probes'
OUTPUT.mkdir(exist_ok=True)
R2='shaping::brain_dump::revision2_tests::'
PROBES=[
 ('o1-forced-can',R2+'outcome_is_a_behaviour_without_forced_can',[('src/shaping/brain_dump/goal.rs','format!("{who} {outcome}")','format!("{who} can {outcome}")')]),
 ('o3-repeated-check',R2+'repeated_check_is_missing_and_required_question_precedes_vague_outcome',[('src/shaping/brain_dump/goal.rs','self.done_when = None;','let _ = done;')]),
 ('o6-timing-duplication',R2+'timing_and_capacity_are_not_duplicated_and_constraints_each_get_a_line',[('src/shaping/brain_dump/goal.rs','&& !identity(&goal).contains(&identity(&when))','&& true')]),
 ('o6-subject-duplication',R2+'repeated_subjects_and_uncited_supports_are_not_fabricated_citations',[('src/shaping/brain_dump/goal.rs','identity(prefix) == identity(&raw_who)','false')]),
 ('s6-muting-without-change',R2+'unchanged_answer_never_mutes_the_whole_block',[('src/shaping/brain_dump.rs','(current != previous).then_some(changed)','Some(changed)')]),
 ('s6-no-color-ink',R2+'changed_lines_ink_unchanged_muted_until_any_keystroke_paste_or_send',[('src/shaping/brain_dump/yohaku.rs','.style(palette.ink.add_modifier(Modifier::ITALIC))','.style(palette.muted.add_modifier(Modifier::ITALIC))')]),
 ('q1-required-priority',R2+'repeated_check_is_missing_and_required_question_precedes_vague_outcome',[('src/shaping/brain_dump/board.rs','.retain(|q| q.target.is_none_or(|p| p == missing));','.retain(|_| true);')]),
 ('s3-open-not-ready',R2+'open_notes_cannot_contradict_ready_and_self_labels_never_render',[('src/shaping/brain_dump.rs','g.unresolved_notes.is_empty() && g.misfits.is_empty()','true')]),
 ('o4-one-constraint-per-line',R2+'timing_and_capacity_are_not_duplicated_and_constraints_each_get_a_line',[('src/shaping/brain_dump/goal.rs','values.iter().flat_map(|s| clauses(s))','values.iter().cloned()')]),
 ('o9-deferred-retained',R2+'quiet_asides_survive_paper_review_and_are_not_constraints',[('src/shaping/brain_dump/goal_view.rs','deferred: g.deferred.clone()','deferred: vec![]')]),
 ('s3-no-false-ready-message',R2+'open_notes_cannot_contradict_ready_and_self_labels_never_render',[('src/shaping/brain_dump/yohaku.rs','None if app.goal_ready() =>','None if app.guess.is_some() =>')]),
 ('s2-no-question-composer',R2+'open_notes_cannot_contradict_ready_and_self_labels_never_render',[('src/shaping/brain_dump/yohaku.rs','app.focused_question().is_none() && app.guess.is_some()','app.focused_question().is_none() && app.goal_ready_without_local_text()')]),
 ('history-before-priority',R2+'fact_and_owned_history_guards_run_before_question_priority_filtering',[('src/shaping/brain_dump/board.rs','pub fn verify(mut g: Guess, request: &BoardRequest) -> Result<Self, String> {\n        g.validate(request)?;','pub fn verify(mut g: Guess, request: &BoardRequest) -> Result<Self, String> {')]),
 ('eval-phrase-in-prompt','shaping::brain_dump::tests::board_process_uses_only_shaping_guidance_and_disabled_resources',[('src/shaping/brain_dump.rs','Return JSON: parts','Preserve way off. Return JSON: parts')]),
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
