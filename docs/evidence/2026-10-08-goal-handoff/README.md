# Goal confirmation only — real helper, isolated TEST sessions

## Boundary and implementation

This slice implements Seed Me **goal confirmation**, not seed confirmation. `SKILL.md` Session lifecycle, `references/seed-contract.md`, the ledger publication schema, and the actual `scripts/session.py`/`viewer.py` were read before implementation. The helper is found beside the supplied `--seed-me SKILL.md`; no Seed Me implementation is copied or vendored.

Ctrl-G / the visible **review goal** control opens the complete selected reading with remaining unanswered **and skipped** questions. Questions do not block goal confirmation. The prompt says **“Running out of questions doesn't mean I understood you.”** Opening, dragging, selection, answering, Enter/F2 without the affirmation, and an empty question queue do not confirm. Type `confirm`, then Enter after the complete review is available; long reviews require scrolling to the end. Pending reshapes/deferred replies/unsubmitted text block stale confirmation. Paste never submits. Escape before affirmation writes nothing.

On affirmation, a bounded background operation calls the official helper's `init`, `read`, `publish`, `read` and `status`. The active session has one settled decision as origin, `authority: user`, and exact displayed reading as both `goal` and origin `answer`. Original dumps/answers are verbatim `observed` receipt strings; answered-question snapshots and unresolved questions are context, not new settled Seed Me decisions. Outcome/options stay draft proposals. Publication JSON is a temporary input outside the session; the helper owns canonical ledger/view writes. It uses the observed version, retains an initialized recovery path, reconciles a publish error through `read`, and refuses to replace an existing different origin. No automatic retry, deletion, seed file, `seed confirm`, `end completed`, `prepare`, or implementation approval.

The official loopback viewer starts quietly. Its URL appears on one Ledger line. **No browser is opened.** Saved-view fallback reports failure explicitly. Only the owned child is stopped on ordinary exit; the session remains active. The session path/next step are printed again after terminal restoration. A confirmed handoff is read-only in Tinkery:

> Goal confirmed. The seed isn't written yet.
>
> Continue this session with Seed Me in Claude Code, Codex, or Pi.

`--seed-session-root` passes the helper's supported `init --root` option. Every positive test used a private **temporary TEST root**, never the operator's `~/.local/share/seed-me/sessions/`. Production without this option uses Seed Me's own default after explicit affirmation.

## Meaning and convergence

- Meaning-bearing authored concrete nouns/scope names remain in the reading and desired experience rather than generic “software result” substitutions. A deterministic check protects literal acronyms in quoted supporting evidence, including the selected confirmation reading. It does not force every old word into a refined goal: uncited peripheral/rejected text remains intact and neutral.
- A fresh bounded meaning audit checks named actors/objects, compounding referents and false choices. Its source quotes are verified; malformed output fails explicitly. This is a **fallible model judgment**, not understanding, confirmation or authorization. It may reject usable readings. No hidden repair/retry follows.
- After an answered question, validation requires **one combined interpretation**; genuine unresolved questions remain separate. Alternative candidates may be `[]`; the old mandatory two-route template was removed so board/harness or checkpoints/autonomy need not become false choices.
- Initial readings retain the 45-word bound. Combined readings have a **65-word bound**: the live checkpoint case showed that carrying two prior meanings, concrete nouns and a new boundary into one 45-word panel forced compression. Tests still reject 46-word initial and 66-word combined readings; no historical long-reading test was removed. This is a bounded consolidation, not permission for unbounded summaries.

Each real submit now makes a shaping call plus a bounded meaning-audit call, and may make the existing question-continuity call after an answer. They can incur charges. Default reasoning remains **low**; the successful evidence below explicitly selected **medium**. No automatic model/reasoning change.

## Passing required live journey

`required/`, `raw-required/`, `accepted-live.log`, `accepted-helper-read.json`, `accepted-helper-status.json`:

- Actual `openai-codex/gpt-5.6-luna`, **medium**, real Pi transport/disabled resources, official Seed Me skill at revision `86066b175b53e13b4180ce4cc9929a626b57a2f6`.
- Exact operator-approved 745-byte dump from the earlier screenshot transcription; SHA-256 `2aab458a45d27a2ca828a3c69e5c6e7b093099b7143da86e247ef94fd90dc5ce`. Typos unchanged, no added newline.
- Exact four-byte operator answer **`both`**. The first real question asked whether compounding meant the meta harness, the codebase, or both.
- Five provider replies: first shape/audit, answered shape/audit/continuity. No failures or automatic retries **within this passing journey**.
- One combined reading, zero candidate alternatives. **AUTOMATED TEST affirmation, NOT the operator confirming this newly generated reading.** The earlier live operator recognition is not transferred to this test.

