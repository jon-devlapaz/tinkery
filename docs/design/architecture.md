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

[Shape](shape-and-handoff.md#shape), [Approve](approve.md), [Watch](watch.md) and
[Taste](taste.md) have separate modules. There are no dependency exceptions: Think hands
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
[seed-to-run join](shape-and-handoff.md#sdlc-handoff) uses SDLC's source-owned record.

## Modules today

`main.rs` runs the terminal loop and `lib.rs` holds the palette and screen sizes. Every other
top-level module has one row, with its area from `MODULES`. Each file's `//!` comment owns the
detail. [The architecture check](../../tests/architecture.rs) fails if this table and `MODULES` differ.

| Module | Area | Owns |
| --- | --- | --- |
| `actions` | shared | The one key map Think and Shape share: F2 runs the main action, F1 lists the others and Esc goes back. |
| `approve` | approve | One verified seed at a time, copying and an explicit worker-start review. Navigation and copying never run work. |
| `calendar` | shared | Time for every writer and reader, and the one RFC 3339 parser. |
| `confirmation` | shared | Caller-declared confirmation context and the one boundary that turns it into an `Actor`. Never authenticated identity or permission. |
| `data_roots` | shared | Where sessions, Think records and Tend's stores live by default. Creates nothing. |
| `github` | shared | Every `gh` read, bounded and cancellable. Never writes to GitHub. |
| `goal` | shared | Goal parts and the one way they compose into the goal block. |
| `help` | shared | Scrolling help and key-bar fitting beside `actions`. No phase or wording. |
| `launch` | menu | The one `Launch` resolved from the command line, which every place reads. Writes nothing. |
| `measure` | measure | Recent PRs and closed-run postmortems, through explicit reads only. |
| `menu` | menu | The seven places, the stack of open places and the one `Visit` contract each keeps. Navigation never starts work. |
| `model_setup` | shared | The saved model setup and its one parser for file and flags. |
| `pi` | shared | The one place a model host starts, Pi or Claude Code, and the budget every request counts against. |
| `process` | shared | Bounded child processes and the background task that owns one. |
| `regular_file` | shared | The one door for opening files on disk. `clippy.toml` forbids `std::fs` opens and reads elsewhere. |
| `repo` | shared | Finding the Git repository Tinkery started in. |
| `sdlc` | shared | tink-sdlc's API 1 in one named checkout. No Tinkery session or seed rules. |
| `seed_me` | shared | Session formats, the ledger and goal saving. It alone owns seed layout and validation, and it is Think's only route to Tend. |
| `shape` | shape | A saved goal's seed and the questions that lead to it, from the screen or the `seed` command through one controller. |
| `taste` | taste | One result beside its unchanged goal. It writes only the operator's verdict label and, on retry, a fresh session. Never an approval, merge or agent launch. |
| `tend` | tend | The garden of saved goals and thoughts, needs links and Think history. Never edits a session. |
| `think` | think | Turning a brain dump into a goal the operator confirms, then saving it. Reads no files or repository. |
| `think_record` | shared | The one record of each confirmed Think session, written by the same `confirm` that saves the goal. |
| `trace` | shared | One goal's records from Think to Taste, joined by its `session_id`, read-only. Transcripts appear by path and SHA-256 only. |
| `thought` | shared | Repository-bound thought storage with exact-byte deduplication. No goal authority. |
| `voice` | shared | Local dictation: consent, setup, takes and their text. |
| `watch` | watch | The runs in one named tink-sdlc checkout, read-only. |
| `worker` | shared | Explicit, bounded worker actions through SDLC. SDLC owns gates, sessions and caps. |
| `words` | shared | The exact-word editor Shape uses. No save or agent policy. |

Vendored libraries are path dependencies, not phases: `vendor/pinstar/` owns
rendering and input primitives and `vendor/graf/` owns graph mechanics. Tend reaches graf
only through `tend/graph.rs`; linking and persistence do not depend on it. Graf knows
no Tinkery phase or Seed Me format. Each library's `UPSTREAM` records its source and
local edits; [Pinstar API](../../vendor/pinstar/API.md) and
[Graf API](../../vendor/graf/API.md) describe their boundaries.

For Shape's typed context, read [Think history](shape-and-handoff.md#think-history).

## Design score

John Ousterhout's *A Philosophy of Software Design* supplies these questions. Row 6 uses David Parnas's
1972 [decomposition criterion](https://doi.org/10.1145/361598.361623).
Structural PRs name the rows they move.

| # | Question |
| --- | --- |
| 1 | Can each module be described in one sentence? |
| 2 | Are interfaces simpler than implementations? |
| 3 | Can an implementation change without touching callers? |
| 4 | Do interface comments describe the abstraction? |
| 5 | Is design discussion part of review? |
| 6 | Does each module hide a decision likely to change? |
| 7 | Can a newcomer see the boundaries without reading code? |
| 8 | Is design improvement part of ordinary work? |

## Limits enforced by `tests/architecture.rs`

| Limit | Constant |
| --- | --- |
| Phase reaches across phases | none allowed |
| `BrainDump` fields / `pub` fields | `BRAIN_DUMP_FIELDS` / `BRAIN_DUMP_PUB_FIELDS` |
| Long non-test source lines | `LONG_LINE` / `LONG_LINES` |
| Test calls to `Conversation`'s test-only accessors | `CONVERSATION_TEST_ACCESS` |
| Library files without a `//!` comment, `pub` items without a doc comment | none allowed |

Checks fail whenever code differs from a limit, even by improving it. Lower improved
limits in the same PR.
