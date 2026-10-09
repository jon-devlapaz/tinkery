# Dogfood 3 repairs / PR #2

Scope: findings 11–18 and the relayed follow-up about question priority/details. First principle: **yohaku no bi**—the default shows the centre reading and focused question, not a whole working draft. Voice and answer choices remain later-only; no audio/transcription, persistence, confirmation, execution, or host-setting changes.

## Same input, not a reconstruction

`original-dump.txt` and `original-answer.txt` are the exact operator-relayed strings (no surrounding quotes or added newline). The dump cuts into f1–f8; the answer adds f9/f10. The existing source/UTF-8 offsets and original-view protections remain.

The Claude Code session separately reported live verification of the earlier uncommitted UI: sharper frame, populated misfits, sparse panel, border highlights, clipping cues and no card-ID titles. This is **relayed evidence**, not our native-desktop verification. That session also found an over-correction: the standupy glossary question displaced the main fork.

## Actual-model reruns and retained failure

Both attempts used the unchanged exact dump/answer, actual `openai-codex/gpt-5.6-luna`, low reasoning, and the existing Seed Me shaping guidance. Four explicit provider calls total; no automatic retries/repairs. Raw provider replies, dimensioned frames, exact Markdown strings and readable Markdown are retained in each directory.

1. **`priority-rerun`**: first question restored the consequential fork; standupy's fragment was marked unresolved. The answer response omitted f4, the earlier priorities evidence. Strict coverage validation rejected it; originals and previous accepted reading were retained. `02-answer-guess.json` is therefore the **retained previous guess**, not the rejected provider response. The raw second response is in `raw-provider/01-stdout.json`. This failure is not hidden or counted as successful reshaping.
2. **`coverage-rerun`**: after generic full-ID coverage and root/symptom guidance (not sample-specific instructions), both submissions were accepted. The first question asks whether ineffective coordination meetings or changing priorities are the deeper concern. Standupy stays on a **doesn't fit yet** card, not in the question in focus. After the exact answer:
   - One frame: **“The team needs a shared, trusted understanding of what matters this week, despite changing priorities.”**
   - Direct support: f4, f9, f10. All other fragments, including the standup symptom f1, standupy f5 and dashboard f8, remain misfits rather than being merged into the centre.
   - Follow-up: **“What visibility would help people understand current priorities without feeling like surveillance?”**
   - Real alternatives remain in details: an explicit shared weekly-priority reference versus a check-in focused on priority changes.

### Judgment, not just counts

This rerun sharpens around the operator's selected concern rather than broadening across all ten fragments. The main fork precedes prior-tool vocabulary; the dashboard/trust follow-up now addresses a consequential remaining tension. The flagged standupy card preserves the unknown without assuming its capabilities. There is one initial misfit and seven after the answer; these are classifications, not a target count.

A fixed-source structural oracle checked distinct initial framings, the non-glossary first question, standupy in misfits, the narrower final support set, symptom demotion, disjoint/full coverage, dashboard question and at least two candidates. It does **not** prove question usefulness generally, capability feasibility or operator recognition of these exact new responses. No further sampling to manufacture a pass.

## UI/interaction changes

- Default centre-only panel; desired experience, misfits and candidates are disclosed in **details**, not deleted. One panel-level provisional label; redundant model `PROVISIONAL:` line prefixes are removed only in agent presentation/export, never from user originals/fragments or saved raw responses.
- Supporting fragments have double/highlighted borders, preserving carbon source text. **[ / ]** switches which reading's evidence is highlighted; it neither records an answer nor sends a request/confirmation. No visible supports-ID list or “Your f1” card titles.
- Misfit cards are marked **doesn't fit yet**. **… Ctrl-O** marks clipped text, including viewport clipping; Ctrl-O exposes intact originals.
- **Ctrl-D** and the visible header control open details from input, board, originals or help; board `d` also toggles. Plain `d` in the input is authored text. Opening hidden details from a modal brings them forward rather than toggling an invisible state off. Typing, focus, positions and provider-call counts are preserved.
- Empty start stays empty: the details control appears only after a submitted source. No-question fallback gently asks whether the reading fits; it does not claim confirmation.

## Validation and reproduction

`validation.log` covers formatting, all-target/upstream tests, strict app/upstream Clippy, both running-binary PTY suites through 320×40, and whitespace checks. `baseline-removals.log` retains the prior nine protection probes; `new-removals.log` records six more baseline-verified removals for misfit/support separation, short centre, highlights, clipping cues, global details and question-priority guidance. Component tests exercise all focus/modal states and literal `d`; PTY tests exercise input/board/original details access without extra requests.

The first new-removal harness invocation failed to match a rustfmt-wrapped guard; literal matching was corrected, not the guard/tests. Its trace is retained as `earlier-removal-harness.log`.

The initial PTY failure was a stale oracle: “I think this is about” became a panel title visible behind the original modal. The test now waits for actual centre content and input-focus footer, not that always-visible heading. Failure log preserved as `earlier-validation.json` (escaped to preserve padded terminal cells); it was not fixed by weakening timeout or dropping a journey.

```sh
TINKERY_LIVE_MODEL=openai-codex/gpt-5.6-luna \
TINKERY_SEED_ME=/path/to/tink-skills/skills/seed-me/SKILL.md \
TINKERY_LIVE_DUMP_FILE=docs/evidence/2026-10-08-dogfood-3/original-dump.txt \
TINKERY_LIVE_ANSWER_FILE=docs/evidence/2026-10-08-dogfood-3/original-answer.txt \
cargo run --locked --example live_brain_dump_check -- /tmp/new-standup-evidence
```

Destination must not exist; real calls can incur charges. `*-paper.json` preserves the full Markdown string; `.md` trims trailing presentation whitespace only. Live frames precede the tiny empty-start-only hiding of the details control; submitted-board rendering is unchanged. Font rendering, original missing-ASCII fault, actual clipboard permissions and native Claude desktop remain manual boundaries. The bounded Pi process is not an OS sandbox.

Next: operator assesses the refined loop. Voice (local transcription into an editable box before explicit submit, no retained audio) and optional answer choices are only later follow-ups; nothing from those slices is built here.
