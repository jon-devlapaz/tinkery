# Think

## Shipped experience

The blank writing box accepts typing or paste without fields, kinds or cards.
Only F2 interprets; Enter is a newline. Originals stay intact on the left.
One “I think you mean…” reading and at most one focused question sit on the right.
Competing interpretations get a disambiguating question, not a second reading.

### Questions

Use the [goal-block contract](goal-block.md). Ask about consequential missing intent,
not discoverable facts. Once required parts exist, ask the most consequential
operator-only choice that would change a goal line: target, scope, boundary or
protected things. Decisions contain a part, text, at least two answers and an optional
question link. Ignore invalid parts or answer counts. Open decisions block
“Nothing I'm unsure about” and travel to review and the seed. Review shows a linked
question once; otherwise it shows an Open decision note. Alternatives survive
handoff even when a question replaces the note.

After shaping, a separate review sees only words, answers, draft parts and shaping
decisions, never the template. It finds silently settled choices about scope,
irreversible actions, protected things, persistence or whose rules apply. The
reading decoder joins decisions; raw recorded replies stay unchanged. Only explicit,
valid shaping indices for the same part let review replace decisions. Uncovered
choices remain, even on the same part. Neither text similarity nor a shared part
justifies deduplication. After cleanup, ask the top shaping question first when
required parts exist. A deduplicated Why or Done when still needs its missing-part
question before review choices. Failed or unreadable review leaves shaping intact.

