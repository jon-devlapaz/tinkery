# Parked ideas

One home for unpublished proposals, not implementation permission or the release
checklist. Published scope/status belongs in issues; [work intake](../../../AGENTS.md#work-intake)
separates publication, design acceptance and implementation approval.

## Shared safeguards

Work on every draft follows [the guardrails](../../../AGENTS.md#guardrails).
Model/provider calls require explicit budget; the default is zero. Never record
real audio. Preserve original words and answers verbatim. Leave operator data,
sessions, installs, settings and terminal/macOS/launcher preferences untouched.
Proposal-specific restrictions and separate data, evaluation and implementation
permissions remain below; these safeguards do not approve any proposal.

## Published from this inventory

- [Release tasks R4–R10/I2](../../design/first-release-issues.md).
- Think exclusions [#117](https://github.com/jon-devlapaz/tinkery/issues/117);
  unsafe session-root refusal [#118](https://github.com/jon-devlapaz/tinkery/issues/118).
- Measure failure [#119](https://github.com/jon-devlapaz/tinkery/issues/119) and
  responsiveness [#120](https://github.com/jon-devlapaz/tinkery/issues/120).
  Neither authorizes a backend migration.
- Closed Shape proposal [#106](https://github.com/jon-devlapaz/tinkery/issues/106);
  historical dogfood [findings](../../../findings/README.md). Closed issue bodies/
  metadata stay unchanged.
- I1 identity/R6 expected-input enforcement are conditional upstream checks, not
  presumed missing capabilities or newly published tasks.

## Think

### Loose ends

- **Large drafts:** full-buffer rewrap/position scans may lag. Combined debug
  insertion/wrap/motion/selection fixture: 224–240 ms, ~131 KB, 65,534–65,536 rows,
  not per-keystroke latency. Measure edit/render separately before cache design;
  caching isn't paste reliability.
- **Approach prefixes:** `PROVISIONAL assumption:` conflicts with reading.txt's
  no-template-prefix rule; needs a budgeted prompt check.
- **Separate goals:** only labels survive; checks/rules stay in originals. Attaching
  them needs a design choice.
- **Dropped wire fields:** `Guess.outcome` duplicates done-when; board ignores
  composed framing, uses parts/supports (`src/think/brain_dump/reading.rs`, `board.rs`).
- [“Like X” early stop](#like-x-early-stop) can leave ambiguous source/scope.
- **Generic fallback questions:** [research](../think/question-generation.md).
  First count fixed-template fallbacks in records/eval logs, no calls needed.
- **Contradicting answers:** same research; add one eval case before changes.
- **Large routing:** split `src/think/brain_dump.rs` without loosening
  [architecture limits](../../design/architecture.md#limits-enforced-by-testsarchitecturers).
- **Record gaps:** separate eval-origin records and gap-flagging script remain.

### Context menu and send aliases (C4–C6, C9)

**Build:** Right-click Copy/Cut/Paste/Select all where meaningful; Copy/Skip on a
question; Esc/click-away closes. Cmd/Ctrl+Enter sends only if distinctly reported;
F2 stays visible/reliable. Selection/Tab already exist; host forwarding varies.

**Acceptance:** Keyboard/mouse context tests; plain Enter newline, letters text;
document/test host forwarding and unsupported shortcuts honestly; no fabricated
clipboard success.

**Read:** [safeguards](#shared-safeguards), [think](../../design/think.md), [principles](../../design/principles.md), [terminal setup](../../terminal-setup.md).

**Don't:** Add focus-key mazes, intercept unreported shortcuts,
or make copy/send confirm anything.

### Evidence on demand (C13)

**Build:** Point/click one reading sentence to reveal only its exact cited original
spans. Extend citations per sentence; validate source offsets. “Words not used”
requires deliberate viewing.

**Acceptance:** Multiple sources/sentences/repeated Unicode quotes map exactly;
invalid citations fail explicitly. Never light all evidence for one sentence.
Clearing restores plain ink; keyboard/mouse and NO_COLOR agree. Revert exact-span
validation to prove failure.

**Read:** [safeguards](#shared-safeguards), [think](../../design/think.md), [principles](../../design/principles.md).

**Don't:** Restore permanent green or imply unused words are rejected.

### Multiple-choice answers

**Build:** Optional 2–4 visibly guessed answers plus free text for choice-shaped questions;
no options for open ones. No preselection. Record `chose: <option>` as operator
selection, never independently observed fact.

**Acceptance:** Keyboard/mouse/free text refine; no default/accidental send or
open-question clutter. Provenance survives review/handoff; choice confirms no
goal/seed. Fakes prove mechanics; evaluate fixation with the operator under
separate budget/dump-set approval.

**Read:** [safeguards](#shared-safeguards), [think](../../design/think.md), [goal block](../../design/goal-block.md), [principles](../../design/principles.md).

**Don't:** Force choices, turn guesses into facts or trade meaning for speed.

### Scratch-space spike — design only

**Produce:** Research spatial sensemaking/deliberate grouping; discuss usefulness,
authority/exactness with the operator first. Mock up 100×30 after-dump space: quoted
spans, unused words, question/notes; entering, grouping, distinct agent suggestion,
returning to the same place.
Decide whether arrangement is evidence (recommended), grouping meaning and how
editing creates sourced operator notes. Rearranging alone costs no call.

**Acceptance:** Report where this beats another typed line; answer design questions
and recommend build/shrink/drop with reasons. Dogfood only operator-supplied dumps
with explicit budget.
No production changes; later build needs separate approval. Hidden extracted cards
aren't this proposal; stickies must not replace dump-first.

**Read:** [safeguards](#shared-safeguards), [think](../../design/think.md), [principles](../../design/principles.md), [goal block](../../design/goal-block.md).

**Don't:** Build canvas, replace dump-first, let arrangement confirm,
mix guesses with verbatim cards, persist canvases or invent operator dumps.

### Like X early stop

Ambiguous examples (“like the eval stuff we just ran”) can stop review before
source/scope is settled and send agents to the wrong source. Added words can
correct it. Proposed final ambiguity check before “Nothing I'm unsure about”
is neither built nor measured.

## Shared

### A calm dark theme

**Build:** Operator-reviewed restrained dark palette, automatic macOS appearance
changes, honest non-macOS fallback; unavailable appearance stays unknown.
Jade remains location, rust consequence;
NO_COLOR works.

**Acceptance:** Review light/dark mockups; measure ≥4.5:1 foreground/background
text contrast including muted/jade/rust. Fake appearance switching and snapshot/PTY
selection/consequence/NO_COLOR tests. Preserve input/navigation/confirmed goal.

**Read:** [safeguards](#shared-safeguards), [principles](../../design/principles.md), [think](../../design/think.md), [menu](../../design/menu.md).

**Don't:** Use generic black, category colours, settings mazes or break NO_COLOR.

### Absorb substrate: keep runs and learn from them

Substrate responsibilities fold into Tinkery: handoff → Think/Shape, dashboard →
Watch, retro → Taste, archive → Measure. Deprecate Substrate only after absorption;
[R7](../../design/first-release-issues.md#r7) already owns Watch.

**Build:** Separate slices:

1. Explicitly retain local run records outside Tinkery history. Follow
   [binding](../../design/job-records.md#goal-to-run-identity-prerequisite)/[R5](../../design/first-release-issues.md#r5)
   identity/revision evidence. Tinkery owns shared reading, not a permanent Substrate
   client. Separately approved identity/initial binding precede R5; retention/lessons
   do not. Revisions/relocation stay deferred; binding doesn't archive deleted worktrees.
2. After Taste, operator may request one guessed lesson, showing proposed test/rule,
   destination and source run. Before calling, show text/destination and require
   explicit budgeted send. If accepted lessons fail to prevent recurrence, suggest
   removal; only operator removes.

**Acceptance:** Labelled keypress for retention/drafting/learning. Distinguish missing,
unreadable, outdated, failed and interrupted from current success. Write lessons
only on acceptance of that exact change; leaving writes nothing and names next step.
Send no run text except displayed explicit send. Accepted lessons name source run.
Reversion probes protect keypress gate/distinct states. Research methods do not
establish unreviewed-memory safety.

**Read:** [safeguards](#shared-safeguards), [first release](../../design/first-release.md), [the constitution](../../yohaku.md).

**Don't:** Add multi-agent dashboards, notifications, restart controls, hosted traces
or agent-written memory. Don't add features to substrate.

### Measure installation-path follow-up

[#104](https://github.com/jon-devlapaz/tinkery/issues/104) observed Measure's historical
`~/.claude/skills/tink-substrate` default. Reconsider under absorption before
publishing; don't polish a retiring dependency or treat old line numbers as current.

### Deferred binding changes

Revisions, relocation, retargeting and migration need demonstrated need,
[binding-contract](../../design/job-records.md#goal-to-run-identity-prerequisite)
review and independently approved work; initial I2 authorizes none.
