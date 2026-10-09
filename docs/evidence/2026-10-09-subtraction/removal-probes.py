"""Fresh-baseline, isolated-copy probes; all provider use is stubbed."""
from pathlib import Path
import argparse,json,os,shutil,subprocess,tempfile
parser=argparse.ArgumentParser();parser.add_argument('--repo',type=Path,required=True);args=parser.parse_args()
repo=args.repo.resolve();root=Path(tempfile.mkdtemp(prefix='tinkery-TEST-subtraction-removals-'))
for name in ['Cargo.toml','Cargo.lock','src','vendor','fixtures','docs']:
    source=repo/name
    if source.is_dir():shutil.copytree(source,root/name)
    else:shutil.copy2(source,root/name)
paths={name:root/path for name,path in {'app':'src/shaping/brain_dump.rs','boundary':'src/shaping/brain_dump/board.rs','view':'src/shaping/brain_dump/yohaku.rs','goal':'src/shaping/brain_dump/goal_view.rs','checks':'src/shaping/brain_dump/checks.rs'}.items()}
original={name:path.read_text() for name,path in paths.items()}
env={**os.environ,'CARGO_TARGET_DIR':str(repo/'target/subtraction-removals')}
def run(test,name):
    for path in (root/'src').rglob('*.rs'):os.utime(path,None)
    result=subprocess.run(['cargo','test','--locked','--lib',test],cwd=root,env=env,capture_output=True,text=True,timeout=180)
    (root/(name+'.log')).write_text(result.stdout+result.stderr);print(name,result.returncode,flush=True);return result
assert run('brain_dump','baseline').returncode==0
# Each tuple is one independent protection reversion; never accumulate mutations.
cases=[
('extra-field-veto',[('boundary','    for key in obj.keys().filter','    if obj.keys().any(|key|!known.contains(&key.as_str())) {panic!("Restored extra-field veto");}\n    for key in obj.keys().filter')],'omissions_extras'),
('word-limit-veto',[('app','        for frame in &self.framings {','        if self.framings.iter().any(|f|f.text.split_whitespace().count()>45){return Err("Restored word limit".into());}\n        for frame in &self.framings {')],'counts_word_lengths'),
('count-coupling-veto',[('app','        for frame in &self.framings {','        if self.framings.len()>2 || self.questions.len()>6 || self.alternatives.len()>4 {return Err("Restored count veto".into());}\n        for frame in &self.framings {')],'counts_word_lengths'),
('forced-citations',[('app','        for frame in &self.framings {','        if self.framings.iter().any(|f|f.supports.is_empty()){return Err("Restored citation minimum".into());}\n        for frame in &self.framings {')],'omissions_extras'),
('narrative-note-veto',[('boundary','            unresolved_notes.push(note);','            let _=note;return Err("Restored narrative-note veto".into());\n            #[allow(unreachable_code)] unresolved_notes.push(String::new());')],'omissions_extras'),
('source-id-guard',[('app','.ok_or("Unknown annotation source")?;','.unwrap_or(&sources[0]);')],'claimed_citations'),
('exact-quote-guard',[('app','.ok_or("Annotation is not an exact source substring at that occurrence")?;','.unwrap_or(0);')],'claimed_citations'),
('grapheme-guard',[('app','        if !grapheme_boundary(&source.text, start) || !grapheme_boundary(&source.text, end) {','        if false && (!grapheme_boundary(&source.text, start) || !grapheme_boundary(&source.text, end)) {')],'claimed_citations'),
('scope-id-guard',[('boundary','        if q.id.starts_with("scope-addition-") {','        if false && q.id.starts_with("scope-addition-") {'),('app','            if q.id.starts_with("scope-addition-")','            if false && q.id.starts_with("scope-addition-")')],'local_history_and_scope'),
('answered-history-guard',[('boundary','            || request.answered.contains(&q.id)','            || false && request.answered.contains(&q.id)'),('app','                || request.answered.contains(&q.id)','                || false && request.answered.contains(&q.id)')],'local_history_and_scope'),
('last-board-retention',[('app','                self.last_failure = Some(e.clone());','                self.guess=None;\n                self.last_failure = Some(e.clone());')],'bad_span_keeps_the_previous'),
('invent-narrative-citation',[('boundary','            unresolved_notes.push(note);','            misfits.push(Anchor{source:1,quote:"Exact".into(),occurrence:0});\n            unresolved_notes.push(note);')],'omissions_extras'),
('remove-advice',[('app','    fn has_advisory(&self) -> bool {\n        true\n    }','    fn has_advisory(&self) -> bool { false }')],'concern_loss_is_logged'),
('semantic-review-veto',[('goal','        let mut unresolved = g','        if !g.framings[self.reading].text.contains("PR") {return;}\n        let mut unresolved = g')],'meaning_findings_are_log_only'),
('absorb-unanswered-addition',[('app','            if addition {','            if false && addition {')],'added_words_are_visible_unresolved'),
('hide-new-source',[('app','            self.source_view = self.sources.len() - 1;','            self.source_view = 0;')],'added_words_are_visible_unresolved'),
('leak-skipped-addition',[('app','            request.sources.retain(|s| s.id != *id);','            request.sources.retain(|_| true);')],'skipping_scope_does_not_include'),
('lose-undone-question',[('app','                self.guess = Some(board);','                self.restored_questions.clear();\n                self.guess = Some(board);')],'undone_skip_survives_a_later'),
('default-bar-on',[('app','            key_bar: false,','            key_bar: true,')],'optional_bar_uses_reserved_row'),
('discard-without-guard',[('app','                self.request_leave();','                self.quit = true;')],'exit_guard_keeps_unsent'),
('omit-header',[('app','    yohaku::render_header(frame, palette);','')],'source_wrap_positions_header'),
('consume-over-cap-input',[('app','        if bytes > 32768 {\n            self.notice = format!(','        if false && bytes > 32768 {\n            self.notice = format!(')],'encoded_cap_counts_metadata'),
('remove-back',[('goal','Paragraph::new("Back")','Paragraph::new("")')],'optional_bar_uses_reserved_row'),
('hide-rejected-attempt',[('checks','if d.decision == "reject"','if d.decision == "never"')],'rejected_schema_attempt_is_readable'),
('automatic-shaping-retry',[('app','            let text = self.complete(input, prompt, cancelled)?;','            let text = self.complete(input.clone(), prompt.clone(), cancelled)?;\n            let _ = self.complete(input, prompt, cancelled)?;')],'one_shape_then_one_log_only'),
]
proven=[]
for name,edits,test in cases:
    for key,source in original.items():paths[key].write_text(source)
    for key,old,new in edits:
        source=paths[key].read_text();assert source.count(old)==1,(name,'finder mismatch',source.count(old));assert old!=new
        paths[key].write_text(source.replace(old,new))
    result=run(test,name)
    assert result.returncode!=0 and 'FAILED' in result.stdout and 'could not compile' not in result.stderr,(name,'not proven')
    proven.append(name)
(root/'result.json').write_text(json.dumps(dict(baseline=True,proven=proven,scope='Fresh isolated copy, one independent code reversion per case, stub providers; not native-model or operator recognition'),indent=2)+'\n')
print('PASS',len(proven),'baseline-verified probes',root,flush=True)
