#!/usr/bin/env python3
import os, shutil, subprocess, tempfile
from pathlib import Path
repo=Path(__file__).resolve().parents[3]
root=Path(tempfile.mkdtemp(prefix='tinkery-intact-removals-'))
for name in ['Cargo.toml','Cargo.lock','src','vendor','fixtures']:
    source=repo/name
    if source.is_dir():shutil.copytree(source,root/name)
    else:shutil.copy2(source,root/name)
brain=root/'src/shaping/brain_dump.rs';view=root/'src/shaping/brain_dump/intact_view.rs'
original={p:p.read_text() for p in [brain,view]}
env={**os.environ,'CARGO_TARGET_DIR':str(repo/'target/intact-removals')}
def test(name):
    p=subprocess.run(['cargo','test','--locked','--lib',name],cwd=root,env=env,text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT)
    print(p.stdout,flush=True)
    return p
baseline=test('shaping::brain_dump')
assert baseline.returncode==0,'Baseline failed; no probes accepted'
def cut(s,begin,end):
    a=s.index(begin);b=s.index(end,a);return s[a:b]
span_guard=cut(original[brain],'        if !grapheme_boundary(&source.text, start)','        Ok(start..end)')
extract_guard=cut(original[view],'        if !grapheme_boundary(&text, start)','        text.get(start..end)')
label='    if !app.sources.is_empty() {\n        frame.render_widget(\n            Paragraph::new("Ctrl-O originals")'
probes=[
 ('automatic-card',brain,'            self.input = Note::new("");\n            self.input_scroll = 0;','            self.add_fragment(sid, &text, 0, text.len());\n            self.input = Note::new("");\n            self.input_scroll = 0;','silent_entry_intact_sources_and_deliberate_extractions_survive_answer_and_layout'),
 ('wrong-occurrence',brain,'.nth(self.occurrence)','.nth(0)','annotations_match_exact_occurrences_and_do_not_cut_unicode_graphemes'),
 ('unmatched-quote',brain,'.ok_or("Annotation is not an exact source substring at that occurrence")?','.unwrap_or(0)','bad_span_keeps_the_previous_reading_and_originals_without_fallback'),
 ('cut-annotation-grapheme',brain,span_guard,'','annotations_match_exact_occurrences_and_do_not_cut_unicode_graphemes'),
 ('cut-extraction-grapheme',view,extract_guard,'','deliberate_keyboard_extraction_is_exact_linked_and_has_no_provider_side_effect'),
 ('all-words-supported',view,'supports.iter().any(|r| r.contains(&index))','!supports.is_empty()','annotation_styles_are_on_the_exact_source_cells_not_on_reworded_cards'),
 ('no-unresolved-style',view,'style = style.add_modifier(Modifier::UNDERLINED);','style = style;','annotation_styles_are_on_the_exact_source_cells_not_on_reworded_cards'),
 ('extract-calls-provider',view,'        self.add_fragment(source, &text, start, end);','        self.add_fragment(source, &text, start, end);\n        self.applied=0; self.submit();','deliberate_keyboard_extraction_is_exact_linked_and_has_no_provider_side_effect'),
 ('title-replaced-by-cue',view,'.title(short_title(&source.text, area.width.saturating_sub(2)))','.title("… Ctrl-O")','clipping_is_visible_and_original_is_one_action_away'),
 ('originals-before-source',brain,label,label.replace('if !app.sources.is_empty()','if true'),'originals_label_exists_once_only_after_source_submit_and_unmarked_words_are_neutral'),
]
for label,path,old,new,name in probes:
    for p,s in original.items():p.write_text(s)
    assert original[path].count(old)==1,(label,'mismatched probe')
    path.write_text(original[path].replace(old,new))
    result=test(name)
    assert result.returncode!=0 and 'FAILED' in result.stdout and 'could not compile' not in result.stdout,(label,'protection not proven')
    print('PROVEN:',label,flush=True)
print('PASS: baseline plus ten independently removed protections; isolated copy:',root,flush=True)