Confirmed test reading, copied exactly into the helper origin:

> You want a human-first board and meta harness that compounds both itself and the codebase: coordinating the entire agentic software development lifecycle with rigor, durability, cohesion, and verification, so you can taste the final product as restaurateur and see a PR ready for human review that makes the next change easier.

The unresolved question remained visible beside confirmation and is retained as context:

> Besides tasting the final product, where should you intervene or approve work before the PR is ready for human review?

Isolated test session:
`/private/tmp/tinkery-TEST-goal-required-gQy8NP/141379a3-14ea-4172-be4c-638ca0c22969`

The helper's own `read` and `status` accept it: active human-schema session, settled user origin, exact displayed reading, current saved snapshot; viewer was started and then stopped by its owner. `seed-contract.md` does **not** exist. Receipt visible at 80×24, 100×30, 160×40 and 320×40. Receipt originals equal the source and answer byte-for-byte. This temporary test session is not durable operator work; archived JSON preserves the evidence if the OS later removes it.

## Preserved failures and scope of the proof

Nothing was erased or silently repaired:

- `live/` / `raw-provider/`: the first low-reasoning run mechanically reached a TEST goal session but substituted “result” for PR. It failed semantic acceptance. Its helper `read`/`status` were archived, then it was officially ended **stopped** with an explicit TEST failure reason (`first-test-stopped.json`). It is not the passing confirmation.
- `contract-rerun/`, `accepted/`, `verified/`, `scoped/`, `compact/` and matching raw directories/logs: independent, explicit development reruns rejected false routes, missing scope/nouns, or over-broad audit judgments. The audit was scoped away from exhaustive classification/qualitative adjectives; original protections remain tested. Names in these directory labels are historical attempt labels, **not assertions of acceptance**.
- `medium/` and `combined/`: adding `relayed-checkpoint-context.txt` after `both` was rejected respectively for the old 45-word limit and a meaning-audit concern about codebase health. The relayed file is quoted context from the request, **not recovered bytes of the operator's original answer**. These extended cases are retained failures, not successful checkpoint-preservation claims. The passing required journey did not submit that extra fixture. A component fixture checks that PR/restaurateur/both meanings/UI/design checkpoints survive freezing and stay source-linked, but it is **not** a live-model recognition result.
- Earlier PTY logs preserve stale modal/frame matching and blocked shutdown when the harness stopped draining terminal output. The corrected harness waits for modal removal and drains until process exit; assertions were not relaxed.
- Earlier removal log preserves a whitespace-sensitive mutation finder failure; corrected independent removals were rerun after a passing baseline.

Low-reasoning reliability and broad model-audit false positives remain limitations. The source/previous board survive every rejection. Structural checks and a model's audit do not establish operator recognition, native typography/clipboard proof, or that the board beats chat.

## Checks

`final-validation.log`: **64 app library tests**, all targets, **40 upstream tests**, formatting, strict workspace/all-target Clippy, build and every terminal suite passed. Official-helper PTY confirmation journeys cover 80×24 through 320×40, prove zero sessions before affirmation, one session despite repeated controls, literal whitespace/Unicode originals, unresolved-question context, quiet live viewer, owned shutdown, active continuation, no seed file, and official refusal of `end --status completed` without a seed. Seven new component tests; no historical tests removed.

`removal-proof.py`, `removal-proofs.log`: passing isolated baseline then seven independently caught removals: implicit Enter, omitted unresolved questions, affirming an unviewed long review, accepting ambiguous local text, ignoring false-choice/noun audit failures, dropping quoted literal acronyms, and retaining two readings after an answer. The latest full validation reruns this script.

CI checks out the public official Seed Me repository at the pinned revision as a **test dependency**, not vendored app code. Real helper/PTY checks run on Ubuntu and macOS; no provider calls or operator session locations in CI.

## Next slice — not built here

Take one session through a **draft seed** and the **intent derived by tink-sdlc's intent template**, with stage 1 importing via `--seed-contract`. Compare side by side:

1. Is **PR ready for human review** still explicit?
2. Are both compounding meanings retained?
3. Are the operator's UI and consequential-design checkpoints retained?
4. Has any suggestion silently become a decision?
5. Is selected work clearly bounded within the larger vision?

That is the next acceptance test for meaning surviving handoff, subject to new operator authorization. This PR does not draft/confirm that seed, invoke intake/prepare, or change the workbench.
