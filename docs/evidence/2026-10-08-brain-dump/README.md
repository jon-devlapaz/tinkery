# Brain-dump front door / first slice

Default launch is now one **What's on your mind?** box. No interpretation/provider request before F2. Submitted editor text stays intact; fragments are application-owned exact excerpts with UTF-8 byte offsets. Responses update a provisional reading and focused question, never existing fragment positions. Answers visibly update the reading; a response arriving during new typing waits for submit. Originals are Ctrl-O away. No saving, confirmation, canonical seed, ledger, execution, or handoff.

This is a **candidate for operator dogfood**, not proof that the board feels better than chat. The provisional reading is a fixed panel, not yet a draggable/editable centre card. Clusters, relationships, settled strip, strike/centre edits and confirmation are deferred. Multiple-choice answers are also later, not implemented here.

## Actual model evidence, including failures

All calls used `openai-codex/gpt-5.6-luna`, low reasoning, actual Pi and the existing Seed Me skill. Inputs are fresh **assistant-authored test dumps**, not operator recognition. Nine explicit provider requests total across the recorded iterations (can incur charges); no automatic retry or repair.

- `live-initial`: one rejected malformed JSON response (extra quote in `undo_cost`). Original/previous-board recovery shown; raw output retained.
- `live-after-syntax-check`: three completed calls, but the first question imported a nonexistent **Ledger:** link convention from the skill. This is a semantic failure, not a pass. Raw replies retained.
- `live-scoped-guidance`: three calls after limiting borrowed guidance to the shaping section and rejecting appended status prose. Fragments/originals/positions survive dump, answer and additional dump. Before/after frames show the answer updating the centre reading. No invented Ledger link.
- `dark-mode-regression`: two calls using a newly authored regression for dogfood findings 7–8. “Dark mode toggle” remains an exact fragment/candidate, not the outcome. Blog and Hamster remain viewing locations. Manual choice versus following system appearance are materially different candidates, with provisional costs. Hamster is kept and questioned, not guessed as a known product. The answer changes the centre reading to include the RSS-reader clarification.

### Honest question/meaning judgments

- Scoped work dump: receipt-wide versus per-check granularity is a real distinction, but it is mechanism-first; **not established as the most consequential question**. The answer is absorbed. The later scratch-space question tests a plausible boundary, but “private” was inferred from “quiet,” and the final outcome dropped the explicit PR-thread viewing location. Original text and candidates retain it; that does **not** make the model outcome correct.
- Dark-mode dump: first question usefully exposes unknown Hamster meaning/location, albeit compound. Two uncertain readings are shown, but the second is mechanism-centric and not a strongly distinct interpretation. After the RSS clarification, the next question re-asks already stated desired coverage instead of probing actual theme/control capability. **This is a quality miss**, not operator recognition.
- Coverage checks prove source references exist, not that every interpretation is faithful. Two framing/alternative counts do not prove semantic diversity or feasibility. No model output was treated as acceptance or implementation authority.

The scoped run also exposed a clipped long focus question at 80×24. The question pane was widened afterward; a regression uses that exact question and checks its final `PR?` at every supported/wide size. Recorded live frames are preserved **prior UI states**, not screenshots of the final renderer.

No further sampling to conceal these misses. Operator recognition and question usefulness are the next acceptance boundary.

## Verification

`validation.log` records formatting, all-target tests, upstream Pinstar tests, both running-binary POSIX PTY suites, strict application/upstream Clippy and whitespace checks. `protection-removals.log` records nine isolated, baseline-verified removals rejected by tests: exact fragments, coverage, two uncertain framings, two candidates, appended status prose, typing interruption, disabled sessions, requested-mechanism separation and alternative instructions.

The new PTY journey exercises explicit submission/answer, originals, OSC52 export, malformed-provider failure, cancellation, clean exit and terminal restoration at 80×24, 100×30, 160×40 and 320×40. Its provider is a **stub**, not real-model evidence. Component tests additionally exercise actual mouse drag, exact source text and retained positions. Existing seven legacy journey families remain under explicit `--scratchpad`.

Plain PTY burst/paced typing metrics are in the log, including legacy note/feedback entry. They are sub-second, not a measured fix for Herdr's reported multi-second send-text pacing. Native Claude desktop input/rendering, original missing-ASCII fault, actual font rendering, clipboard permissions and human recognition remain unproven. No fonts or host settings changed.

## Reproduce

```sh
cargo build --locked
python3 -B tests/brain_dump_terminal.py
python3 -B tests/terminal_smoke.py
TINKERY_LIVE_MODEL=openai-codex/gpt-5.6-luna \
TINKERY_SEED_ME="$HOME/dev/active/factory/seed-me/skills/seed-me/SKILL.md" \
cargo run --locked --example live_brain_dump_check -- /tmp/new-brain-evidence
# Add TINKERY_LIVE_DARK_MODE=1 for the two-request dark-mode regression.
```

Real runs can incur charges; the destination must not exist. Optional `TINKERY_LIVE_PI_COMMAND` names a capture wrapper; raw provider replies here came from the explicit wrapper used during validation. The bounded Pi adapter disables sessions/tools/extensions/MCP/context resources; this is **not an OS sandbox**.

Frames are escaped, dimensioned JSON. `*-paper.json` preserves the exact Markdown string, including trailing spaces; `.md` is a readable copy with trailing whitespace trimmed only. Raw provider replies and prior failed states remain intact.

## Later, not in this PR

Optional focused-question choices: 2–4 agent-coloured guesses plus “something else” free text; never preselected, omitted for open-ended questions. Attribute a chosen answer to the operator as `chose: <option>`. Always permit free text and judge fixation explicitly (Jansson & Smith 1991). Inspiration: operator-described cobrew `ask`, not verified implementation parity. Keep answers visibly updating the centre card. Recorded in the design brief's later list and follow-up #57.
