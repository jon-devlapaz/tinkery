# Think

## Shipped experience

One blank writing box: type/paste without fields, kinds or cards. Only F2 interprets;
Enter is newline. Originals stay intact on the left, one “I think you mean…” reading
and at most one focused question on the right. Competing interpretations get a
disambiguating question, not a second reading.

### Questions

Use the [goal-block contract](goal-block.md). Ask consequential missing intent,
not discoverable facts. Once required parts exist, ask the most consequential
operator-only choice that would change a goal line: target, scope, boundary or
protected things. Decisions contain part, text, at least two answers and optional
question link. Invalid parts/answer counts are ignored. Open decisions block
“Nothing I'm unsure about” and travel to review/seed. Review shows a linked
question once; otherwise an Open decision note. Alternatives survive handoff even
when a question replaces the note.

After shaping, separate review sees only words, answers, draft parts and shaping
decisions, never the template. It finds silently settled choices about scope,
irreversible actions, protected things, persistence or whose rules apply. The
reading decoder joins decisions; raw recorded replies stay unchanged. Only explicit
valid same-part shaping indices let review replace decisions. Uncovered choices
remain, even on the same part; no similarity/same-part deduplication. After cleanup,
ask the top shaping question first when required parts exist. Deduplicated Why/Done
when still needs its missing-part question before review choices. Failed/unreadable
review leaves shaping intact.

