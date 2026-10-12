# Think

## Shipped experience

A blank box asks what's on your mind. Type or paste without choosing fields,
kinds or cards. Nothing interprets the text until F2. Enter stays a newline.
The original remains intact on the left; one reading, titled “I think you mean…”,
and at most one question in focus sit on the right. No second reading.
If interpretations compete, ask the question that decides between them.

The reading uses the [goal block](goal-block.md), not dense template sentences.
Ask about consequential missing intent, not facts an agent could discover. Once the
required parts are present, ask the most consequential open decision: a choice only
the person can make whose answers would change a goal line (a boundary, a scope, what
happens to protected things). The reading lists decisions as typed items (part, text,
two or more answers). Each keeps its part open, so it blocks "Nothing I'm unsure
about" until an answer settles it, and travels to review and the seed as an
"Open decision" note. A decision without two answers or a known part is ignored.
After each reading, a separate review call sees only the person's words, their
answers and the draft parts (not the template) and names decisions the draft settled
silently: a target or scope, a boundary on deleting, sending or overwriting, what
happens to protected things, persistence, whose rules apply. Its decisions join the
reading's, and once required parts are present its top question is asked first; a
failed or unreadable review leaves the reading as it was. It stops once the target
and any irreversible boundary are settled: no edge cases the person's rules already
cover and no questions about how an action is carried out (a live trial asked
"whole folders or just files?" after the real decision was answered). Each answer
lands in the one part it answers rather than being repeated across parts.

Known gap: the review can stop one question early when a line points at something
by example ("like the eval stuff we just ran") that could mean two different
sources or scopes. In a live trial it left that reading in place, and an agent would
have built from the wrong source. The "anything to add or change?" box corrects it
in one line. A candidate fix is a final check for such lines before "Nothing I'm
unsure about"; it is not built or measured yet.
Questions stop when no decision is left. Unresolved notes carry only open intent,
never status disclaimers. Answers
and added dumps remain distinct exact sources. Sending refines the reading;
refining a ready reading does not start a new dump. Skipped questions stay skipped;
Ctrl-Z can restore a skip. Separate goals stay visibly aside, not in each other's
constraints. Deferred items remain context for the seed.

The original is plain ink by default. Exact cited spans are validated invisibly;
uncited words are not implicitly rejected. A send is one shaping call plus one
decision review, both before the reading is shown, so budget for two; nothing runs
after it appears. Practice and the documented suite use fakes. (History: the review
replaced a log-only meaning audit that ran in the background and never changed a
reading; #34 deleted it. In the v3 eval the review roughly doubled the decisions
asked, from 23% to 40%, with questions on clear dumps staying within the
0.5-per-run target. A drop in silent guesses, 18% to 14%, is likely but unproven:
the difference is within noise.)

Both prompts live in `src/think/brain_dump/prompts/`, one rule per line, with a note
naming the eval or live-trial finding behind each tuned rule; notes never reach the
model. A test holds the bytes sent to Pi equal to a recorded copy, and another holds
each prompt's field names equal to its decoder's, so a wording change is deliberate
and needs an eval and the operator's budget.

When no question is in focus (none left, skipped, or a part asked twice), one
muted line says which of three states holds:

| State | Line |
| --- | --- |
| Goal ready | Nothing I'm unsure about. Review when you're ready. |
| A required part is missing | Still missing: Done when. Review when you're ready. (names the part) |
| Required parts present, something else open | Some parts remain open for the seed. Review when you're ready. |

The missing line names only the first missing part in the order who, outcome, why,
done when. Without any composed goal it drops “Review when you're ready.” F1 help
uses the same first sentence, then offers F3 only when a composed goal exists (else: type
it, then F2 sends). If the first reading was cancelled or never applied, help says F2 tries again. Only goal content decides the state, including sent
words not yet applied and any skipped question still unanswered; recording or
unsent text does not change it. Review still lists everything open.

Review is deliberate (F3), then typed `confirm` plus Enter. It freezes the displayed
goal; [the receipt](shape-and-handoff.md) follows only successful saving. Review can
carry remaining questions and missing parts into the seed: readiness is not needed
to save, and saving is not seed or implementation approval.

## Think record

The same typed `confirm` also writes a **think record**: one pretty-printed JSON file
per confirmed session in `~/.local/share/tinkery/think/` (`--record-root` overrides it),
named `<UTC confirm time>_<Seed Me session id>.json`. The review prompt and key bar say
so (`type confirm` → `save goal and record`), so the write rides a labelled action
(Yohaku article 4). Sessions that end without confirming (Esc, F10, quit, crash) leave
no record; that was the operator's call on 2026-10-12, to revisit after reading a few.

A record holds `schema` (1), the tinkery version, `origin` (`live` for Pi,
`simulated` otherwise), model, thinking level and a hash of both system prompts, the
start (first turn) and confirm times, the Seed Me session id, every `turn` in the order
it happened (`words`, `asked`, `answered`, `skipped`, `restored`, `scope_asked`,
`scope_answered`, each with its time and the person's words verbatim), every F5 check
as it already serializes, and the goal block exactly as confirmed.

`src/think/record.rs` owns the format; `Conversation` keeps the turns. The record is
written after Seed Me's save reads back, so a failed save writes none. If the record
can't be written the goal stays saved and the receipt says the record wasn't written.
Think never reads a record back. The record is not a ledger: that word is Seed Me's
`ledger.json`. Gap flags and the eval harness reading records are separate slices.

## Interaction contracts

- State-aware muted bar; two-layer F1 help, technical details only on demand.
  On review the bar reads `type confirm save goal and record` when a record will be written.
- Letters are text while writing. Tab or Shift-Tab moves between writing and the
  left original once one exists; the same key returns, and anything typed returns
  to writing and drops the original's selection. No letter is a command, with
  either surface focused; skipping, copying and the cards use F8, F9 and F7 (Ctrl-X).
  Cards that cover the original hide it: it holds no selection and cannot be copied. In writing, Shift with arrows,
  Home or End and mouse drag select by logical position and whole graphemes;
  Ctrl-A selects all of the active surface only; typing, Enter, Backspace, Delete
  and paste replace it as one edit, refused whole past the 4,096-byte limit.
  Dictation never deletes a selection. Ctrl-C copies the active
  surface's nonempty selection; otherwise it cancels a running request or does
  nothing. It never leaves think. F4 (the reader) and F6 (voice) are separate.
- F10 is guarded when leaving loses unsaved work, and returns to the menu.
- F4 originals and F5 why are help-only, not permanent bar clutter.
- Local dictation inserts editable text, never auto-sends; see [voice](voice.md).
- Answering does not switch the original away from the dump. This is an interim
  fix, **not** the proposed notecard stack.

## Open work only

[Drafts 1–4 and 6–8](backlog.md) cover goal polish, the notecard stack, right-click
and send aliases, per-sentence evidence on demand, multiple-choice answers,
scratch-space research and an operator-supplied eval harness. None is shipped.
Extracted-card shortcuts remain in technical help.