Stop when the target and irreversible boundaries are settled. Ask no already-covered
edge cases or implementation questions. Answers belong only to their part.
The [early-stop gap](#open-work-only) remains. Notes hold open intent, not evidence
or status disclaimers. Evidence authorship, test history and unreproduced failures
stay in originals, not invented requirements. Explicit requirements and undecided
success checks or permissions still belong in goals or questions.

Answers and added dumps remain separate exact sources. Sending refines even a ready
reading, never starts another dump. Skips persist; Ctrl-Z restores them. Separate
goals stay aside, not in each other's constraints; deferred items remain seed context.
Constraint-polarity errors remain possible; decision coverage is a model
judgment, not proof that choices mean the same thing. See [#64 evidence](../../findings/README.md#goal-history-and-review-regression-check-64).
Originals are plain ink. Exact cited spans validate invisibly; uncited words are
not implicitly rejected.

### Model calls

Each live send makes at most two calls: shaping, then decision review. Scope
questions may use none. No calls follow display, and nothing retries automatically.
Practice and verification use fakes; [historical evals](#decision-review-evidence-34)
promise no general quality.

`src/think/brain_dump/prompts/` holds one rule per line and a finding or eval note
per tuned rule. Notes never reach models. Tests pin sent Pi bytes and decoder
field names. Prompt changes require deliberate evaluation and an operator budget.

### Readiness and review

With no focused question (none left, skipped or a part asked twice), show one muted line:

| State | Line |
| --- | --- |
| Ready | Nothing I'm unsure about. Review when you're ready. |
| Required part missing | Still missing: Done when. Review when you're ready. (Name the first missing part: who, outcome, why, done when.) |
| Required parts present, other intent open | Some parts remain open for the seed. Review when you're ready. |

Without a composed goal, omit the review invitation. F1 uses the same first sentence
and offers F3 only with a saveable goal. In practice or without Seed Me, help says
saving needs real mode and Seed Me. With no goal, say type then F2. If the first
reading was unapplied or cancelled, offer an F2 retry. State depends only on goal
content, including sent-but-unapplied words and unanswered skips, never recording
or unsent text. Review lists all open intent.

F3 deliberately reviews. Typed `confirm` plus Enter freezes the displayed goal and,
after successful saving, shows the [receipt](shape-and-handoff.md). Missing parts
and questions may travel to the seed. Readiness is not required to save; saving
is neither seed nor implementation approval.

## Think record

Typed `confirm` also writes one pretty JSON `<UTC confirm time>_<Seed Me session id>.json`
in `~/.local/share/tinkery/think/`, overridden by `--record-root`. Without an explicit
root, unset, empty or relative HOME refuses `--shape-pi` startup and asks for a root,
rather than writing under the current directory. The review and bar label
`type confirm` → `save goal and record` discloses the write. Unconfirmed exits,
including crashes, leave no record.

Schema 3 retains the version, origin (`live` for Pi, otherwise `simulated`), model,
thinking level, both prompts' hash, first-turn and confirm times, session folder,
canonical path, source ID, launch Git root `repo` (or null), exact confirmed goal,
every ordered, timed, verbatim turn (`words`, `asked`, `answered`, `skipped`,
`restored`, `scope_asked`, `scope_answered`) and serialized F5 checks.
Rejected checks retain `raw`; passing checks use decoded `candidate` and `raw: null`.
Each Think visit gets separate Pi diagnostic history. Reused settings start no
process or file read and cannot mix goals. Standalone return and reentry preserve
the record folder.

`repo` means made in, not about. Review shows `made in <folder>` before confirmation.
It never enters the goal, Seed Me ledger or model requests. Observations stay
separate from operator words.

Shared `src/think_record.rs` owns the format; Conversation owns turns. Write only
after Seed Me saving and readback succeed. Record failure leaves the goal saved
and adds a receipt notice; the reason and path live in How it works. Think never
reads records. This is not Seed Me's `ledger.json`. Tend's deliberate
[history](shape-and-handoff.md#think-history) adds no model context.
Eval record reading and gap flags are separate slices.

## Interaction contracts

### Reading

Overflow keeps muted `PgUp/PgDn scroll reading` below the reading. Keys and the wheel
reveal all before review; Every key names the keyboard route. Hide the hint when
content fits, including after resize. Scrolling changes no goal or original text.

### Help

The muted bar and help agree on F2, F3, F6, F8 and F10. Offer only permitted actions;
F3 needs real mode with Seed Me. Empty help says “Type, then F2 sends it.”
F1 opens bounded reading-side help without changing originals or drafts.
Right now leads with the state; F1 opens Every key, and `h` opens How it works.
The layer title is jade, headings and keys bold ink, and labels and borders muted.
Short pages fit; long pages scroll under a fixed title and navigation. Show the
scroll hint only on overflow. Empty technical sections add no gap before Last result.

Esc first cancels voice, otherwise closes or backs from technical help. Reading
cancellation remains after help closes. Help blocks typing, paste and hidden send,
review or voice actions. F10 retains the loss guard, Ctrl-C cancellation, Ctrl-D
Why, and `b` changes the preference without revealing the hidden bar. `?` remains
text while writing. The review bar says `type confirm save goal and record` when recording.

### Editing and focus

- Tab or Shift-Tab toggles writing and the original once present. Typing returns
  to writing and clears original selection. Letters never command; F8 skips and F9 copies.
- Shift with arrows, Home or End, and mouse drag select logical whole graphemes.
  Ctrl-A selects only the active surface. Writing input, Enter, deletion or paste
  replaces selection as one edit. There is no numeric editor-size cap. Allocation
  refusal preserves text and selection. Finite fixtures promise neither unlimited
  memory nor responsiveness.
- Ctrl-E extracts; F7 or Ctrl-X toggles cards. Covered originals have no selection
  and cannot be copied.
- Ctrl-C copies a nonempty active selection, otherwise cancels a request or does
  nothing; it never exits. Dictation never deletes selection.

### Original bytes and display

Writing, sources and cards retain received UTF-8, including CRLF, CR, tabs and controls.
Copy uses base64 OSC 52, not executable source. The host owns clipboard support and
permission. LF starts rows; CR displays as ␍, tab as ⇥, ASCII controls as control
pictures, and C1 as �. CRLF is one grapheme. Display-only marks map cells to raw
grapheme offsets for motion, selection, highlighting and voice preview, never
substring reconstruction. Rows use machine-sized integers; only the visible viewport
narrows to terminal coordinates.

Literal ESC `[201~` ends bracketed paste and cannot occur inside its envelope.
Crossterm replaces invalid UTF-8 before Paste delivery. Retain received strings,
not unavailable predecode bytes. Valid-UTF-8 complete-envelope tests include OSC
sentinels: bytes survive sending and copying, never execute on display.

### Sending and leaving

- The encoded shaping request limit is 32 KiB, including metadata, history and the
  waiting reply's refreshed reading. One shared preview measures the final payload
  as it grows or shrinks. Idle frames reuse it until input, extraction, voice or
  a reply arrives. Unapplicable replies make size unknown; F2 preserves the old
  reading and sends one retry. Transactional preflight refusal starts no host and
  commits neither words nor the waiting reply. Copy a fitting replacement with
  Ctrl-A then Ctrl-C, deliberately replace it and send once. If retained context
  alone exceeds the limit, new words cannot fix it. F1 explains copying local words
  and originals using Tab, Ctrl-PgUp or Ctrl-PgDn, then Ctrl-A and Ctrl-C.
  Copying never prunes history or restarts.
- Distinguish provider input or context rejection, rate limits, timeout and overflow.
  Shaping failure preserves committed originals and the prior reading. Failed,
  unreadable or oversized review is best-effort: keep valid shaping and explain the
  skipped review in Why. Never automatically retry, chunk, summarize or send.
- With nothing to send or retry, F2 says “Write something, then F2 sends it.” on the
  initial or refining screen, without requesting or saving. The next input clears
  it; there is no permanent warning.
- One `refuse` route explains ineffective keys and recovery under writing or in the
  review's confirm slot. It takes at most two narrow-pane rows and overrides
  Thinking and receipts. The next key or paste clears display; Last result retains
  it. Review names unsent words, a waiting reply or an in-progress reading.
  Enter before the reading's end says PgDn. Confirmation saves only from the end.
- An unfinished save says “Handoff not complete. F1 then h shows why; Enter retries.”
  Last result retains the reason. Review messages grow, leaving Back its own row.
- Idle Esc and F10 guard unsaved work. `--home` returns to the menu; standalone
  exits before saving, opens the menu after saving. Esc first cancels voice or the
  reading, closes overlays, leaves review, clears original selection or focus,
  or declines the loss prompt.
- F4 originals and F5 why stay help-only. [Local voice](voice.md) inserts editable
  words, never sends. Answering keeps the original dump visible.

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
