from pathlib import Path
import os,shutil,subprocess,tempfile,json
repo=Path('/Users/jondev/dev/active/tinkery');root=Path(tempfile.mkdtemp(prefix='tinkery-TEST-log-only-removals-'))
for name in ['Cargo.toml','Cargo.lock','src','vendor','fixtures']:
 s=repo/name
 if s.is_dir():shutil.copytree(s,root/name)
 else:shutil.copy2(s,root/name)
paths={'board':root/'src/shaping/brain_dump.rs','view':root/'src/shaping/brain_dump/yohaku.rs','goal':root/'src/shaping/brain_dump/goal_view.rs','checks':root/'src/shaping/brain_dump/checks.rs'}
original={n:p.read_text() for n,p in paths.items()}
env={**os.environ,'CARGO_TARGET_DIR':str(repo/'target/log-only-removals')}
def run(test,name):
 for p in (root/'src').rglob('*.rs'):os.utime(p,None)
 r=subprocess.run(['cargo','test','--locked','--lib',test],cwd=root,env=env,capture_output=True,text=True,timeout=100)
 (root/(name+'.log')).write_text(r.stdout+r.stderr);print(name,r.returncode,flush=True);return r
assert run('brain_dump','baseline').returncode==0
cases=[
 ('remove-advice','board','    fn has_advisory(&self) -> bool {\n        true\n    }','    fn has_advisory(&self) -> bool { false }','concern_loss_is_logged'),
 ('semantic-veto-review','goal','        let mut unresolved = g','        if !g.framings[self.reading].text.contains("PR") {return;}\n        let mut unresolved = g','meaning_findings_are_log_only'),
 ('force-second-reading','board','!(1..=2).contains(&self.framings.len())','self.framings.len()!=if self.uncertain{2}else{1}','boundary_rejects_unsafe_unsupported_excess'),
 ('absorb-unanswered-addition','board','            if addition {','            if false && addition {','added_words_are_visible_unresolved'),
 ('hide-new-source','board','            self.source_view = self.sources.len() - 1;','            self.source_view = 0;','added_words_are_visible_unresolved'),
 ('leak-skipped-addition','board','            request.sources.retain(|s| s.id != *id);','            request.sources.retain(|_| true);','skipping_scope_does_not_include'),
 ('lose-undone-question-on-result','board','                self.guess = Some(g);','                self.restored_questions.clear();\n                self.guess = Some(g);','undone_skip_survives_a_later'),
 ('default-bar-on','board','            key_bar: false,','            key_bar: true,','optional_bar_uses_reserved_row'),
 ('discard-without-guard','board','                self.request_leave();','                self.quit = true;','exit_guard_keeps_unsent'),
 ('omit-header','board','    yohaku::render_header(frame, palette);','','source_wrap_positions_header'),
 ('consume-over-cap-input','board','        if bytes > 32768 {\n            self.notice = format!(', '        if false && bytes > 32768 {\n            self.notice = format!(','encoded_cap_counts_metadata'),
 ('remove-back','goal','Paragraph::new("Back")','Paragraph::new("")','optional_bar_uses_reserved_row'),
 ('hide-rejected-attempt','checks','if d.decision == "reject"','if d.decision == "never"','rejected_schema_attempt_is_readable'),
 ('auto-shape-again','board','            let text = self.complete(input, prompt, cancelled)?;','            let text = self.complete(input.clone(), prompt.clone(), cancelled)?;\n            let _ = self.complete(input, prompt, cancelled)?;','one_shape_then_one_log_only'),
]
proven=[]
for name,file,old,new,test in cases:
 for n,s in original.items():paths[n].write_text(s)
 s=original[file];assert s.count(old)==1,(name,'finder mismatch',s.count(old));paths[file].write_text(s.replace(old,new))
 r=run(test,name);assert r.returncode!=0 and 'FAILED' in r.stdout and 'could not compile' not in r.stderr,(name,'not proven');proven.append(name)
(root/'result.json').write_text(json.dumps(dict(baseline=True,proven=proven,scope='isolated source copies/stub providers; not native-model or operator recognition'),indent=2))
print('PASS',len(proven),'baseline-verified removal/reversion probes',root,flush=True)
