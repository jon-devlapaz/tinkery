# Backlog

These proposals are drafts, not published issues or implementation permission.
Review each remaining slice with the operator before publishing; approved slices
get one branch/PR each, linked to their issue. Completed portions are listed below
and in each proposal's context.

Every proposal inherits the [working guardrails](../../CLAUDE.md#guardrails):
temporary TEST roots, fake providers/voice, zero calls without an explicit budget,
regression tests and important reversion probes. Each don't-list also names the
risks specific to its slice.

The [first-release plan](first-release.md) selects a complete six-stage journey.
Its [issue drafts](first-release-issues.md) split the release work into bounded
tasks; R1, R3 and R4 take the evaluation work in proposal 7 below. The other
proposals remain optional unless the release trial demonstrates a blocker.

## Loose ends

Known gaps found while building, smaller than a proposal. Each still needs an
approved issue before work starts.

- **"Like X" lines stop think early.** The review can leave an ambiguous source or
  scope in place. See think's [known gap](think.md#open-work-only).
- **Think's routing remains large.** Split `src/think/brain_dump.rs` further without
  loosening the [architecture limits](architecture.md#limits-enforced-by-testsarchitecturers).
- **The review merges by top-level keys.** `review::merge` writes its decisions and
  questions into the raw reading by key, the residue named in
  [architecture](architecture.md) diagnostic 3.
- **Think records keep raw model replies.** Each check's `raw` reply goes into the
  [think record](think.md#think-record), so records from long sessions get large.
  Decide what a record needs from the reply.
- **An unset `HOME` writes records to a relative path.** Without `HOME` or
  `--record-root`, records land under `.local/share/tinkery/think` in the current
  folder. Refuse or say where instead.
- **Possible approaches can vanish.** The reading prompt names `alternatives` without
  their shape. In #86's live recheck the model keyed each one `option`; `reading.rs`
  reads only `label`, so all three were dropped and the section was empty.
- **Gaps across think records aren't flagged yet.** The rest of the record slice:
  the eval harness writes records with origin `eval`, and a separate script reads
  them and flags gaps.

## Previously published, completed issues

Closed issues with merged PRs: history, not proof that a whole proposal is complete.

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

**Done:** [#86](https://github.com/jon-devlapaz/tinkery/issues/86). A live baseline at
fadab4c (6 calls, the Oct 9 dogfood dumps) no longer invented deadlines or asked
generic Done-when questions. It still turned an injected instruction into an
"also in your dump" goal; one reading-prompt rule fixed that in a 2-call recheck.
One run per dump is a small sample, not operator recognition.

## 2. Context menu and send aliases (C4–C6, C9)

**Context:** Writing selection and Tab into the original shipped in issue #14
([think](think.md#interaction-contracts)). There is still no right-click menu or
modified-Enter send. Terminal hosts report modified keys and right clicks differently.

**Build:** Right-click offers Copy/Cut/Paste/Select all where meaningful, Copy/Skip
question on a question, and closes on Esc/click-away. Cmd/Ctrl+Enter sends only when
distinctly reported; F2 remains reliable and visible.

**Acceptance:** Context-specific menus pass tests at keyboard and mouse. Plain Enter
remains a newline; letters remain text. Document/test host forwarding and unsupported
shortcuts honestly; no fabricated clipboard success.

**Read:** [think](think.md), [principles](principles.md), [terminal setup](../terminal-setup.md).

**Don't:** Introduce a focus-management key maze, intercept unreported host shortcuts,
make copy/send confirm anything, record audio, call providers without budget,
or change terminal/launcher settings or operator data.

## 3. Evidence on demand (C13)

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

## 4. A calm dark theme

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

## 5. Multiple-choice answers

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

## 6. Scratch-space spike — design only

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

## 7. Operator-supplied real-dump eval harness

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
