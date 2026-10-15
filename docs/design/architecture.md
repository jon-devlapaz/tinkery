# Architecture

Read [the constitution](../yohaku.md) first. This page owns module responsibilities,
dependency rules and the design score. [Architecture tests](../../tests/architecture.rs)
enforce the limits below; a limit may only get stricter.

## Phases and the dependency rule

Each built [place](menu.md) is a **phase**, with its own compartment:

- A phase uses only shared code, never another phase.
- Only the menu knows every phase.
- Shared code names no phase.

Each top-level module belongs to one area, listed in `MODULES` in the check. A new module must be listed
there, so a new phase can't count as shared code by default.

Published task specifications live in GitHub Issues. The
[parked backlog](../research/shared/backlog.md) is the single home for unpublished
ideas; the area entry points below contain navigation only.

| Area | Code | Work entry point |
| --- | --- | --- |
| menu (also lists approve, watch and taste, which aren't built) | `src/menu.rs` | [#118](https://github.com/jon-devlapaz/tinkery/issues/118), still needs approval |
| think | `src/think/` | [think](../research/shared/backlog.md#think) |
| tend | `src/tend/` | [#123](https://github.com/jon-devlapaz/tinkery/issues/123) |
| shape | `src/shape.rs` | [#149](https://github.com/jon-devlapaz/tinkery/issues/149) built; [#150](https://github.com/jon-devlapaz/tinkery/issues/150) next |
| measure | `src/measure.rs` | [#119](https://github.com/jon-devlapaz/tinkery/issues/119) failure presentation; [#120](https://github.com/jon-devlapaz/tinkery/issues/120) owned responsive reader |
| shared | `src/lib.rs` (palette, sizes), `src/goal/`, `src/seed_me/`, `src/think_record.rs`, `src/voice/` | [shared](../research/shared/backlog.md) |

Shape, the Seed Me step, is in the first release (operator decision, 2026-10-15) and has its
own module; see [Shape](shape-and-handoff.md#shape). Approve, watch and taste also get their
own modules when built. There are no exceptions
to the dependency rule: think hands a saved goal to tend only through `seed_me`.

## Factory boundaries

The operator-supplied [factory principles](https://x.com/zachlloydtweets/status/2110505938604134737)
are context for these boundaries, not new requirements or implementation permission:

- **Factory as code:** Tinkery versions its prompts, model-call boundary, tests and
  CI. External SDLC owns implementation orchestration; Tinkery does not yet
  configure or verify the whole factory's permissions, automations and sandboxes.
- **Models and harnesses:** Think uses an explicitly selected provider/model
  through Pi. Continuing in Claude, Codex or Pi is a
  [manual handoff](shape-and-handoff.md#continuation-contract), not interchangeable
  execution inside Tinkery. Model choice does not establish open-weight model
  compatibility or comparative quality.
- **Owned evidence:** [Think records](think.md#think-record) keep confirmed-session
  words, turns, checks, model identity, prompt fingerprint and the exact goal in
  local JSON. Unconfirmed sessions leave no record; this is not complete factory
  telemetry. Connecting a goal to the exact external run and its evidence remains
  an [unaccepted design](job-records.md#goal-to-run-identity-prerequisite).

The [release plan](first-release.md) owns the remaining journey and its evidence
requirements. A complete factory and improved outcomes are not shipped claims.

## Modules today

| Module | Owns |
| --- | --- |
| `main.rs` | Command-line options and the terminal loop; wires the menu or think to a real or practice host. |
| `lib.rs` | The palette (one job per colour) and the minimum terminal size. |
| `menu.rs` | The menu: the seven places and navigation between them, including Esc from Shape back to the Tend it came from. |
| `measure.rs` | The ledger's read-only report, display safety limits, failure facts and safe two-layer disclosure. |
| `measure/reader.rs` | One reader across visits: request ownership, deadline, bounded pipes, process-group cancellation and cleanup-first shutdown. |
| `measure/viewport.rs` | Incremental machine-sized diagnostic scrolling over bounded, grapheme-safe source segments; no subprocess or navigation policy. |
| `measure/plain.rs` | Prepared success-report glyph/row offsets; visible-only clipping equivalent to the original Paragraph, including invisible runs. |
| `tend/mod.rs` | The saved-goal garden's discovery states, selection, detail and copying the continuation prompt again. |
| `tend/history.rs` | One owned history read at a time and a read-only conversation view with separate technical help. |
| `tend/linking.rs` | Explicit source/pick state, link refusals and derived needs wording, independent of garden drawing. |
| `tend/garden.rs` | The graph's framing: selected-line needs, read notes and explicit refusals. |
| `tend/graph.rs` | Graf adapter: path-keyed topology, deterministic layout, palette, labels, viewport and pointer geometry; no physics worker or writes. |
| `tend/discovery.rs` | Scanning for sessions off the paint path, with unknown, complete, incomplete and failed kept apart. |
| `tend::needs` | Tinkery-owned needs links, strict atomic persistence and readiness derived from readable session identities; never writes Seed Me files. |
| `seed_me/` | Every fact about Seed Me: saving a confirmed goal through its helper, reading a session's files and stage, the continuation prompt, and (in `shaping.rs`) where a seed stands from the helper's `read` and `status` (shared). |
| `shape.rs` | Shape's list of goals waiting for a seed and one session's read-only view: the goal as confirmed, one stage and one next step; reads off the paint path. |
| `goal/mod.rs` | Goal parts, constraint normalisation and composing the goal block (shared). |
| `goal/confirmed.rs` | Rendering a confirmed goal the same way in review, receipt and tend detail (shared). |
| `think/brain_dump.rs` | Think's `BrainDump`, which routes keys, mouse and ticks to its four parts, paints them, and reports one `Exit` to the menu. |
| `think/brain_dump/conversation.rs` | The conversation, turn by turn: words, extracts, reading, the rules for questions, skips, scope and the next request, and the ordered turns the think record keeps. Fields are private; nothing about the screen or jobs. |
| `think/brain_dump/reading.rs` | The reading's contract with the model: the request, the prompt, the reply's wire shape (including open choices linked to questions) and its lenient decoder. |
| `think/brain_dump/review.rs` | The decision review: its prompt, what it sees, and how its decisions join the reading's. |
| `think/brain_dump/prompts/` | The two prompts as text, one rule per line, with a note naming the finding behind each tuned rule. Notes never reach the model. |
| `think/brain_dump/host.rs` | Who answers a reading request: Pi (a shaping call, then the review) or the practice simulation; a stateful host starts a separate history for each Think visit. |
| `think/brain_dump/board.rs` | Verifying a decoded reading; a `Board` only exists once verified. |
| `think/brain_dump/checks.rs` | The per-send record shown in F5 "why". |
| `think/brain_dump/goal_view.rs`, `intact_view.rs`, `voice_input.rs`, `layout.rs` | Think's review screen, original pane, dictation input, and where everything goes on screen with help words, key bar and notices. |
| `think/brain_dump/help_view.rs` | Think's bounded help overlay: visual hierarchy, scrolling body, fixed navigation and the boundary against hidden actions. |
| `think/drafting.rs` | The Pi call boundary: one bounded, cancellable request, optionally limited by a call budget. |
| `think_record.rs`, `think_record/read.rs` | The shared confirmed-session format, its one write, and exact-session history attribution; no phase behavior. |
| `think/canvas.rs`, `note.rs` | Extracted cards, and the raw text box with safe display, grapheme-offset mapping and machine-sized wrapped rows. Editor storage does not own send policy. |
| `voice/` | Local dictation: the host boundary, helper process, protocol and owned setup. |

Vendored libraries are path dependencies, not phases: `vendor/pinstar/` owns
rendering/input primitives and `vendor/graf/` owns graph mechanics. Tend reaches graf
only through `tend/graph.rs`; linking and persistence do not depend on it. Graf knows
no Tinkery phase or Seed Me format. Each library's `UPSTREAM` records its source and
local edits; [Pinstar API](../../vendor/pinstar/API.md) and
[Graf API](../../vendor/graf/API.md) describe their boundaries.

## Design score

The yardstick is the eight-question diagnostic from John Ousterhout's *A Philosophy of Software Design*: the score
is how many pass, times 1.25. Each PR that changes structure says which rows it moves.

| # | Question | Recorded assessment |
| --- | --- | --- |
| 1 | Can each module be described in one sentence? | Yes: see the module map; the reading's types, prompt and decoder live in `reading.rs` (#38) |
| 2 | Are interfaces simpler than implementations? | Yes |
| 3 | Can an implementation change without touching callers? | Yes: each model contract lives beside its decoder; tests check their field names agree. Seed Me's format lives only in `seed_me/`. The review decodes only its own reply; the reading's decoder joins it (#89) |
| 4 | Do interface comments describe the abstraction? | Yes: modules and `pub` items carry interface comments; the build denies `missing_docs` and `unreachable_pub` (#39) |
| 5 | Is design discussion part of review? | Yes: [the working guide](../../AGENTS.md#guardrails) requires a changeability check and names the rows moved (#39) |
| 6 | Does each module hide an important decision? | Yes: `Conversation` hides question, skip and scope rules behind methods |
| 7 | Can a newcomer see the boundaries without reading code? | Yes: this page |
| 8 | Is design improvement part of ordinary work? | Yes |

**Recorded score: 10/10** (2026-10-12, after #39; first scored at 2d25a7c, 5/10).
This is a dated assessment, not a fresh measurement. Remaining debt:
`brain_dump.rs`'s narrow interface over a large amount of routing.
Paste reliability improves diagnostics 3 and 6 by keeping raw/display mapping in
Note and using conversation staging for both encoded-size preview and sending.
It preserves the simple editor interface and documented promises in diagnostics
2 and 4. Cached preview size stays inside Think's input/reply routing, preserving
those boundaries; the request boundary now has one definition in drafting (3 and 6).
No new score is claimed.
On 2026-10-14, [#123](https://github.com/jon-devlapaz/tinkery/issues/123) keeps one
module per phase and holds diagnostics 1 and 7: each module retains a one-sentence
job and the map shows the garden and future Seed Me boundary.

For #119, Measure owns failure classification, diagnostic escaping and its reading/help
state; the menu only routes navigation. This preserves diagnostics 3 and 6 by keeping
ledger error interpretation out of callers, and 1 and 7 by retaining one phase boundary.
The diagnostic viewport hides Ratatui's local scroll limit and reflow cache within
Measure (3 and 6); callers still see one reading. It does not change subprocess
ownership or claim a new design score.

For #127, Tend owns the link format, refusals and derived readiness; Seed Me alone
interprets ledger identity (3 and 6). Linking state stays separate from graph drawing.
The graf adapter hides topology, static layout and pointer geometry; garden framing
keeps the footer independent of that renderer (2, 3 and 6). The module map and public
contracts preserve diagnostics 1, 4 and 7. No new design score is claimed.

For #149, Shape is its own phase; Tend reaches it only through the menu, which keeps the
Tend view to restore (diagnostics 1, 3 and 7). Every Seed Me helper field Shape needs is read
in `seed_me/shaping.rs`, so a Seed Me format change stays in one module (3 and 6). No new
score is claimed.

For #143, the confirmed-session format becomes shared so Think writes and Tend
reads through one owner (diagnostics 1, 3 and 6). The menu retains one history
reader across visits; Tend owns its view. The map exposes these boundaries (7);
no phase reaches into another and no new score is claimed.

For #120, Measure's reader hides process ownership, limits and cancellation across visits
(3 and 6); the menu routes a small open/cancel/poll/shutdown interface (2). The terminal
loop restores the terminal before draining Measure on normal exit or I/O error. Bounded,
incremental viewport indexing and prepared shared display text avoid rebuilding all
output on a frame. The module map and interface promises preserve 1, 4 and 7. No shared
reader framework or new design score is claimed.

## Limits enforced by `tests/architecture.rs`

| Limit | Today | Lowered by |
| --- | --- | --- |
| Phase reaches across phases | none allowed | done in #36 |
| `BrainDump` fields / `pub` fields | 13 / 0 | done in #37 |
| Non-test source lines over 200 characters | 13 | done in #38 (the prompts moved to text files) |
| Library files without a `//!` comment, `pub` items without a doc comment | none allowed | done in #39 |

A check fails when the code gets worse than its limit, and also when it gets better, so the PR that improves it
lowers the limit too.
