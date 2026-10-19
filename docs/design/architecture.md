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
work. Measure's [failure presentation](https://github.com/jon-devlapaz/tinkery/issues/119)
and [owned reader](https://github.com/jon-devlapaz/tinkery/issues/120) have separate
issues. The module map below describes responsibilities; `MODULES` owns area assignments.

[Shape](shape-and-handoff.md#shape) is a first-release phase, and [Watch](watch.md) a view. Approve and taste
get separate modules when built. There are no dependency exceptions: Think hands
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
unconfirmed sessions or complete factory telemetry. Exact external-run linkage
remains [unaccepted](job-records.md#goal-to-run-identity-prerequisite).
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
| `measure.rs` | The ledger's read-only report, display safety limits, failure facts and safe two-layer disclosure. |
| `measure/reader.rs` | One reader across visits: request ownership, deadline, bounded pipes, process-group cancellation and cleanup-first shutdown. |
| `measure/viewport.rs` | Incremental machine-sized diagnostic scrolling over bounded, grapheme-safe source segments; no subprocess or navigation policy. |
| `measure/plain.rs` | Prepared success-report offsets for glyphs and rows; visible-only clipping equivalent to the original Paragraph, including invisible runs. |
| `tend/mod.rs` | The saved-goal garden's discovery states, selection, detail and copying the continuation prompt again. |
| `tend/history.rs` | One owned history read at a time and a read-only conversation view with separate technical help. |
| `tend/linking.rs` | The source and target selection state, link refusals and derived needs wording, independent of garden drawing. |
| `tend/garden.rs` | The graph's framing: selected-line needs, read notes and explicit refusals. |
| `tend/graph.rs` | Graf adapter: path-keyed topology, deterministic layout, palette, labels, viewport and pointer geometry; no physics worker or writes. |
| `tend/discovery.rs` | Scanning for sessions off the paint path, with unknown, complete, incomplete and failed kept apart. |
| `tend::needs` | Tinkery-owned needs links, strict atomic persistence and readiness derived from readable session identities; never writes Seed Me files. |
| `seed_me/` | All Seed Me formats and helper calls: goal saving, session files, stages and continuation; `shaping.rs` reads seed status; `answering.rs` saves answers, questions, findings, drafts and confirmation (shared). |
| `shape.rs` | Goals awaiting seeds; each session's confirmed goal, stage, folder, words and next step. Reads and steps stay off the paint path. |
| `shape/words.rs` | Shape's in-memory textarea adapter: editing, selection, cursor-following and original line-ending provenance; no save or agent policy. |
| `shape/step.rs`, `shape/prompt.txt` | The step's budget check, folder, model input, prompt, reply decoding and recording through the shared helper boundary. Replies contain findings with a question or seed text. |
| `goal/mod.rs` | Goal parts, constraint normalisation and composing the goal block (shared). |
| `goal/confirmed.rs` | Rendering a confirmed goal the same way in review, receipt and tend detail (shared). |
| `think/brain_dump.rs` | Think's `BrainDump`, which routes keys, mouse and ticks to its four parts, paints them, and reports one `Exit` to the menu. |
| `think/brain_dump/conversation.rs` | Words, extracts, readings, question rules, skips, scope, requests and ordered record turns. Fields are private; methods hide these rules. It knows nothing about screens or jobs. |
| `think/brain_dump/reading.rs` | The model request, prompt, wire reply and lenient decoder, including open choices linked to questions. |
| `think/brain_dump/review.rs` | The decision review: its prompt, what it sees, and how its decisions join the reading's. |
| `think/brain_dump/prompts/` | The two prompts as text, one rule per line, with a note naming the finding behind each tuned rule. Notes never reach the model. |
| `think/brain_dump/host.rs` | Pi shaping then review, or practice simulation. Stateful hosts start separate histories for each Think visit. |
| `think/brain_dump/board.rs` | A verified reading; `Board` exists only after verification. |
| `think/brain_dump/checks.rs` | The per-send record shown in F5 "why". |
| `think/brain_dump/goal_view.rs`, `intact_view.rs`, `voice_input.rs`, `layout.rs` | The review screen, original pane, dictation and screen layout, including help, key bar and notices. |
| `think/brain_dump/help_view.rs` | Think's bounded help overlay: visual hierarchy, scrolling body, fixed navigation and the boundary against hidden actions. |
| `think/drafting.rs` | Think's Pi host: its model settings and Seed Me working-draft instructions, over the shared `pi.rs` boundary. |
| `think_record.rs`, `think_record/read.rs` | The shared confirmed-session format, its one write, and exact-session history attribution; no phase behavior. |
| `think/canvas.rs`, `note.rs` | Extracted cards, and the raw text box with safe display, grapheme-offset mapping and machine-sized wrapped rows. Editor storage does not own send policy. |
| `voice/` | Local dictation: the host boundary, helper process, protocol and owned setup. |
| `watch.rs` | The runs in one named tink-sdlc checkout from its API 1 `status --json`: refusing other interfaces, waiting runs first, one run's detail. |
| `watch/run.rs` | One bounded status read off the paint path: deadline, output limits, and stopping the process group when Watch is left. |

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
staging and one drafting request boundary. The menu routes Measure's open, cancel,
poll and shutdown interface; restore the terminal before cleanup on exit or error.
Prepared, bounded display avoids rebuilding all content per frame. Linking, graph
geometry and the garden footer stay separate. Think writes shared records; Tend
reads them; the menu retains the reader. Shared `repo.rs` finds the root once.
Only helper-backed Shape verification claims seed-ready; file discovery reports
unchecked declarations.

This documentation condensation improves boundary visibility (7); it changes no
module, enforced limit or measured score.

## Limits enforced by `tests/architecture.rs`

| Limit | Today |
| --- | --- |
| Phase reaches across phases | none allowed |
| `BrainDump` fields / `pub` fields | 13 / 0 |
| Non-test source lines over 200 characters | 13 |
| Library files without a `//!` comment, `pub` items without a doc comment | none allowed |

Checks fail whenever code differs from a limit, even by improving it. Lower improved
limits in the same PR.
