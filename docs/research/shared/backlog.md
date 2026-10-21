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

## Published from this inventory

- [Release tasks R4–R10 and I2](../../design/first-release-issues.md) hold their specifications.
- Think exclusions are in [#117](https://github.com/jon-devlapaz/tinkery/issues/117);
  unsafe session-root refusal is in [#118](https://github.com/jon-devlapaz/tinkery/issues/118).
- Retired Measure reader history is in [#119](https://github.com/jon-devlapaz/tinkery/issues/119)
  and [#120](https://github.com/jon-devlapaz/tinkery/issues/120).
  Neither authorizes a backend migration.
- Historical dogfood stays in [findings](../../../findings/README.md).
  Closed issue bodies and metadata stay unchanged.
- I1 identity and R6 expected-input enforcement are conditional upstream checks,
  not presumed missing capabilities or newly published tasks.

## Think

### Loose ends

- **Large drafts:** full-buffer wrapping and position scans may lag. Measure editing
  and rendering separately before cache design; the combined fixture is not
  per-keystroke evidence. Caching isn't paste reliability.
- **Approach prefixes:** `PROVISIONAL assumption:` conflicts with reading.txt's
  no-template-prefix rule. It needs a budgeted prompt check.
- **Separate goals:** only labels survive; checks and rules stay in originals.
  Attaching them needs a design choice.
- **Dropped wire fields:** `Guess.outcome` duplicates done-when. The board ignores
  composed framing and uses parts and supports (`src/think/brain_dump/reading.rs`, `board.rs`).
- [“Like X” early stop](#like-x-early-stop) can leave the source or scope ambiguous.
- **Generic fallback questions:** first count fixed-template fallbacks in records
  and eval logs, without calls. See [research](../think/question-generation.md).
- **Contradicting answers:** the same research suggests adding one eval case before changes.
- **Large routing:** split `src/think/brain_dump.rs` without loosening
  [architecture limits](../../design/architecture.md#limits-enforced-by-testsarchitecturers).
- **Record gaps:** eval-origin records and a separate gap-flagging script remain to build.

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

### Multiple-choice answers

**Build:** Offer an optional 2–4 visibly guessed answers plus free text for
choice-shaped questions, but no options for open ones. Never preselect.
Record `chose: <option>` as the operator's selection, not an independently observed fact.

**Acceptance:** Keyboard choices, mouse choices and free text refine the reading.
There is no default or accidental send, and open questions stay uncluttered.
Provenance survives review and handoff; choices confirm no goal or seed.
Fakes prove mechanics. Evaluate fixation with the operator under a separately
approved budget and dump set.

**Read:** [safeguards](#shared-safeguards), [think](../../design/think.md), [goal block](../../design/goal-block.md), [principles](../../design/principles.md).

**Don't:** Force choices, turn guesses into facts or trade meaning for speed.

### Scratch-space spike — design only

**Produce:** Research spatial sensemaking and deliberate grouping. Discuss usefulness,
authority and exactness with the operator first. Mock up a 100×30 after-dump space
with quoted spans, unused words, the question and notes. Show entry, grouping,
a distinct agent suggestion and return to the same place. Decide whether arrangement
is evidence (recommended), what grouping means and how editing creates sourced
operator notes. Rearranging alone costs no call.

**Acceptance:** Report where this beats another typed line. Answer the design
questions and recommend building, shrinking or dropping it, with reasons.
Dogfood uses only operator-supplied dumps with an explicit budget.
Make no production changes; a later build needs separate approval. Hidden extracted
cards aren't this proposal; stickies must not replace dump-first.

**Read:** [safeguards](#shared-safeguards), [think](../../design/think.md), [principles](../../design/principles.md), [goal block](../../design/goal-block.md).

**Don't:** Build the canvas, replace dump-first, let arrangement confirm anything,
mix guesses with verbatim cards, persist canvases or invent operator dumps.

### Like X early stop

Ambiguous examples (“like the eval stuff we just ran”) can stop review before the
source or scope is settled and send agents to the wrong source. Added words can
correct it. The proposed final ambiguity check before “Nothing I'm unsure about”
is neither built nor measured.

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

### Keep runs and learn from them

The external aggregation tool is retired, not awaiting absorption. Measure reads
no source; [its contract](../../design/menu.md#measure) owns current behavior.
[R7](../../design/first-release-issues.md#r7) owns Watch.

**Build:** Keep these as separate slices:

1. Explicitly retain local run records outside Tinkery history. Follow the identity
   and revision evidence in the [binding contract](../../design/job-records.md#goal-to-run-identity-prerequisite)
   and [R5](../../design/first-release-issues.md#r5). Use source-owned records,
   not the retired aggregation tool. Separately approved identity and initial binding
   precede R5; retention and lessons do not. Revisions and relocation stay deferred.
   A binding doesn't archive a deleted worktree.
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
[binding-contract](../../design/job-records.md#goal-to-run-identity-prerequisite)
review and independently approved work. Initial I2 authorizes none.
