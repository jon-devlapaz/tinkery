# Think backlog

Drafts, not permission. [Work intake](../../../CLAUDE.md#work-intake) says how a draft becomes work.

## Loose ends

- **The think screen can hide Must not lines.** At 100×30 the newsletter goal's exclusions
  were cut mid-line or missing (3/3 live runs, 2026-10-13); review shows them. A boundary the
  person can't see breaks Art 6 (`src/think/brain_dump/layout.rs`).
- **Approaches carry a "PROVISIONAL assumption:" prefix.** Seen in most live readings with
  approaches on 2026-10-13, against reading.txt's "no template prefixes inside values".
  Needs a budgeted prompt check.
- **A separate goal keeps only its label.** Since #74's fix, its check and rules stay in the
  intact originals, not with the other goal. Attaching them to that goal is a design choice.
- **The reading's wire shape keeps data the board drops.** `Guess.outcome` repeats done-when
  and the composed framing text is ignored; the board uses `parts` and framing `supports` only
  (`src/think/brain_dump/reading.rs`, `board.rs`). Two owners for one value.
- **"Like X" lines stop think early.** The review can leave an ambiguous source or
  scope in place. See think's [known gap](../../design/think.md#open-work-only).
- **Think's fallback questions are generic** ([research](question-generation.md)).
  When a required part is empty and the model asked nothing, `board.rs` asks a fixed template
  such as "Who is this for?". User studies find generic, low-quality questions hurt more than
  they help. First step needs no calls: count how often the fallback fires in think records and eval logs.
- **Nothing flags an answer that contradicts an earlier one** ([research](question-generation.md)).
  Multi-turn clarification degrades as conversations grow; tracking conflicting answers helped
  most. Add one eval case with a contradicting answer before changing anything.
- **Think's routing remains large.** Split `src/think/brain_dump.rs` further without
  loosening the [architecture limits](../../design/architecture.md#limits-enforced-by-testsarchitecturers).
- **Gaps across think records aren't flagged yet.** The rest of the record slice:
  the eval harness writes records with origin `eval`, and a separate script reads
  them and flags gaps.

## Context menu and send aliases (C4–C6, C9)

**Context:** Writing selection and Tab into the original shipped in issue #14
([think](../../design/think.md#interaction-contracts)). There is still no right-click menu or
modified-Enter send. Terminal hosts report modified keys and right clicks differently.

**Build:** Right-click offers Copy/Cut/Paste/Select all where meaningful, Copy/Skip
question on a question, and closes on Esc/click-away. Cmd/Ctrl+Enter sends only when
distinctly reported; F2 remains reliable and visible.

**Acceptance:** Context-specific menus pass tests at keyboard and mouse. Plain Enter
remains a newline; letters remain text. Document/test host forwarding and unsupported
shortcuts honestly; no fabricated clipboard success.

**Read:** [think](../../design/think.md), [principles](../../design/principles.md), [terminal setup](../../terminal-setup.md).

**Don't:** Introduce a focus-management key maze, intercept unreported host shortcuts,
make copy/send confirm anything, record audio, call providers without budget,
or change terminal/launcher settings or operator data.

## Evidence on demand (C13)

**Context:** Always-on highlights were removed. Exact-span checks remain, but a
person cannot ask which words support just one sentence of the reading.

**Build:** Click or keyboard-point at a reading sentence to reveal only its cited
original spans. Extend the response contract to support citations per sentence;
validate exact source offsets. Consider “words not used” only as a deliberate view.

**Acceptance:** Multiple sentences/sources and repeated Unicode quotes map to the
correct exact spans. Invalid citations fail explicitly; selecting one sentence
never lights all evidence. Clearing returns to plain original ink. Keyboard and
mouse agree; NO_COLOR cues remain. Revert exact-span validation to prove the test fails.

**Read:** [think](../../design/think.md), [principles](../../design/principles.md).

**Don't:** Restore permanent green, rewrite originals, imply unused words are
rejected, use providers without budget, or write operator sessions.

## Multiple-choice answers

**Context:** Some focused questions naturally offer alternatives, but guesses can
anchor the operator. Free text is currently the answer route.

**Build:** Optional 2–4 likely answers plus a free-text route for choice-shaped
questions; omit choices for open questions. Visibly treat options as guesses,
never preselect. Record an answer as “chose: <option>”, attributed to the operator's
selection, not as an independently observed fact.

**Acceptance:** Keyboard/mouse choice and free text both refine the reading.
No default answer or accidental send; open-ended questions remain uncluttered.
Answer provenance survives review/handoff; selecting confirms no goal or seed.
Fake-provider journeys establish mechanics; evaluate fixation with the operator
only under a separately approved budget/dump set.

**Read:** [think](../../design/think.md), [goal block](../../design/goal-block.md), [principles](../../design/principles.md).

**Don't:** Force choices, turn guesses into facts, trade meaning for speed,
rewrite original answers, call providers without budget, or write operator sessions.

## Scratch-space spike — design only

**Context:** The old `--scratchpad` canvas was removed (issue #21); extracted cards
remain, hidden in technical help. Stickies-first made people organize too early. A scratch space
needs to earn its place after the dump, not become an orphan mode.

**Produce:** Research spatial sensemaking and deliberate grouping; discuss usefulness,
authority and exactness with the operator first. Mock up a 100×30 space seeded with
quoted spans, unused words, the question and existing notes; show entering, grouping,
a visibly distinct agent suggestion and returning to the same place. Decide whether
arrangement is evidence (recommended), what one grouping means, and how editing
creates an operator note while retaining its source. Rearranging alone costs no call.

**Acceptance:** A design report answers those questions, names a task where this
beats another typed line, and recommends build/shrink/drop with reasoning. Dogfood
uses only operator-supplied dumps and explicit budget. No production behavior changes;
a later build needs a separate approved slice.

**Read:** [think](../../design/think.md), [principles](../../design/principles.md), [goal block](../../design/goal-block.md).

**Don't:** Implement the canvas proposal, replace dump-first, let arrangement confirm
anything, mix guesses with verbatim cards, persist canvases, invent operator dumps,
or touch operator data/settings. Research model calls need explicit budget.
