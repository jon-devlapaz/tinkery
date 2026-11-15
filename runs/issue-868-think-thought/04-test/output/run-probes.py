from pathlib import Path
import subprocess, sys, os
ROOT=Path('/Users/jondev/dev/worktrees/tinkery/codex/probes-868')
OUT=Path('/Users/jondev/scratch/tinkery-868-evidence')
p=ROOT/'src/think/brain_dump.rs'
original=p.read_text()
anchor='''            Ok(crate::thought::Saved::New | crate::thought::Saved::Duplicate) => {
                self.exit = Exit::Garden
'''
assert original.count(anchor)==1
mutations={
'provider':'''                if let Ok(preview) = self.preview_send(&self.input.text) {
                    let _ = self.host.reshape(preview.request, &AtomicBool::new(false));
                }
''',
'goal':'''                if let Some(config) = &self.handoff.config {
                    config.confirm(seed_me::Affirmed {
                        goal: self.input.text.clone(),
                        outcome: "TEST unintended goal".into(),
                        options: vec![seed_me::Choice { label: "A".into(), benefit: "b".into(), cost: "c".into(), undo_cost: "u".into() }],
                        originals: vec![seed_me::Original { id: 0, in_reply_to: None, text: self.input.text.clone() }],
                        context: "{}".into(),
                        source: "TEST unintended capture confirmation".into(),
                    }).expect("TEST injected goal creation");
                }
'''}
env=dict(os.environ)
env.pop('CARGO_TARGET_DIR',None)
def build(label):
    r=subprocess.run(['cargo','+stable','build','--locked'],cwd=ROOT,env=env,capture_output=True,text=True)
    (OUT/f'{label}-build.log').write_text(r.stdout+r.stderr)
    if r.returncode: raise RuntimeError(r.stderr)
def test(label):
    r=subprocess.run([sys.executable,'-c',"import sys;sys.path.insert(0,'tests');import tend_thought_terminal as t;t.capture_guards((80,24))"],cwd=ROOT,env=env,capture_output=True,text=True)
    (OUT/f'{label}-test.log').write_text(r.stdout+r.stderr)
    return r
try:
    build('baseline')
    r=test('baseline');assert r.returncode==0,r.stderr
    print('Baseline green',flush=True)
    for name,mutation in mutations.items():
        p.write_text(original.replace(anchor,anchor.replace('                self.exit = Exit::Garden\n',mutation+'                self.exit = Exit::Garden\n')))
        (OUT/f'{name}-mutation.txt').write_text(mutation)
        build(name)
        r=test(name)
        assert r.returncode!=0,f'{name} mutation escaped'
        print(name+' failed as required: '+r.stderr.splitlines()[-1],flush=True)
        p.write_text(original)
finally:
    p.write_text(original)
    build('restored')
    r=test('restored')
    assert r.returncode==0,r.stderr
    print('Restored green; source matches original',flush=True)
