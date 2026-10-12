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
failed or unreadable review leaves the reading as it was.
Questions stop when no decision is left. Unresolved notes carry only open intent,
never status disclaimers. Answers
and added dumps remain distinct exact sources. Sending refines the reading;
refining a ready reading does not start a new dump. Skipped questions stay skipped;
Ctrl-Z can restore a skip. Separate goals stay visibly aside, not in each other's
constraints. Deferred items remain context for the seed.

The original is plain ink by default. Exact cited spans are validated invisibly;
uncited words are not implicitly rejected. A background meaning/continuity audit
is log-only: it does not gate a valid reading, retry it, or show warning flags.
In live mode a submitted reshape includes a shaping call, the decision review and
that audit, so budget for all three. Practice and the documented suite use fakes.

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

## Interaction contracts

- State-aware muted bar; two-layer F1 help, technical details only on demand.
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
