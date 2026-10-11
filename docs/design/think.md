# Think

## Shipped experience

A blank box asks what's on your mind. Type or paste without choosing fields,
kinds or cards. Nothing interprets the text until F2. Enter stays a newline.
The original remains intact on the left; one reading, titled “I think you mean…”,
and at most one question in focus sit on the right. No second reading or F7 toggle.
If interpretations compete, ask the question that decides between them.

The reading uses the [goal block](goal-block.md), not dense template sentences.
Ask about consequential missing intent, not facts an agent could discover. Answers
and added dumps remain distinct exact sources. Sending refines the reading;
refining a ready reading does not start a new dump. Skipped questions stay skipped;
Ctrl-Z can restore a skip. Separate goals stay visibly aside, not in each other's
constraints. Deferred items remain context for the seed.

The original is plain ink by default. Exact cited spans are validated invisibly;
uncited words are not implicitly rejected. A background meaning/continuity audit
is log-only: it does not gate a valid reading, retry it, or show warning flags.
In live mode a submitted reshape includes a shaping call and that audit, so budget
for both. Practice and the documented suite use fakes.

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
it, then F2 sends). Only goal content decides the state, including sent
words not yet applied and any skipped question still unanswered; recording or
unsent text does not change it. Review still lists everything open.

Review is deliberate (F3), then typed `confirm` plus Enter. It freezes the displayed
goal; [the receipt](shape-and-handoff.md) follows only successful saving. Review can
carry remaining questions and missing parts into the seed: readiness is not needed
to save, and saving is not seed or implementation approval.

## Interaction contracts

- State-aware muted bar; two-layer F1 help, technical details only on demand.
- Letters are text while writing. Ctrl-C copies an existing selection; otherwise
  it cancels a running request or does nothing. It never leaves think.
- F10 is guarded when leaving loses unsaved work, and returns to the menu.
- F4 originals and F5 why are help-only, not permanent bar clutter.
- Local dictation inserts editable text, never auto-sends; see [voice](voice.md).
- Answering does not switch the original away from the dump. This is an interim
  fix, **not** the proposed notecard stack.

## Open work only

[Drafts 1–4 and 7–9](backlog.md) cover goal polish, the notecard stack, regular
text-field selection and right-click behavior, a keyboard route into the original,
per-sentence evidence on demand, multiple-choice answers, scratch-space research,
and an operator-supplied eval harness. These are not shipped features.
Existing extracted-card shortcuts remain in technical help; the earlier
`--scratchpad` prototype is neither the default nor a menu destination.
