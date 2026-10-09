# Answer-loop repair: exact dump, `both`, then a test-authored answer

## Inputs and provenance

`original-dump.txt` is byte-identical to the operator-approved Ctrl-O screenshot transcription from the intact-dump run: **745 bytes**, typos unchanged, no added newline, SHA-256 `2aab458a45d27a2ca828a3c69e5c6e7b093099b7143da86e247ef94fd90dc5ce`.

`operator-answer.txt` is exactly **`both`**, four bytes, no added newline.

**`test-second-answer.txt` is authored by the coding assistant for this validation, NOT supplied by the operator and NOT an accepted product requirement.** After the real next question asked about intervention before final judgment, the test reply was:

> Only if a decision changes the original intent or would be expensive to undo. Otherwise I judge the finished result; I do not want to direct the cooking or supervise each step.

The example waited for this explicit fixture after capturing the first answer; it did not invent it automatically or attribute it to the operator.

## Repair

- The application snapshots the exact focused question with its answer-source link at submit. Every request includes this complete settled history, not just ephemeral IDs or the last guess. `both` refers to the question it actually answered.
- Known answered IDs/equivalent text cannot regain focus. Pi mode also performs **one bounded extra continuity check** after an answer when questions are proposed: it withholds paraphrases of already answered issues, promotes remaining candidates, or shows no unanswered question. No invented replacement question or automatic retry. Bad verdicts/references fail explicitly and preserve the prior reading/originals. This is a model judgment, not proof of all possible paraphrases and never an authorization gate.
- Settled answers occupy a quiet line; full exact question/answer history is in details and Markdown export. “Settled” means answered, not goal/seed confirmation.
- A persistent first-line cue says **Reading updated from your answer: both**. Changed wording, changed evidence only, and genuinely unchanged readings are distinguished. The cue is bold against normal-weight jade reading text; no authored source glyphs/colour are changed.
- The restaurateur is interpreted as **final human judge**, not just “delicious” adjectives or routine cooking. This is carried in the actual first reading/outcome; the second reading carries coordination/compounding, then the test-authored intervention boundary. No claim that every generated reading independently restates every role detail.
- Source scroll chrome only appears when the source overflows. The annotation legend uses plain words and requires actually visible marked source cells. Empty queue counts/`Later:` disappear. The input stays **Your reply / F2 submit**; by default its submit answers the focused question. Ctrl-N explicitly toggles a separate dump with a notice, not a silent label flip.

## Actual model journey

`openai-codex/gpt-5.6-luna`, low reasoning, same scoped Seed Me guidance/disabled-resource Pi transport as the previous run. **Five provider calls:** three shaping replies (`00`, `01`, `03`), two continuity judgments (`02`, `04`). No retries or failures in this real journey. Raw files remain in `raw-provider/`.

1. First focus: **Should “compounds itself” primarily mean the harness improves through use, the codebases become easier to change, or both?**
2. Exact operator answer **both**. Next focus: **At what points do you want to intervene before judging the finished result?** The first reading explicitly says “the final human judge”; the outcome says “judging the finished result.” Two readings, zero misfits, no automatic extractions. `02-answer-state.json` records the exact answered question/source and update cue.
3. The labelled **test-authored** second answer above. Next focus: **What must you be able to see or trust before a change feels ready for human review?** Both previous questions remain in settled history. The second reading includes intervention only on consequential intent/costly-to-undo decisions; this reflects test data, not operator confirmation.

Both continuity verdicts returned `repeated:[]`: the new shaping history already advanced correctly. The supplied operator paraphrase is additionally exercised by the mocked-Pi regression/verdict-enforcement test; the real run is not evidence that the checker caught a naturally occurring repeat.

`live/` preserves exact paper JSON, readable Markdown, application state/notice/update/history, application-resolved quote byte anchors, and frames at **80×24, 100×30, 160×40, 320×40**. Answer updates, settled lines, and different next questions are visible at every size. Exact source/answer equality, source-linked history, quote byte ranges, and absence of empty queue chrome were checked. The long dump stays one intact card and may scroll.

## Regression/counterfactual evidence

`final-validation.log`: formatting, all app targets (**57 library tests**), upstream Pinstar (**40 tests**), build, both PTY suites through 320×40, strict app/upstream Clippy and whitespace checks passed. Five new component tests cover two-answer settlement/update visibility, renamed answered topics/no-question state, stable input/empty chrome, honest unchanged readings, and bounded Pi paraphrase-verdict enforcement plus malformed/foreign/duplicate verdict rejection. No historical tests removed.

`removal-proofs.log` / `removal-proof.py`: passing baseline, then **seven isolated removals** caught by their tests: losing question snapshots, permitting equivalent text under new IDs, ignoring the semantic verdict, hiding the update cue, removing its visual contrast, showing an empty queue, and showing source scroll chrome when content fits.

`earlier-removal-proof.log` is retained: four protections were caught, but removing cue boldness did NOT fail because the old palette made all reading text bold. That exposed an inadequate contrast assertion. Reading text was made normal-weight, the cue stayed bold, and the test now checks both sides of the contrast. The final counterfactual run catches that removal. This was not a weakened oracle or a fabricated passing proof.

## Limits

The operator liked the previous whole-card direction; this latest loop still needs their recognition assessment. Model question/role judgments and structural checks are not native-desktop/font/clipboard proof or a guarantee that the board beats chat. The second answer's preferences are test-only. No canonical confirmation, execution, handoff, persistence, voice, multiple-choice answer UI, or merge was added. After an answered submit, Pi may make two requests; each remains bounded/cancellable and can incur charges. Existing real mode is explicit, default mode remains simulated/unsaved.
