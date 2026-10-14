# Parked ideas

The single home for unpublished proposals across all areas. These ideas are not
implementation permission or the release checklist. Concrete task specifications
live in GitHub Issues; [work intake](../../../AGENTS.md#work-intake) separates
publication, design acceptance and implementation approval.

## Shared safeguards

Work on every draft follows [the guardrails](../../../AGENTS.md#guardrails).
Model/provider calls require explicit budget; the default is zero. Never record
real audio. Preserve original words and answers verbatim. Leave operator data,
sessions, installs, settings and terminal/macOS/launcher preferences untouched.
Proposal-specific restrictions and separate data, evaluation and implementation
permissions remain below; these safeguards do not approve any proposal.

## Published from this inventory

- Release tasks: [R4–R10 and I2](../../design/first-release-issues.md).
- Think exclusions: [#117](https://github.com/jon-devlapaz/tinkery/issues/117).
- Implicit session-root refusal: [#118](https://github.com/jon-devlapaz/tinkery/issues/118).
- Measure failure presentation: [#119](https://github.com/jon-devlapaz/tinkery/issues/119); responsive reads: [#120](https://github.com/jon-devlapaz/tinkery/issues/120). Both need approval; neither authorizes a backend migration.
- Existing Shape proposal: [#106](https://github.com/jon-devlapaz/tinkery/issues/106); no duplicate was created.
- Historical dogfood findings stay in [findings](../../../findings/README.md),
  with their completed issue links. Closed issue bodies and metadata are unchanged.
- I1 run identity and R6 expected-input approval enforcement remain conditional
  upstream capability checks, not newly published issues or presumed missing work.

## Think

### Loose ends

- **Large drafts rewrap the whole buffer.** The debug insertion/wrap/motion/full-selection
  fixture measured 224 to 240 ms for about 131 KB and 65,534 to 65,536 logical rows.
  This is a combined fixture time, not a pure per-keystroke measurement. Wrapping rebuilds
  all rows and cursor lookup scans positions, so typing and motion can lag on large drafts.
  Measure single-edit/render latency separately before choosing wrap caching. Caching is
  later work, not part of paste reliability.

- **Approaches carry a "PROVISIONAL assumption:" prefix.** Seen in most live readings with
  approaches on 2026-10-13, against reading.txt's "no template prefixes inside values".
  Needs a budgeted prompt check.
- **A separate goal keeps only its label.** Since #74's fix, its check and rules stay in the
  intact originals, not with the other goal. Attaching them to that goal is a design choice.
- **The reading's wire shape keeps data the board drops.** `Guess.outcome` repeats done-when
  and the composed framing text is ignored; the board uses `parts` and framing `supports` only
  (`src/think/brain_dump/reading.rs`, `board.rs`). Two owners for one value.
- **"Like X" lines stop think early.** The review can leave an ambiguous source or
  scope in place. See think's [known gap](#like-x-early-stop).
- **Think's fallback questions are generic** ([research](../think/question-generation.md)).
  When a required part is empty and the model asked nothing, `board.rs` asks a fixed template
  such as "Who is this for?". User studies find generic, low-quality questions hurt more than
  they help. First step needs no calls: count how often the fallback fires in think records and eval logs.
- **Nothing flags an answer that contradicts an earlier one** ([research](../think/question-generation.md)).
  Multi-turn clarification degrades as conversations grow; tracking conflicting answers helped
  most. Add one eval case with a contradicting answer before changing anything.
- **Think's routing remains large.** Split `src/think/brain_dump.rs` further without
  loosening the [architecture limits](../../design/architecture.md#limits-enforced-by-testsarchitecturers).
- **Gaps across think records aren't flagged yet.** The rest of the record slice:
  the eval harness writes records with origin `eval`, and a separate script reads
  them and flags gaps.

### Context menu and send aliases (C4–C6, C9)

**Context:** Writing selection and Tab into the original shipped in issue #14
([think](../../design/think.md#interaction-contracts)). There is still no right-click menu or
modified-Enter send. Terminal hosts report modified keys and right clicks differently.

**Build:** Right-click offers Copy/Cut/Paste/Select all where meaningful, Copy/Skip
question on a question, and closes on Esc/click-away. Cmd/Ctrl+Enter sends only when
distinctly reported; F2 remains reliable and visible.

**Acceptance:** Context-specific menus pass tests at keyboard and mouse. Plain Enter
remains a newline; letters remain text. Document/test host forwarding and unsupported
shortcuts honestly; no fabricated clipboard success.

**Read:** [safeguards](#shared-safeguards), [think](../../design/think.md), [principles](../../design/principles.md), [terminal setup](../../terminal-setup.md).

**Don't:** Introduce a focus-management key maze, intercept unreported host shortcuts,
or make copy/send confirm anything.

### Evidence on demand (C13)

**Context:** Always-on highlights were removed. Exact-span checks remain, but a
person cannot ask which words support just one sentence of the reading.

**Build:** Click or keyboard-point at a reading sentence to reveal only its cited
original spans. Extend the response contract to support citations per sentence;
validate exact source offsets. Consider “words not used” only as a deliberate view.

**Acceptance:** Multiple sentences/sources and repeated Unicode quotes map to the
correct exact spans. Invalid citations fail explicitly; selecting one sentence
never lights all evidence. Clearing returns to plain original ink. Keyboard and
mouse agree; NO_COLOR cues remain. Revert exact-span validation to prove the test fails.

**Read:** [safeguards](#shared-safeguards), [think](../../design/think.md), [principles](../../design/principles.md).

**Don't:** Restore permanent green or imply unused words are rejected.

### Multiple-choice answers

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

**Read:** [safeguards](#shared-safeguards), [think](../../design/think.md), [goal block](../../design/goal-block.md), [principles](../../design/principles.md).

**Don't:** Force choices, turn guesses into facts or trade meaning for speed.

### Scratch-space spike — design only

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

**Read:** [safeguards](#shared-safeguards), [think](../../design/think.md), [principles](../../design/principles.md), [goal block](../../design/goal-block.md).

**Don't:** Implement the canvas proposal, replace dump-first, let arrangement confirm
anything, mix guesses with verbatim cards, persist canvases or invent operator dumps.

### Like X early stop

Known gap: the review can stop one question early when a line points at something
by example ("like the eval stuff we just ran") that could mean two different
sources or scopes. In a live trial it left that reading in place, and an agent would
have built from the wrong source. The "anything to add or change?" box corrects it
in one line. A candidate fix is a final check for such lines before "Nothing I'm
unsure about"; it is not built or measured yet.

## Shared

### A calm dark theme

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

**Read:** [safeguards](#shared-safeguards), [principles](../../design/principles.md), [think](../../design/think.md), [menu](../../design/menu.md).

**Don't:** Substitute generic black, add category colors or a settings maze,
or break NO_COLOR.

### Absorb substrate: keep runs and learn from them

**Context:** The operator decided on 2026-10-14 that tink-substrate's jobs fold into
Tinkery and substrate is deprecated once they are absorbed. Its Seed Me handoff
belongs to think and shape, its dashboard to watch, its retro to taste and its
archive to measure. A live tink-sdlc run that night left its only record in an
untracked worktree folder, and no step turned what it learned into anything lasting.
Watch itself is already planned as [R7](../../design/first-release-issues.md#r7);
this entry covers what R1 to R10 leave out.

**Build:** Two pieces, each its own slice. First, a run's record stays local,
outside Tinkery's history, and is kept only by an explicit operator action. Its
identity and source-revision evidence follow R2's [binding contract](../../design/job-records.md#goal-to-run-identity-prerequisite)
and [R5](../../design/first-release-issues.md#r5). Tinkery owns shared reading;
Substrate is not its permanent aggregation service. The separately approved
identity/initial-binding prerequisites precede R5, but run retention and lessons
do not. Binding revisions and relocation are deferred. A binding locates a run;
it does not archive it or keep a deleted worktree alive.
Second, after Taste, the operator may ask for one
proposed lesson from the run. Before any model call, Tinkery shows what run text
would be sent and where, and sends it only on an explicit action within a budget. It
is shown as Tinkery's guess, with the change it would make (a test or a rule), where
that change would go, and the run it came from. When an accepted lesson doesn't
prevent a repeat, Tinkery suggests removing it; only the operator removes it.

**Acceptance:** Nothing is kept, drafted or learned without a keypress that says
what it does. Missing, unreadable, outdated, failed and interrupted records stay
distinct, and none reads as current success. A proposed lesson is written only when
the operator accepts that exact change; leaving writes nothing and says what comes
next. No run text leaves the machine except by that shown, explicit send. An accepted lesson names its source run.
Reversion probes protect the keypress gate and the distinct record states.

**Evidence:** Claude Code's [agent view](https://code.claude.com/docs/en/agent-view)
puts sessions that need input or review first and links to their transcripts.
Anthropic's [agent eval guide](https://www.anthropic.com/engineering/demystifying-evals-for-ai-agents)
and Hamel Husain's [error analysis](https://hamel.dev/blog/posts/evals-faq/why-is-error-analysis-so-important-in-llm-evals-and-how-is-it-performed.html)
turn reviewed failures into tests. Research on self-written agent memory
([Reflexion](https://arxiv.org/abs/2303.11366), [ExpeL](https://arxiv.org/abs/2308.10144))
shows methods, not that unreviewed lessons are safe.

**Read:** [safeguards](#shared-safeguards), [first release](../../design/first-release.md), [the constitution](../../yohaku.md).

**Don't:** Add a multi-agent dashboard, notifications, restart controls, a hosted
trace service, or agent-written memory. Don't add features to substrate.

### Measure installation-path follow-up

Historical source: [#104](https://github.com/jon-devlapaz/tinkery/issues/104):

> Follow-up, not in scope: `src/menu.rs:150` defaults measure's substrate path to `~/.claude/skills/tink-substrate`.

Reconsider under the absorption decision before publishing anything. The line
number belongs to the original observation, not a claim about today's code.
Avoid polishing a dependency intended for retirement.

### Deferred binding changes

Binding revisions, relocation, retargeting and migration are parked beyond the
initial-only I2 slice. Review a demonstrated need and the
[binding contract](../../design/job-records.md#goal-to-run-identity-prerequisite)
before proposing independently approved work. Initial bindings are not permission
to implement these operations.
