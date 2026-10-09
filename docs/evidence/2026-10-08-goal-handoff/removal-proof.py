from pathlib import Path
import os,shutil,subprocess,tempfile
repo=Path(__file__).resolve().parents[3];root=Path(tempfile.mkdtemp(prefix='tinkery-goal-removals-'))
for name in ['Cargo.toml','Cargo.lock','src','vendor','fixtures']:
 s=repo/name
 if s.is_dir():shutil.copytree(s,root/name)
 else:shutil.copy2(s,root/name)
paths={name:root/'src/shaping/brain_dump'/name for name in ['goal_view.rs','meaning_check.rs']};paths['brain_dump.rs']=root/'src/shaping/brain_dump.rs'
original={name:p.read_text() for name,p in paths.items()}
env={**os.environ,'CARGO_TARGET_DIR':str(repo/'target/goal-removals')}
def run(name):
 p=subprocess.run(['cargo','test','--locked','--lib',name],cwd=root,env=env,capture_output=True,text=True);print(p.stdout+p.stderr,flush=True);return p
assert run('goal_tests').returncode==0
cases=[
 ('implicit-enter','goal_view.rs','review.input=="confirm" && review.scroll>=review.max','true','goal_review_lists_unresolved_questions_and_requires_distinct_full_review_affirmation'),
 ('omit-open-questions','goal_view.rs','let questions = if review.affirmation.unresolved.is_empty()','let questions = if true','goal_review_lists_unresolved_questions_and_requires_distinct_full_review_affirmation'),
 ('allow-unseen-questions','goal_view.rs','review.input=="confirm" && review.scroll>=review.max','review.input=="confirm"','unviewed_long_review_cannot_be_affirmed_and_skipped_questions_remain_visible'),
 ('allow-ambiguous-local-text','goal_view.rs','|| !self.input.text.is_empty()','|| false','confirmation_is_unavailable_for_practice_pending_sources_or_unsent_text'),
 ('permit-false-fork','meaning_check.rs','!verdict.missing.is_empty() || verdict.false_choice','false','meaning_audit_rejects_literal_noun_loss_false_choices_and_bad_evidence'),
 ('omit-literal-acronym-gate','goal_view.rs','if !meaning_check::retains(&g.framings[self.reading].text, &term)','if false','meaning_audit_rejects_literal_noun_loss_false_choices_and_bad_evidence'),
 ('keep-two-after-answer','brain_dump.rs','if !request.settled.is_empty() {\n                1\n            } else if self.uncertain','if self.uncertain','answered_scope_requires_one_combined_reading_and_allows_no_manufactured_options'),
]
for name,file,old,new,test in cases:
 for f,s in original.items():paths[f].write_text(s)
 assert original[file].count(old)==1,(name,'finder mismatch')
 paths[file].write_text(original[file].replace(old,new));p=run(test)
 assert p.returncode!=0 and 'FAILED' in p.stdout and 'could not compile' not in p.stderr,(name,'not proven')
 print('PROVEN:',name,flush=True)
print('PASS: seven baseline-verified independent removals, isolated source:',root,flush=True)
