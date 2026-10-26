# Parked ideas

This is the home for unpublished proposals, not implementation permission or the
release checklist. Published scope and status belong in issues.
[Work intake](../../../AGENTS.md#work-intake) separates publication, design acceptance
and implementation approval.

## Shared safeguards

Every draft follows [the guardrails](../../../AGENTS.md#guardrails).
Model and provider calls require an explicit budget; the default is zero. Never record
real audio. Preserve original words and answers verbatim. Leave operator data,
sessions, installs, settings and terminal, macOS and launcher preferences untouched.
The restrictions and separate data, evaluation and implementation permissions below
do not approve any proposal.

## Published records

Published scope and status belong in the [first-release issue index](../../design/first-release-issues.md)
and linked issues, not this backlog. Think exclusions are recorded in [#117](https://github.com/jon-devlapaz/tinkery/issues/117);
unsafe session-root refusal is recorded in [#118](https://github.com/jon-devlapaz/tinkery/issues/118).
Retired Measure reader work is recorded in [#119](https://github.com/jon-devlapaz/tinkery/issues/119)
and [#120](https://github.com/jon-devlapaz/tinkery/issues/120). Neither authorizes a backend migration.
Historical dogfood stays in [findings](../../../findings/README.md). Closed issue bodies and metadata stay unchanged.

## Think

### Think interaction review

The [single interaction proposal](../think/interaction-proposal.md) is awaiting
operator approval; merging its documentation does not accept a new interaction.
Read the [dogfood index](../../../findings/2026-10-22-think-dogfood.md) first for
observations, linked issues and evidence limits at the recorded revision.

### Loose ends

- **Separate goals:** only labels survive; checks and rules stay in originals.
  Attaching them needs a design choice.
- **Dropped wire fields:** `Guess.outcome` duplicates done-when. The board ignores
  composed framing and uses parts and supports (`src/think/brain_dump/reading.rs`, `board.rs`).

### Context menu and send aliases (C4–C6, C9)

**Build:** Right-click offers Copy, Cut, Paste and Select all where meaningful,
or Copy and Skip on a question. Esc or clicking away closes it. Cmd+Enter or
Ctrl+Enter sends only if distinctly reported; F2 stays visible and reliable.
Host forwarding varies.

**Acceptance:** Test context menus with keyboard and mouse. Plain Enter remains a
newline; letters remain text. Document and test host forwarding and unsupported
shortcuts honestly. Never fabricate clipboard success.

**Read:** [safeguards](#shared-safeguards), [think](../../design/think.md), [principles](../../design/principles.md), [terminal setup](../../terminal-setup.md).

**Don't:** Add focus-key mazes, intercept unreported shortcuts,
or make copying or sending confirm anything.

### Evidence on demand (C13)

**Build:** Point at or click one reading sentence to reveal only its exact cited
original spans. Extend citations per sentence and validate source offsets.
“Words not used” requires deliberate viewing.

**Acceptance:** Multiple sources, sentences and repeated Unicode quotes map exactly.
Invalid citations fail explicitly. Never light all evidence for one sentence.
Clearing restores plain ink. Keyboard and mouse agree, with NO_COLOR cues intact.
Revert exact-span validation to prove failure.

**Read:** [safeguards](#shared-safeguards), [think](../../design/think.md), [principles](../../design/principles.md).

**Don't:** Restore permanent green or imply unused words are rejected.

**Issue:** [#428](https://github.com/jon-devlapaz/tinkery/issues/428) tracks the bounded implementation proposal; it is not approved for implementation.

## Shared

### A calm dark theme

**Build:** Propose a restrained dark palette for operator review. Follow macOS
appearance changes automatically and use an honest fallback elsewhere; unavailable
appearance stays unknown. Jade means location, rust means consequence, and NO_COLOR works.

**Acceptance:** Review light and dark mockups. Measure text contrast of at least
4.5:1 between foreground and background, including muted, jade and rust text.
Use fake appearance-switching tests, snapshots and PTYs for selection,
consequence prompts and NO_COLOR. Preserve input, navigation and the confirmed goal.

**Read:** [safeguards](#shared-safeguards), [principles](../../design/principles.md), [think](../../design/think.md), [menu](../../design/menu.md).

**Don't:** Use generic black, category colours, settings mazes or break NO_COLOR.

### Choose the build harness

Parked from [#423](https://github.com/jon-devlapaz/tinkery/issues/423) on 2026-10-26.
The operator wants to pick Pi or Claude Code as the build harness in Tinkery.

**Build:** Wait for [tink-sdlc #110](https://github.com/jon-devlapaz/tink-sdlc/issues/110)
to show that a Claude Code launcher can meet protocol 1. Until then, choosing a harness
means passing another `--launcher` to tink-sdlc's `step build prepare`. A setting needs a
consumer first: Approve copies only intake and stage 1, and Watch is read-only and repeats
tink-sdlc's own next action. That command also needs a report path, a model pin and the
stage-3 checkout, which API 1 doesn't expose.

**Acceptance:** One saved choice, stored with [#422](https://github.com/jon-devlapaz/tinkery/issues/422)'s
saved setup under `~/.config/tinkery/`, beside voice's `voice.json`. Picking or copying
makes no provider calls and launches nothing.

**Read:** [safeguards](#shared-safeguards), [menu](../../design/menu.md), [watch](../../design/watch.md).

**Don't:** Grant a call budget, pick the operator's harness, add a second
Think/Shape model host or build a settings maze.

### Keep runs and learn from them

The external aggregation tool is retired, not awaiting absorption.
[Measure's contract](../../design/menu.md#measure) owns its explicit repo PR reads;
run retention and lessons are separate.
[R7](../../design/first-release-issues.md#r7) owns Watch.

**Build:** Keep these as separate slices:

1. Explicitly retain local run records outside Tinkery history. Follow the identity
   and revision rules before proposing an implementation. The earlier Tinkery
   binding path was replaced by [#258](https://github.com/jon-devlapaz/tinkery/issues/258);
   this retention proposal has no accepted record contract. Use source-owned records,
   not the retired aggregation tool. Revisions and relocation stay deferred. A
   binding doesn't archive a deleted worktree.
2. After Taste, the operator may request one guessed lesson. Show the proposed test
   or rule, its destination and source run. Before calling, show the text and
   destination and require an explicit budgeted send. If accepted lessons fail to
   prevent recurrence, suggest removal; only the operator removes them.

**Acceptance:** Retention, drafting and learning each require a labelled keypress.
Distinguish missing, unreadable, outdated, failed and interrupted records from
current success. Write lessons only on acceptance of that exact change. Leaving
writes nothing and names the next step. Send no run text except through the
shown explicit send. Accepted lessons name their source run. Reversion probes
protect the keypress gate and distinct states. Research methods do not establish
that unreviewed memory is safe.

**Read:** [safeguards](#shared-safeguards), [first release](../../design/first-release.md), [the constitution](../../yohaku.md).

**Don't:** Add multi-agent dashboards, notifications, restart controls, hosted traces
or agent-written memory. Don't restore the retired aggregation dependency.

### Deferred binding changes

Revisions, relocation, retargeting and migration need a demonstrated need,
review against the unaccepted binding proposal and independently approved work. The
initial binding proposal was replaced by [#258](https://github.com/jon-devlapaz/tinkery/issues/258);
that work does not authorize these changes.
