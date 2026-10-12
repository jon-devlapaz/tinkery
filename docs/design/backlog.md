# Backlog — remaining proposals and completed issues

The eight proposals below describe remaining work, not published issues or
implementation permission. Some earlier portions have shipped; their completed
issues are listed separately below and noted in each proposal's context.
Review the remaining scope with the operator before publishing a new issue. When
approved, work on one branch/PR per bounded slice and link the published issue.
All acceptance work follows `CLAUDE.md`: temporary TEST roots, fake providers/voice,
zero calls unless the operator gives a budget, regression tests and important
reversion probes.

## Previously published, completed issues

These issues are closed and their implementation PRs are merged. They are history,
not remaining work or evidence that an entire related proposal is complete.

| Issue | Completed scope | Merged PR |
| --- | --- | --- |
| [#13](https://github.com/jon-devlapaz/tinkery/issues/13) | Explicit goal-prefix normalization and accurate open-intent wording | [#18](https://github.com/jon-devlapaz/tinkery/pull/18) |
| [#14](https://github.com/jon-devlapaz/tinkery/issues/14) | Grapheme-safe writing selection and keyboard access to the original | [#23](https://github.com/jon-devlapaz/tinkery/pull/23) |
| [#15](https://github.com/jon-devlapaz/tinkery/issues/15) | Nonblocking menu session discovery | [#24](https://github.com/jon-devlapaz/tinkery/pull/24) |
| [#16](https://github.com/jon-devlapaz/tinkery/issues/16) | Current terminal-setup documentation | [#17](https://github.com/jon-devlapaz/tinkery/pull/17) |
| [#19](https://github.com/jon-devlapaz/tinkery/issues/19) | One line per constraint item, without re-splitting | [#20](https://github.com/jon-devlapaz/tinkery/pull/20) |
| [#21](https://github.com/jon-devlapaz/tinkery/issues/21) | Legacy scratchpad removal | [#22](https://github.com/jon-devlapaz/tinkery/pull/22) |
| [#25](https://github.com/jon-devlapaz/tinkery/issues/25) | Stale documentation and dated evidence pruning | [#26](https://github.com/jon-devlapaz/tinkery/pull/26) |
| [#27](https://github.com/jon-devlapaz/tinkery/issues/27) | Legacy workbench demo removal | [#28](https://github.com/jon-devlapaz/tinkery/pull/28) |
| [#29](https://github.com/jon-devlapaz/tinkery/issues/29) | Unreachable note-card prototype removal | [#30](https://github.com/jon-devlapaz/tinkery/pull/30) |

## 1. Goal-block polish

**Context:** Constraint prefixes, clause casing, the three-state no-question line and
one line per constraint shipped in issues #13 and #19 ([goal block](goal-block.md#constraint-syntax)).
Dogfood still found generic Done-when questions, repeated or invented timing, and an
injected instruction appearing as another goal.

**Build:** Ask concrete, dump-specific success questions. Avoid repeating `when` or
turning remembered timing into a deadline. Keep injected instructions out of “also in
your dump”.

**Acceptance:** Fixture tests cover repeated/false deadlines, concrete questions and
injected-topic exclusion, without hiding unresolved questions or inventing a new
consent gate. The frozen reviewed block remains identical in receipt, stored origin
and detail. Test the protection reverted; operator recognition needs separately
approved dogfood, not a claim from stubs.

**Read:** [goal block](goal-block.md), [think](think.md), [shape and handoff](shape-and-handoff.md).

**Don't:** Rewrite originals, force “can”, invent numbers/deadlines, lose deferred
work, change Seed Me, spend provider budget without approval, or write operator sessions.

## 2. The notecard stack (D4)

**Context:** Answering no longer switches away from the dump, but answers are still
paged sources, not a stack. A person should see their original thought while refining it.

**Build:** Keep the whole dump at the top. Put answers below as real notecards with
muted question edges; newest open, others folded to their edges. Click an edge or
use arrows outside typing to open a card. Long dumps scroll independently with
edges pinned. Replace “Words n / m” and source paging, without exposing sticky organization first.

**Acceptance:** Several answers and a long dump remain reachable without the dump
vanishing. Card headers identify the answered question, keyboard/click routes agree,
typing isn't stolen, exact text survives folding, and returning keeps position.
PTYs cover small and wide sizes; revert the dump-retention protection to prove a regression.

**Read:** [think](think.md), [principles](principles.md).

**Don't:** Rewrite/split the dump automatically, add category colors, require
organization, implement the scratch-space spike, call providers without budget,
or persist to operator roots.

## 3. Context menu and send aliases (C4–C6, C9)

**Context:** Writing selection and Tab into the original shipped in issue #14
([think](think.md#interaction-contracts)). There is still no right-click menu or
modified-Enter send. Terminal hosts report modified keys and right clicks differently.

**Build:** Right-click offers Copy/Cut/Paste/Select all where meaningful, Copy/Skip
question on a question, and closes on Esc/click-away. Cmd/Ctrl+Enter sends only when
distinctly reported; F2 remains reliable and visible.

**Acceptance:** Context-specific menus pass tests at keyboard and mouse. Plain Enter
remains a newline; letters remain text. Document/test host forwarding and unsupported
shortcuts honestly; no fabricated clipboard success.

**Read:** [think](think.md), [principles](principles.md), `docs/terminal-setup.md`.

**Don't:** Introduce a focus-management key maze, intercept unreported host shortcuts,
make copy/send confirm anything, record audio, call providers without budget,
or change terminal/launcher settings or operator data.

## 4. Evidence on demand (C13)

**Context:** Always-on highlights were removed. Exact-span checks remain, but a
person cannot ask which words support just one sentence of the reading.

**Build:** Click or keyboard-point at a reading sentence to reveal only its cited
original spans. Extend the response contract to support citations per sentence;
validate exact source offsets. Consider “words not used” only as a deliberate view.

**Acceptance:** Multiple sentences/sources and repeated Unicode quotes map to the
correct exact spans. Invalid citations fail explicitly; selecting one sentence
never lights all evidence. Clearing returns to plain original ink. Keyboard and
mouse agree; NO_COLOR cues remain. Revert exact-span validation to prove the test fails.

**Read:** [think](think.md), [principles](principles.md).

**Don't:** Restore permanent green, rewrite originals, imply unused words are
rejected, use providers without budget, or write operator sessions.

## 5. A calm dark theme

**Context:** The operator wants a calm, not generic-black terminal. The shipped
palette is light. Jade must still mean location; rust must still mean consequence.

**Build:** Propose a restrained dark palette for operator review; follow macOS dark
mode automatically, including changes while running. Define an honest non-macOS
fallback without treating unavailable appearance as known. Keep NO_COLOR working.

**Acceptance:** Review light/dark text mock-ups with the operator. Measure actual
foreground/background contrast for ordinary text (at least 4.5:1), including muted
hints and semantic jade/rust. Fake appearance-change tests cover automatic switching;
snapshot/PTY tests cover selection, consequence prompts and NO_COLOR's non-color cues.
Changing appearance must preserve input, navigation and the confirmed goal.

**Read:** [principles](principles.md), [think](think.md), [menu](menu.md).

**Don't:** Substitute generic black, add category colors or a settings maze,
change terminal/macOS preferences, break NO_COLOR, record audio, spend provider
budget without approval, or touch operator installs/data.

## 6. Multiple-choice answers

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

**Read:** [think](think.md), [goal block](goal-block.md), [principles](principles.md).

**Don't:** Force choices, turn guesses into facts, trade meaning for speed,
rewrite original answers, call providers without budget, or write operator sessions.

## 7. Scratch-space spike — design only

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

**Read:** [think](think.md), [principles](principles.md), [goal block](goal-block.md).

**Don't:** Implement the canvas proposal, replace dump-first, let arrangement confirm
anything, mix guesses with verbatim cards, persist canvases, invent operator dumps,
or touch operator data/settings. Research model calls need explicit budget.

## 8. Operator-supplied real-dump eval harness

**Context:** Stub tests prove mechanics, not whether a messy human dump becomes a
goal they recognize. Historical dogfood exposed actor, constraints, timing and
injection failures. The operator must supply and permit use of the real dumps.

**Build:** A harness accepting an explicitly supplied dataset, with provenance,
expected invariants, isolated roots, configured provider/model and hard call budget.
Provide a zero-call fake mode first. Report actor/outcome preservation, missing-line
questions, constraint filing, other/deferred topics, injection handling, exact
confirmation/handoff and operator recognition separately from automated checks.

**Acceptance:** No supplied dataset or no approved real-call budget means no real
calls; exhaustion stops before another request. Fake tests prove limits and error
handling. Record before/after evidence reproducibly without copying private dumps
into the repository or declaring stub quality results to be human recognition.
Operator review owns the quality verdict; counts include each send's decision review.

**Read:** [think](think.md), [goal block](goal-block.md), [shape and handoff](shape-and-handoff.md).

**Don't:** Invent/harvest operator dumps, scrape histories or sessions, commit private
text, silently invoke live examples, record real audio, exceed budget, or write
operator sessions/installs.