Stop when target and irreversible boundaries are settled: no already-covered edge
cases or implementation questions. Answers belong only to their part. The
[early-stop gap](#open-work-only) remains. Notes hold open intent, not evidence/status
disclaimers. Evidence authorship, test history and unreproduced failures stay in
originals, not invented requirements. Explicit requirements and undecided success
checks/permissions still belong in goals/questions.

Answers/added dumps remain separate exact sources. Sending refines even a ready
reading, never starts another dump. Skips persist; Ctrl-Z restores. Separate goals
stay aside, not in each other's constraints; deferred items remain seed context.
Constraint-polarity errors remain possible; decision coverage is a model
judgment, not proof that choices mean the same thing. See [#64 evidence](../../findings/README.md#goal-history-and-review-regression-check-64).
Originals are plain ink; exact cited spans validate invisibly, and uncited words
are not implicitly rejected.

### Model calls

At most two per live send: shaping then decision review. Scope questions may use
none. No calls after display or automatic retries. Practice/verification use fakes;
[historical evals](#decision-review-evidence-34) promise no general quality.

`src/think/brain_dump/prompts/` holds one rule per line and a finding/eval note per
tuned rule; notes never reach models. Tests pin sent Pi bytes and decoder field
names. Prompt changes require deliberate eval and operator budget.

### Readiness and review

With no focused question (none left, skipped or part asked twice), one muted line:

| State | Line |
| --- | --- |
| Ready | Nothing I'm unsure about. Review when you're ready. |
| Required part missing | Still missing: Done when. Review when you're ready. (name first missing part: who, outcome, why, done when) |
| Required parts present, other intent open | Some parts remain open for the seed. Review when you're ready. |

Without a composed goal omit the review invitation. F1 uses the same first sentence,
offers F3 only with a saveable goal; practice/no Seed Me explains real-mode/Seed Me
requirement, no goal says type then F2, unapplied/cancelled first reading says F2
retry. State depends only on goal content, including sent-but-unapplied words and
unanswered skips, never recording/unsent text. Review lists all open intent.

F3 deliberately reviews; typed `confirm`+Enter freezes the displayed goal and,
after successful save, shows [receipt](shape-and-handoff.md). Missing parts/questions
may travel to seed: readiness is not required to save, nor is saving seed or
implementation approval.

## Think record

Typed `confirm` also writes one pretty JSON `<UTC confirm time>_<Seed Me session id>.json`
in `~/.local/share/tinkery/think/`, overridden by `--record-root`. Without an explicit
root, unset/empty/relative HOME refuses `--shape-pi` startup, asking for one rather
than writing under cwd. Review/bar label `type confirm` → `save goal and record`
discloses the write. Unconfirmed exits/crashes leave no record.

Schema 3 retains version, origin (`live` for Pi, else `simulated`), model/thinking,
both prompts' hash, first-turn/confirm times, session folder/canonical path/source
ID, launch Git root `repo` (or null), exact confirmed goal, every ordered timed verbatim
turn (`words`, `asked`, `answered`, `skipped`, `restored`, `scope_asked`,
`scope_answered`) and existing serialized F5 checks. Rejected checks retain `raw`;
passing checks use decoded `candidate`, `raw: null`. Each Think visit gets separate
Pi diagnostic history: reused settings start no process/file read and cannot mix
goals. Standalone return/reentry preserves the record folder.

`repo` means made in, not about. Review shows `made in <folder>` before confirm.
It never enters the goal, Seed Me ledger or model requests: observations stay
separate from operator words.

Shared `src/think_record.rs` owns format; Conversation owns turns. Write only after
Seed Me save/readback succeeds. Record failure leaves goal saved and a receipt
notice; reason/path live in How it works. Think never reads records. This is not
Seed Me's `ledger.json`. Tend's deliberate [history](shape-and-handoff.md#think-history)
adds no model context. Eval record reading/gap flags are separate slices.

## Interaction contracts

### Reading

Overflow keeps muted `PgUp/PgDn scroll reading` below the reading; keys/wheel reveal
all before review and Every key names the keyboard route. Hide hint when fitting,
including resize. Scrolling changes no goal/original text.

### Help

Muted bar/help agree on F2/F3/F6/F8/F10, offering only permitted actions (F3 only
real mode with Seed Me). Empty help says “Type, then F2 sends it.” F1 opens bounded
reading-side help without changing originals/drafts: Right now, F1 Every key,
`h` How it works. Jade layer title, bold ink headings/keys, muted labels/border.
Short pages fit content; long pages scroll under fixed title/navigation. Scroll
hint only on overflow; empty technical sections add no gap before Last result.

Esc first cancels voice, else closes/backs from technical help; reading cancellation
remains after help closes. Help blocks typing/paste/hidden send/review/voice.
F10 retains loss guard, Ctrl-C cancellation, Ctrl-D Why, `b` changes preference
without revealing hidden bar. `?` is text while writing. Review bar says
`type confirm save goal and record` when recording.

### Editing and focus

- Tab/Shift-Tab toggles writing/original once present. Typing returns to writing
  and clears original selection. Letters never command; F8 skips, F9 copies.
- Shift+arrows/Home/End and drag select logical whole graphemes; Ctrl-A selects
  active surface only. Writing input/Enter/deletion/paste replaces selection as
  one edit. No numeric editor-size cap; allocation refusal preserves text/selection.
  Finite fixtures promise neither unlimited memory nor responsiveness.
- Ctrl-E extracts; F7/Ctrl-X toggles cards. Covered originals have no selection/copy.
- Ctrl-C copies a nonempty active selection, else cancels a request or does nothing,
  never exits. Dictation never deletes selection.

### Original bytes and display

Writing/sources/cards retain received UTF-8, including CRLF/CR/tabs/controls. Copy
uses base64 OSC 52, not executable source. Clipboard support/permission is the
host's. LF starts rows; CR → ␍, tab → ⇥, ASCII controls → control pictures, C1 → �;
CRLF is one grapheme. Display-only marks map cells to raw grapheme offsets for
motion, selection, highlighting and voice preview, never substring reconstruction.
Rows use machine-sized integers; only visible viewport narrows to terminal coordinates.

Literal ESC `[201~` ends bracketed paste and cannot occur inside its envelope.
Crossterm replaces invalid UTF-8 before Paste delivery; retain received strings,
not unavailable predecode bytes. Valid-UTF-8 complete-envelope tests include OSC
sentinels: bytes survive send/copy, never execute on display.

### Sending and leaving

- Encoded shaping request limit is 32 KiB including metadata/history and waiting
  reply's refreshed reading. One shared preview measures final payload, growing
  or shrinking; idle frames reuse it until input/extraction/voice/reply. Unapplicable
  replies make size unknown; F2 preserves old reading and sends one retry.
  Transactional preflight refusal starts no host and commits neither words nor
  waiting reply. A fitting replacement can be copied whole with Ctrl-A/Ctrl-C,
  deliberately replaced and sent once. If retained context alone exceeds limit,
  new words cannot fix it. F1 explains copying local words and originals via Tab,
  Ctrl-PgUp/PgDn, Ctrl-A/Ctrl-C. Copying never prunes/restarts.
- Distinguish provider input/context rejection, rate limit, timeout and overflow.
  Shaping failure preserves committed originals/prior reading. Failed/unreadable/
  oversized review is best-effort; keep valid shaping, explain skipped review in
  Why. Never automatically retry, chunk, summarize or send.
- Nothing to send/retry: F2 says “Write something, then F2 sends it.” on initial
  or refining screen, without request/save. Next input clears it; no permanent warning.
- One `refuse` route explains ineffective keys and recovery under writing or in
  review's confirm slot. At most two narrow-pane rows; it overrides Thinking/receipts.
  Next key/paste clears display but Last result retains it. Review names unsent
  words/waiting reply/in-progress reading; Enter before reading end says PgDn.
  Confirmation saves only from the end.
- Unfinished save says “Handoff not complete. F1 then h shows why; Enter retries.”;
  Last result retains reason. Review messages grow, leaving Back its own row.
- Idle Esc/F10 guard unsaved work. `--home` returns to menu; standalone exits before
  save, opens menu after save. Esc first cancels voice/reading, closes overlays,
  leaves review, clears original selection/focus or declines loss prompt.
- F4 originals/F5 why stay help-only. [Local voice](voice.md) inserts editable words,
  never sends. Answering keeps the original dump visible.

## Open work only

[Backlog](../research/shared/backlog.md#think) owns proposals, including
[“Like X” early stop](../research/shared/backlog.md#like-x-early-stop). Help-only
extracted cards are not the proposed scratch space.

## Decision-review evidence (#34)

See [historical evidence](../../findings/README.md#decision-review-evidence-34).

## Evidence-note regression check (#58)

See [historical evidence](../../findings/README.md#evidence-note-regression-check-58).

## Goal-history and review regression check (#64)

See [historical evidence](../../findings/README.md#goal-history-and-review-regression-check-64).
