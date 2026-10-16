# Think

## Shipped experience

A blank box asks what's on your mind. Type or paste without choosing fields,
kinds or cards. Nothing interprets the text until F2. Enter stays a newline.
The original remains intact on the left; one reading, titled “I think you mean…”,
and at most one question in focus sit on the right. No second reading.
If interpretations compete, ask the question that decides between them.

### Questions

The reading follows the [goal-block contract](goal-block.md). Ask about
consequential missing intent, not facts an agent could discover. Once the
required parts are present, ask the most consequential open decision: a choice only
the person can make whose answers would change a goal line (a boundary, a scope, what
happens to protected things). The reading lists decisions as typed items (part, text,
two or more answers, and an optional linked question). Each keeps its part open, so it blocks "Nothing I'm unsure
about" until an answer settles it, and travels to review and the seed. Review
shows its linked question once; an unasked or unlinked choice stays an "Open
decision" note. Alternatives remain in the handoff even when a question replaces
the note on screen. A decision without two answers or a known part is ignored.
After each reading, a separate review call sees only the person's words, their
answers, the draft parts and shaping decisions (not the template) and names decisions the draft settled
silently: a target or scope, a boundary on deleting, sending or overwriting, what
happens to protected things, persistence, whose rules apply. Its decisions join the
reading's in the reading's decoder; the recorded reply stays as the model sent it.
For the same choice, the review explicitly names the shaping decision indices it covers; only valid references to the same part replace those entries.
Uncovered choices remain, including different choices about the same part. No
text-similarity or same-part rule guesses that two decisions are the same. Once
required parts are present after the reading's cleanup, its top question is asked first.
A Why or Done-when removed as a duplicate still needs its specific missing-part
question before a review decision. A
failed or unreadable review leaves the reading as it was. It stops once the target
and any irreversible boundary are settled: no edge cases the person's rules already
cover and no questions about how an action is carried out (a live trial asked
"whole folders or just files?" after the real decision was answered). Each answer
lands in the one part it answers rather than being repeated across parts.

