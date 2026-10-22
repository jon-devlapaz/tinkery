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

Use the [Think backlog](../research/shared/backlog.md#think) for unpublished Think
work. Measure's former [failure presentation](https://github.com/jon-devlapaz/tinkery/issues/119)
and [owned reader](https://github.com/jon-devlapaz/tinkery/issues/120) are retired.
The module map below describes responsibilities; `MODULES` owns area assignments.

[Shape](shape-and-handoff.md#shape), [Approve](approve.md) and [Watch](watch.md) have separate modules. Taste will
get its own module when built. There are no dependency exceptions: Think hands
saved goals to Tend only through `seed_me`. The menu alone connects Shape and Tend.

## Factory boundaries

[Factory principles](https://x.com/zachlloydtweets/status/2110505938604134737) are
context, not requirements or permission. Tinkery versions prompts, model boundaries,
tests and CI; external SDLC owns implementation orchestration. Tinkery neither
configures nor verifies whole-factory permissions, automations or sandboxes.

Pi uses an explicitly selected provider and model. [Manual continuation](shape-and-handoff.md#continuation-contract)
in Claude, Codex or Pi is not interchangeable execution inside Tinkery. Model
selection establishes neither open-weight compatibility nor comparative quality.

[Think records](think.md#think-record) retain confirmed-session evidence, not
unconfirmed sessions or complete factory telemetry. The read-only
[seed-to-run join](shape-and-handoff.md#sdlc-handoff) uses SDLC's source-owned record;
the broader [binding-store proposal](job-records.md#goal-to-run-identity-prerequisite)
is not implemented.
[Release evidence](first-release.md) is required before claiming a complete factory
or improved outcomes.

## Modules today

| Module | Owns |
| --- | --- |
| `main.rs` | Command-line options and the terminal loop; wires the menu or think to a real or practice host. |
| `lib.rs` | The palette (one job per colour) and the minimum terminal size. |
| `pi.rs` | The one place Pi starts: a bounded, cancellable request with or without read-only tools, the call budget and prompt-file assembly (shared). |
| `repo.rs` | Finding the Git repository Tinkery was started in, by looking for `.git` only (shared). |
| `menu.rs` | The seven places and navigation, including Esc from Shape back to its original Tend view. |
| `measure.rs` | Recent PR list, details and evidence limits; explicit reads only, no writes or providers. |
| `measure/read.rs` | Bounded, cancellable `gh` reads scoped to the launch repo; no shell or retries. |
| `tend/mod.rs` | The saved-goal garden's discovery states, selection, detail and copying the continuation prompt again. |
| `tend/history.rs` | One owned history read at a time and a read-only conversation view with separate technical help. |
| `tend/linking.rs` | The source and target selection state, link refusals and derived needs wording, independent of garden drawing. |
| `tend/garden.rs` | The graph's framing: selected-line needs, read notes and explicit refusals. |
| `tend/graph.rs` | Graf adapter: path-keyed topology, deterministic layout, palette, labels, viewport and pointer geometry; no physics worker or writes. |
| `tend/discovery.rs` | Scanning for sessions off the paint path, with unknown, complete, incomplete and failed kept apart. |
| `tend::needs` | Tinkery-owned needs links, strict atomic persistence and readiness derived from readable session identities; never writes session files. |
| `seed_me/` | Session formats, goal saving and the one-time root migration; `answering.rs` adapts Shape actions, typed facts and operator review reasons to the shared ledger without duplicating its fact rules. Goal initialization uses the Rust ledger; exit receipts retain data until printing, when the shared verifier supplies the current next step. Contract guidance is bundled, and copied intake commands name this Tinkery executable (shared). |
| `seed_me/ledger` | Validation, transitions, explicit answer actions and cascading reopen, confirmation risks and completion-review coverage, byte-bound teach-back records, seed decisions and checks, confirmation receipts, verification and ending; shared writer lock and atomic saves. No phase policy, viewer or network connection. |
| `seed_me/shaping.rs` | Classifying ledger-owned status and review flags for display, with optional Tinkery-executable intake text; no subprocess or mutation. |
| `approve.rs` | One verified seed at a time, read-only commands and a retained verifier; no approval or launch. |
| `seed_me/ready.rs` | Consistent verified goal/acceptance snapshots and the shared intake commands; no mutation. |
| `shape.rs` | Goals awaiting seeds; each session's confirmed goal, stage, folder, words and next step. Reads and steps stay off the paint path. |
| `shape/forms.rs` | Unsaved deferral and replacement-answer fields, navigation and cancellation; no validation or publication policy. |
| `shape/words.rs` | Shape's in-memory textarea adapter: editing, selection, cursor-following and original line-ending provenance; no save or agent policy. |
| `shape/step.rs`, `shape/prompt.txt` | The step's budget check, folder, model input, prompt, reply decoding and recording through the shared session boundary. Replies contain findings with a question or named seed prose fields; `seed_me` alone owns their layout and validation. |
| `goal/mod.rs` | Goal parts, constraint normalisation and composing the goal block (shared). |
| `goal/confirmed.rs` | Rendering a confirmed goal the same way in review, receipt and tend detail (shared). |
| `think/brain_dump.rs` | Think's `BrainDump`, which routes keys, mouse and ticks to its four parts, paints them, and reports one `Exit` to the menu. |
| `think/brain_dump/conversation.rs` | Words, extracts, readings, question rules, skips, scope, unresolved review questions and notes, requests and ordered record turns. Fields are private; methods hide these rules. It knows nothing about screens or jobs. |
| `think/brain_dump/reading.rs` | The model request, prompt, wire reply and lenient decoder, including open choices linked to questions. |
| `think/brain_dump/review.rs` | The decision review: its prompt, what it sees, and how its decisions join the reading's. |
| `think/brain_dump/prompts/` | The two prompts as text, one rule per line, with a note naming the finding behind each tuned rule. Notes never reach the model. |
| `think/brain_dump/host.rs` | Pi shaping then review, or practice simulation. Stateful hosts start separate histories for each Think visit. |
| `think/brain_dump/board.rs` | A verified reading; `Board` exists only after verification. |
| `think/brain_dump/checks.rs` | The per-send record shown in F5 "why". |
| `think/brain_dump/goal_view.rs`, `intact_view.rs`, `voice_input.rs`, `layout.rs` | The review screen, original pane, dictation and screen layout, including help, key bar and notices. |
| `think/brain_dump/help_view.rs` | Think's bounded help overlay: visual hierarchy, scrolling body, fixed navigation and the boundary against hidden actions. |
| `think/drafting.rs` | Think's Pi host: its model settings and bundled working-draft instructions, over the shared `pi.rs` boundary. |
| `think_record.rs`, `think_record/confirmation.rs`, `think_record/read.rs` | Confirmed-session format, caller-declared context, the shared authorization-to-actor boundary and exact-session history attribution; no phase behavior. |
| `think/canvas.rs`, `note.rs` | Extracted cards, and the raw text box with safe display, grapheme-offset mapping and machine-sized wrapped rows. Editor storage does not own send policy. |
| `voice/` | Local dictation: the host boundary, helper process, protocol and owned setup. |
| `watch.rs` | The runs in one named tink-sdlc checkout from its API 1 `status --json`: refusing other interfaces, waiting runs first, one run's detail. |
| `sdlc.rs`, `sdlc/run.rs` | Shared read-only API 1 status, exact original-seed matching and bounded process ownership; no store, checkout discovery or phase policy. |

Vendored libraries are path dependencies, not phases: `vendor/pinstar/` owns
rendering and input primitives and `vendor/graf/` owns graph mechanics. Tend reaches graf
only through `tend/graph.rs`; linking and persistence do not depend on it. Graf knows
no Tinkery phase or Seed Me format. Each library's `UPSTREAM` records its source and
local edits; [Pinstar API](../../vendor/pinstar/API.md) and
[Graf API](../../vendor/graf/API.md) describe their boundaries.

## Design score

John Ousterhout's *A Philosophy of Software Design* supplies these eight questions.
Each pass scores 1.25. Structural PRs name the rows they move.

| # | Question | Recorded assessment |
| --- | --- | --- |
| 1 | Can each module be described in one sentence? | Yes: see the module map. |
| 2 | Are interfaces simpler than implementations? | Yes |
| 3 | Can an implementation change without touching callers? | Yes: model contracts live beside decoders; tests align field names. Only `seed_me/` owns Seed Me's format. Review decodes its reply; reading joins it. |
| 4 | Do interface comments describe the abstraction? | Yes: modules and `pub` items carry interface comments; the build denies `missing_docs` and `unreachable_pub`. |
| 5 | Is design discussion part of review? | Yes: [the working guide](../../AGENTS.md#guardrails) requires changeability review and naming affected rows. |
| 6 | Does each module hide an important decision? | Yes: Conversation hides question, skip and scope rules. |
| 7 | Can a newcomer see the boundaries without reading code? | Yes: this page |
| 8 | Is design improvement part of ordinary work? | Yes |

**Recorded score: 10/10** (2026-10-12, after #39). This is not a fresh measurement.
The remaining debt is `brain_dump.rs`'s narrow interface over substantial routing.

The module map owns these boundaries. Preview and sending share conversation
staging and one drafting request boundary; submit sends the prepared, measured request
without reconstructing it. Measure owns its PR list, details and help; the menu owns
entry and return. Linking, graph geometry and the garden footer stay separate.
Think writes shared records; Tend
reads them. Shared `repo.rs` finds the root once.
Only Tinkery's ledger-backed Shape verification claims seed-ready; file discovery reports
unchecked declarations.

The shared SDLC read and seed identity improve diagnostics 3 and 6: both phases
use one source-owned join without knowing each other. The module map improves
boundary visibility (7). Seed checks, confirmation and ending now hide behind typed ledger calls,
extending diagnostics 3 and 6 without changing phase callers; the map keeps diagnostic 7 current.
Named seed prose fields derive from the ledger's canonical artifact sequence, improving
interface simplicity (2), format changeability (3) and hidden decisions (6): the model
cannot choose heading order, and the prompt attaches only compatible contract guidance.
The module map keeps that boundary visible (7).
Closing risks and completion-review rules remain ledger-owned, as do append-only teach-back
receipts that do not change decision premises (3, 6). Shape only displays risks and obtains
explicit keys for capture, comparison and confirmation (2); the map keeps these boundaries
visible (7). Mismatches never become a confirmation gate.
Retiring skill parameters simplifies interfaces (2); bundled guidance and Rust initialization
remove an external implementation dependency (3). Ledger saving, byte-preserving migration and unique-ID history attribution
hide their decisions behind shared boundaries (6); the module map records the owners (7).
No limit is relaxed; the recorded score remains 10/10.

First-run friction fixes keep probe-answer provenance at the seed prose boundary,
revision-bound reading in Shape, title slugs and combined shell commands behind the
shared handoff formatter, and older-run presentation in Watch. These improve interface
simplicity (2), changeability without caller churn (3) and hidden decisions (6); the map
keeps the owners visible (7). No architecture limit or recorded score changes.

Agent-operated Think and Shape resolve typed actors through one shared authorization boundary.
Persistence reads actor data rather than rewriting human prose; seed verification binds named
attribution and keeps human-only external intake unavailable. These improve interface simplicity
(2), changeability (3) and hidden decisions (6); this map and the
[confirmation contract](think.md#confirmation-attribution-185) keep boundaries visible (7).
No architecture limit or recorded score changes.

Removing Measure's external reader and configuration simplifies interfaces (2),
removes an external implementation dependency (3) and confines its unknown display
without reader lifecycle decisions (6). The module map keeps boundaries visible (7).
No diagnostic worsens; no architecture limit or recorded score is loosened.

Measure's PR reader hides GitHub process limits and cancellation from rendering (3, 6).
The phase owns selection and evidence semantics behind open/key/poll/render (2); the
module map records the boundary (7). No phase dependency or architecture limit changes.

## Limits enforced by `tests/architecture.rs`

| Limit | Today |
| --- | --- |
| Phase reaches across phases | none allowed |
| `BrainDump` fields / `pub` fields | 13 / 0 |
| Non-test source lines over 200 characters | 8 |
| Library files without a `//!` comment, `pub` items without a doc comment | none allowed |

Checks fail whenever code differs from a limit, even by improving it. Lower improved
limits in the same PR.
