#!/usr/bin/env python3
import os,shutil,subprocess,tempfile
from pathlib import Path
repo=Path(__file__).resolve().parents[3]
root=Path(tempfile.mkdtemp(prefix='tinkery-answer-removals-'))
for name in ['Cargo.toml','Cargo.lock','src','vendor','fixtures']:
 s=repo/name
 if s.is_dir():shutil.copytree(s,root/name)
 else:shutil.copy2(s,root/name)
brain=root/'src/shaping/brain_dump.rs';view=root/'src/shaping/brain_dump/intact_view.rs';guard=root/'src/shaping/brain_dump/question_continuity.rs'
original={p:p.read_text() for p in [brain,view,guard]}
env={**os.environ,'CARGO_TARGET_DIR':str(repo/'target/intact-removals')}
def test(name):
 p=subprocess.run(['cargo','test','--locked','--lib',name],cwd=root,env=env,text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT)
 print(p.stdout,flush=True);return p
assert test('shaping::brain_dump').returncode==0,'Baseline failed'
def block(s,a,b):
 start=s.index(a);end=s.index(b,start);return s[start:end]
record=block(original[brain],'                self.settled.push(Settled {','            }\n            self.sources.push')
update=block(original[brain],'                self.update = self.guess.as_ref().map','                self.guess = Some(g);')
bold=block(original[brain],'        if app.update.is_some() && !app.details {','        let paragraph = Paragraph::new(content)')
filtering=block(original[guard],'    guess\n        .questions','    Ok(())')
probes=[
 ('lose-question-snapshot',brain,record,'','both_is_an_answer_with_a_visible_update_and_full_settled_question'),
 ('permit-renamed-exact-question',brain,'a.id == b.id || words(&a.text) == words(&b.text)','a.id == b.id','answered_ids_and_equivalent_text_with_new_ids_never_regain_focus'),
 ('ignore-semantic-verdict',guard,filtering,'','paraphrased_answered_questions_are_withheld_by_bounded_pi_continuity_check'),
 ('hide-answer-update',brain,update,'                self.update=None;\n','both_is_an_answer_with_a_visible_update_and_full_settled_question'),
 ('remove-glance-emphasis',brain,bold,'','both_is_an_answer_with_a_visible_update_and_full_settled_question'),
 ('show-empty-queue',brain,'} else if queue.is_empty() {','} else if false {','reply_label_is_stable_and_empty_chrome_is_hidden'),
 ('show-scroll-when-source-fits',view,'let block = if max > 0 {','let block = if true {','reply_label_is_stable_and_empty_chrome_is_hidden'),
]
for name,path,old,new,testname in probes:
 for p,s in original.items():p.write_text(s)
 assert original[path].count(old)==1,(name,'mismatched finder')
 path.write_text(original[path].replace(old,new));result=test(testname)
 assert result.returncode!=0 and 'FAILED' in result.stdout and 'could not compile' not in result.stdout,(name,'not proven')
 print('PROVEN:',name,flush=True)
print('PASS: baseline and seven independent removal probes; isolated copy:',root,flush=True)