Questions stop when no decision is left; the [known early-stop gap](#open-work-only)
remains open. Unresolved notes carry only open intent, never status disclaimers. Statements about who produced evidence, which tests ran,
or a historical failure not reproduced stay in the intact originals. The reading
prompt does not turn that context into goal requirements or unfinished intent;
explicit requirements and undecided success checks or permission boundaries still
belong in the goal or its open questions. Answers and added dumps remain distinct
exact sources. Sending refines the reading; refining a ready reading does not start
a new dump. Skipped questions stay skipped;
Ctrl-Z can restore a skip. Separate goals stay visibly aside, not in each other's
constraints. Deferred items remain context for the seed.

Constraint-polarity errors remain possible; decision coverage remains a model
judgment, not proof that choices mean the same thing. See the
[bounded #64 evidence](../../findings/README.md#goal-history-and-review-regression-check-64).

The original is plain ink by default. Exact cited spans are validated invisibly;
uncited words are not implicitly rejected.

### Model calls

Budget at most two calls per live send: one shaping call, then one decision review.
Some sends ask about scope without calling a model. Nothing runs after the reading
appears, and nothing retries automatically. Practice and the documented suite use
fakes. [Historical evaluation](#decision-review-evidence-34) is evidence, not a
general quality guarantee.

Both prompts live in `src/think/brain_dump/prompts/`, one rule per line, with a note
naming the eval or live-trial finding behind each tuned rule; notes never reach the
model. A test holds the bytes sent to Pi equal to a recorded copy, and another holds
each prompt's field names equal to its decoder's, so a wording change is deliberate
and needs an eval and the operator's budget.

### Readiness and review

When no question is in focus (none left, skipped, or a part asked twice), one
muted line says which of three states holds:

| State | Line |
| --- | --- |
| Goal ready | Nothing I'm unsure about. Review when you're ready. |
| A required part is missing | Still missing: Done when. Review when you're ready. (names the part) |
| Required parts present, something else open | Some parts remain open for the seed. Review when you're ready. |

The missing line names only the first missing part in the order who, outcome, why,
done when. Without any composed goal it drops “Review when you're ready.” F1 help
uses the same first sentence, then offers F3 only when a composed goal exists and can be
saved (practice or no Seed Me: says saving needs real mode and Seed Me; no goal: type
it, then F2 sends). If the first reading was cancelled or never applied, help says
F2 tries again. Only goal content decides the state, including sent words not yet
applied and any skipped question still unanswered; recording or
unsent text does not change it. Review still lists everything open.

Review is deliberate (F3), then typed `confirm` plus Enter. It freezes the displayed
goal; [the receipt](shape-and-handoff.md) follows only successful saving. Review can
carry remaining questions and missing parts into the seed: readiness is not needed
to save, and saving is not seed or implementation approval.

## Think record

The same typed `confirm` also writes a **think record**: one pretty-printed JSON file
per confirmed session in `~/.local/share/tinkery/think/` (`--record-root` overrides it),
named `<UTC confirm time>_<Seed Me session id>.json`. With no `--record-root` and an
unset, empty or relative `HOME`, `--shape-pi` refuses to start and says to pass one,
rather than writing under the current folder. The review prompt and key bar say
so (`type confirm` → `save goal and record`), so the write rides a labelled action
([Yohaku article 4](../yohaku.md#the-articles)). Sessions that end without confirming
(Esc, F10, quit, crash) leave no record. This is the operator's chosen boundary.

A record holds `schema` (3), the tinkery version, `origin` (`live` for Pi,
`simulated` otherwise), model, thinking level and a hash of both system prompts, the
start (first turn) and confirm times, the Seed Me folder name, canonical session path
and source-owned session ID, `repo` (the absolute root of the Git repository Tinkery
was started in, or `null` outside one), every `turn` in the order
it happened (`words`, `asked`, `answered`, `skipped`, `restored`, `scope_asked`,
`scope_answered`, each with its time and the person's words verbatim), every F5 check
as it already serializes, and the goal block exactly as confirmed. A check keeps the
model's `raw` reply only when it rejected, where F5 shows it; a passing check's reply
is already in its decoded `candidate`, so its `raw` is `null`. Each Think
visit has its own Pi diagnostic history, so a new goal cannot collect a previous
goal's replies. Reusing the model settings starts no process and rereads no file;
all sends in the current goal remain in its own history. The record folder is
preserved when standalone Think returns to the menu and opens another goal.

`repo` says where the goal was made, never what it is about. When it will be
recorded, review shows `made in <folder>` under the goal before `confirm`, so nothing
is saved that the person hasn't seen ([Yohaku article 4](../yohaku.md#the-articles)).
It stays out of the goal, the Seed Me ledger and every model request; the operator's
words and Tinkery's observations stay apart (article 3).

The shared `src/think_record.rs` owns the format; `Conversation` keeps the turns. The record is
written after Seed Me's save reads back, so a failed save writes none. If the record
can't be written the goal stays saved and the receipt says the record wasn't written;
the reason and path stay in How it works.
Think never reads a record back. The record is not a ledger: that word is Seed Me's
`ledger.json`. Tend's deliberate [history view](shape-and-handoff.md#think-history)
reads it without supplying model context. Gap flags and the eval harness reading
records are separate slices.

## Interaction contracts

### Reading

When the reading overflows, a muted `PgUp/PgDn scroll reading` line stays below it.
The existing keys and mouse wheel reveal the whole reading before review; Every key
also names the keyboard route. The hint disappears when the reading fits, including
after a resize. Scrolling changes no goal text or original words
([#117](https://github.com/jon-devlapaz/tinkery/issues/117)).

### Help

The state-aware muted bar uses F2, F3, F6, F8 and F10 labels that agree with help.
It never offers a key the current mode refuses: F3 appears only in real mode with Seed Me.
Empty-box help says “Type, then F2 sends it.” F1 opens a bounded overlay on the
reading side; originals and drafts stay intact. Right now leads with the current
state, F1 opens Every key, and `h` opens How it works.

Jade names the layer; headings and keys are bold ink, labels and the thin border
muted. Short help fits its content. Long pages scroll beneath a fixed title and
close/back/scroll keys; Scroll appears only on overflow. Empty technical sections
add no spacing before Last result.

Esc first cancels an active voice take, otherwise closes or returns from How it
works. Reading cancellation remains available after closing help. While help is
open, typing, paste and hidden send/review/voice actions do nothing. F10 keeps the
unsaved-loss guard, Ctrl-C keeps cancellation, Ctrl-D keeps its explicit Why route,
and `b` changes the key-bar preference without revealing the bar beneath help.
Literal `?` remains text while writing. On review the bar reads
`type confirm save goal and record` when a record will be written.

### Editing and focus

- **Focus:** Tab or Shift-Tab switches between writing and the left original once
  one exists. Typing returns to writing and clears the original's selection.
  Letters are never commands in either surface; F8 skips and F9 copies.
- **Selection:** Shift with arrows, Home or End and mouse drag select by logical
  position and whole graphemes. Ctrl-A selects only the active surface. In writing,
  typing, Enter, Backspace, Delete and paste replace the selection as one edit.
  The writing box has no numeric request-size cap. Allocation refusal leaves the
  previous text and selection intact; finite regression fixtures do not promise
  unlimited memory or responsiveness.
- **Cards:** Ctrl-E extracts a selection; F7 or Ctrl-X shows/hides the cards.
  While they cover the original, it holds no selection and cannot be copied.
- **Copy and dictation:** Ctrl-C copies the active surface's nonempty selection;
  otherwise it cancels a running request or does nothing. It never leaves think.
  Dictation never deletes a selection.

### Original bytes and display

The writing box, sources and extracted cards retain received UTF-8 exactly,
including CRLF, CR, tabs and controls. Copying uses base64 OSC 52, not executable
source bytes. Terminal clipboard support and permission remain host capabilities.
LF starts a row; CR displays as ␍, tab as ⇥, ASCII controls as control pictures,
and C1 controls as �. CRLF remains one grapheme. These marks are display only.
Each wrapped cell maps to a raw grapheme offset for motion, selection, highlights
and fake/local voice preview; no substring search reconstructs source offsets.
Editor and original scroll rows use machine-sized integers, not terminal u16
coordinates. Only the visible viewport is narrowed to terminal coordinates.

The terminal paste envelope is a separate boundary. A literal ESC `[201~` closes
bracketed paste and cannot be represented inside that envelope. Crossterm decodes
invalid UTF-8 with replacement characters before delivering a Paste string.
Tinkery retains that received string, not unavailable pre-decoding bytes. Tests
use valid UTF-8 and a complete envelope, including executable OSC sentinels whose
raw bytes survive copying and sending but never execute during display.

### Sending and leaving

- The encoded shaping request still has a 32 KiB boundary, including metadata,
  history and a waiting reply's refreshed reading. One shared preview prepares and
  measures the final outgoing request, whether the waiting reading grows or shrinks.
  Idle frames reuse the size until a key, paste, mouse event, extraction, voice
  insertion or reply arrives. A waiting reply that cannot be applied leaves the preview
  size unknown.
  F2 keeps the previous reading on that failure and still sends one retry.
  Preflight is transactional:
  refusal starts no host and commits neither local words nor the waiting reply.
  If a fitting replacement exists, Ctrl-A then Ctrl-C copies the full draft;
  deliberate replacement and F2 sends once. If retained context alone cannot
  fit, editing new words cannot fix this conversation. F1 explains copy-out:
  copy local words, Tab to the original pane, Ctrl-PgUp/PgDn between originals,
  and Ctrl-A then Ctrl-C for each. Copying does not prune history or start over.
- Provider input/context rejection, rate limits, timeout and output overflow are
  separate failures. Shaping failure keeps committed originals and the previous
  reading. Failed, unreadable or oversized decision review is best-effort and
  does not discard an otherwise valid shaping result; Why names the skipped review.
  Nothing retries, chunks, summarizes or sends automatically.
- When there is nothing to send or retry, F2 shows “Write something, then F2
  sends it.” on the normal screen, both before a first reading and while refining
  one. It sends no request and saves nothing. The next input clears the notice;
  the empty screen has no permanent warning.
- **Refusals:** a key that does nothing says why, and what to do instead, where
  the person is looking: on the status row under the writing box, or in place of
  the confirm instruction on review. Every refusal goes through one route
  (`refuse`), fits two rows of the narrowest pane, and wins the status row over
  “Thinking…” and receipts. The next key or paste clears it from the screen; it
  stays as Last result in How it works. Review names its own blocker (unsent
  words, a waiting reply, a reading still coming), and Enter on an unread review
  says to scroll with PgDn; confirm still saves only from the end.
- **A save that didn't finish** shows “Handoff not complete. F1 then h shows
  why; Enter retries.” in the same place; the reason stays as Last result. The
  review's message rows grow to fit, so Back always keeps a row of its own.
- Esc from idle writing and F10 guard unsaved work. With `--home` they return
  to the menu; standalone think exits before a goal is saved and opens the menu
  after saving. Esc still cancels voice or a reading first, closes overlays,
  returns from review, clears original selection/focus, or declines the loss prompt.
- F4 originals and F5 why are help-only, not permanent bar clutter.
- Local dictation inserts editable text, never auto-sends; see [voice](voice.md).
- Answering does not switch the original away from the dump.

## Open work only

The [parked backlog](../research/shared/backlog.md#think) owns remaining proposals,
including the ["Like X" early-stop gap](../research/shared/backlog.md#like-x-early-stop).
Extracted-card shortcuts remain in technical help; they are not the proposed scratch
space.

## Decision-review evidence (#34)

See [historical evidence](../../findings/README.md#decision-review-evidence-34).

## Evidence-note regression check (#58)

See [historical evidence](../../findings/README.md#evidence-note-regression-check-58).

## Goal-history and review regression check (#64)

See [historical evidence](../../findings/README.md#goal-history-and-review-regression-check-64).
