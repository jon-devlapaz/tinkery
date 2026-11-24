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

Published task scope and status live in GitHub Issues. Measure's former
[failure presentation](https://github.com/jon-devlapaz/tinkery/issues/119) and
[owned reader](https://github.com/jon-devlapaz/tinkery/issues/120) are retired.
`MODULES` owns area assignments; the map below describes responsibilities.

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

| Module | Owns |
| --- | --- |
| `main.rs`, `main/input.rs` | The terminal loop and bounded ordered input. Mouse input after resize waits for a source-level drain acknowledgement for that resize, never an empty transfer queue. Input and read completions wake the UI without shortening its heartbeat. A Unix hangup-only watchdog reports terminal loss and detaches a trapped reader; normal shutdown joins input workers. Opens the menu, or Think alone, as one `Home` from one resolved `Launch`. |
| `lib.rs` | The palette (one job per colour) in light or dark, the header's light/dark switch and the model label's room, the minimum terminal size, paging, grapheme-safe list titles and the clipped-view `more` cue. |
| `confirmation.rs` | Caller-declared confirmation context, its named constructors and the one boundary that resolves it into the `Actor` every session write takes; serde words are the stored format. Never authenticated identity, seed confirmation or implementation permission (shared). |
| `calendar.rs` | Time for every writer and reader: UTC timestamps from the system clock, the receipts' one "now" stamp, the operator's local date and the one RFC 3339 parser. Each writer chooses its stored fraction and zone suffix (shared). |
| `launch.rs` | One option table for parsing and `--help`, and the one `Launch` resolved from it: the `--operator` declaration, data roots, the one drafting transport and Shape agent, Think's `ThinkConfig`, worker and Watch inputs, and `seed new`/`seed step`. Home, Think alone, Think and the command all read this same value; it writes nothing (menu area). |
| `data_roots.rs` | Where sessions, Think records and Tend's stores live by default: one absolute-HOME check, one refusal per root naming the flag that replaces it. Creates nothing (shared). |
| `model_setup.rs`, `model_setup/panel.rs` | The saved model setup's one parser (file and flags), with private fields, the default thinking level and the agent rule (`Setup::for_agent`) named once, atomic save, header label and Model panel; the terminal loop owns when it shows (shared). |
| `pi.rs` | The one place a model host starts, Pi or Claude Code: which models and levels each accepts, a bounded, cancellable request with or without read-only tools held inside one folder ([`pi/folder.mjs`](../../pi/folder.mjs) for Pi, `dontAsk` for Claude Code), the call budget, one launch's `Drafting` transport (a checked call and its allowance, which Shape and Tend's connections share) and prompt-file assembly (shared). |
| `help.rs` | Beside `actions.rs`: help that scrolls under a fixed footer row, and key-bar fitting that closes up, then shortens, before it cuts anything off. No phase or wording (shared). |
| `process.rs` | Bounded child processes: start in an owned process group, bounded stdout and stderr, a deadline, cancellation, stop-and-reap of the group, and a typed outcome that keeps exit status, stdout and stderr apart; plus `Task`, the owned background task with cancel-on-drop. No screen words, tool error formats or phase names (shared). |
| `regular_file.rs` | The one door for opening files on disk: nonblocking opens that refuse anything but a regular file, bounded reads that never follow a link, whole-or-nothing publishes with directory syncs, and random bytes. `clippy.toml` forbids `std::fs` opens and reads elsewhere (shared). |
| `github.rs` | Every `gh` call: the one canonical PR URL check (`PrUrl`), PR view, PR list, a file at a commit with 404 as absence, browser opening, and one reading of each `statusCheckRollup` entry. Bounded at 30 seconds and cancellable; never shows `gh` stderr, writes to GitHub or retries (shared). |
| `repo.rs` | Finding the Git repository Tinkery was started in, by looking for `.git` only (shared). |
| `menu.rs` | The menu, one stack of open places over it, and the one `Visit` contract each place keeps: key, paste, mouse, poll and render, each answered by one `Outcome` (stay, model, back, home, quit, open). What outlives a visit stays in the menu: the session scan, the Approve verifier, receipts, the key bar choice and the worker panel. Think launched alone is the same `Home`. |
| `menu/visits.rs` | Each place's side of `Visit`: how its keys, help and reads map to an `Outcome`. Help that blocks keys stays inside the place; no outcome starts work a place did not already guard. |
| `menu/retry.rs` | The hand-off of one reviewed Taste rejection to a saved Shape attempt. |
| `menu/place.rs` | The seven-place enum, display names and purposes, and ordered navigation list; no routing or actions. |
| `measure.rs` | Recent PR list, details and evidence limits; explicit reads only, no writes or providers. |
| `measure/runs.rs`, `measure/runs/records.rs` | Closed-run postmortems: one explicit, cancellable read of tink-sdlc status, the run's labels, its steps' Pi launch logs and Git, then list and details. Unknown is never zero; no writes or providers. |
| `tend/mod.rs` | The mixed garden's discovery states, identity and goal detail, with goal-only navigation to Shape and saved Think history. Owns every Tend key, help layer, reader and refresh rule behind one place surface (`key`, `paste`, `pointer`, `poll`, `offer`, `render`); one branch per state gives both the help and the key bar. |
| `tend/thought.rs` | Repository thought selection, exact-word reader and unsent Think copies; no editing, persistence or goal authority. |
| `thought.rs` | Shared repository-bound thought storage with exact-byte deduplication and durable-write outcomes; no phase imports or goal authority. |
| `tend/connection.rs` | One bounded explicit request, latest exploration, source freshness and receipt coordination. |
| `tend/connection/panel.rs` | Source and guidance review, explicit discovery, goal save and correction edits. |
| `tend/connection/correction.rs` | One repository correction, bounded reads and stale-view-safe explicit replacement. |
| `seed_me/connection.rs` | Strict v1/v2 Tend creation evidence and exact correction words shared by history, attribution and saving; the one place that evidence meets Think records for history and origins. |
| `seed_me/connection_save.rs` | Frozen confirmation, owned staging and exclusive publication to one destination. |
| `tend/history.rs` | One owned history read at a time and a read-only conversation view with separate technical help. |
| `tend/linking.rs` | The source and target selection state, link refusals and derived needs wording, independent of garden drawing. |
| `tend/garden.rs` | The graph's framing: selected-line needs, read notes and explicit refusals. |
| `tend/graph.rs` | Graf adapter: each graf node keeps its typed goal or thought entry, so selection, rings and labels never parse string ids; deterministic layout, palette, labels, viewport and pointer geometry; no physics worker or writes. |
| `tend/discovery.rs` | Scanning for sessions off the paint path, with unknown, complete, incomplete and failed kept apart, and the one ledger-id uniqueness rule that linking, the graph, readiness and connections read. It trusts ids that `seed_me::read` already checked. |
| `tend::needs` | Tinkery-owned needs links, strict atomic persistence and readiness derived from readable session identities; never writes session files. |
| `seed_me/` | Session formats, goal saving and the one-time root migration. `session.rs` holds the one ledger-id rule (`valid_id`) that every id read from disk passes. Goal initialization uses the Rust ledger; exit receipts retain data until printing, when the shared verifier supplies the current next step. Contract guidance is bundled, and copied intake commands name this Tinkery executable (shared). |
| `seed_me/ledger` | Validation, transitions, explicit answer actions and cascading reopen, confirmation risks and completion-review coverage, byte-bound teach-back records, seed decisions and checks, confirmation receipts, verification and ending; shared writer lock and atomic saves. No phase policy, viewer or network connection. Publication refusals carry a kind (stale, busy, unverified, refused), so callers choose words without matching text; republished schema-1 ledgers keep golden bytes. Counters are 128-bit integers, other integers keep their digits, and saved bytes and digests keep the former number forms; handoff and status are typed results. |
| `seed_me/files.rs` | The session folder's file names, seed reads, and the Tink Contract's one header parser: status, `revision:` label, title and the Full section headings. |
| `approve.rs` | One verified seed at a time, read-only commands, a retained verifier and navigation to worker-start review; no execution on navigation or copying. |
| `seed_me/retry.rs` | The retry hand-off in one place: a new attempt's starting checkout and commit, pinned in the goal's evidence, and the Git checks that hold a checkout and build to that commit; carries the prior goal unchanged and its decisions as assumptions. Only the operator's save creates the session. |
| `seed_me/ready.rs` | Where a session's seed stands, read once through the ledger and its verifier for Shape, receipts and handoff; consistent verified goal/acceptance snapshots and typed intake shared by copying and explicit execution; no mutation. |
| `seed_me/runs.rs` | The run-to-seed join: a run lends its goal, or counts as started, only through its recorded canonical source path and current bytes; reads only. |
| `worker.rs`, `worker/orientation.rs`, `worker/panel.rs` | Bounded intake, stage and worker actions through SDLC, including whether a failed start permits retry or requires inspection. The one step-state model and its words, which Watch also uses. The worker screen and its start disclosure, beside the start they describe. Recent Pi launch logs, read off the terminal thread, inform the operator’s cap choice before F2 is offered. Only the explicitly configured allowance can reach the worker. |
| `shape.rs`, `shape/help.rs` | Goals awaiting seeds; each session's confirmed goal, stage, folder, words, draft reading and next step. One constructor takes where it opens, the agent, the attribution and the hold; one `offer` takes a whole discovery read. F1 help, including capped discovery failures, paints through `help.rs`. Reads and steps stay off the paint path. |
| `shape/answering.rs`, `shape/answering/` | Adapts Shape actions, typed facts and operator review reasons to the shared ledger without duplicating its fact rules. Ledger words decode here once: a broken rule as a typed refusal with its node, who settled a decision, and an option's undo cost. |
| `shape/shaping.rs` | Shape's view of where a seed stands: questions, findings, decisions, review lines and a draft's view (its words, criteria and flags) over the shared classification; no subprocess or mutation. |
| `shape/forms.rs` | Unsaved deferral and replacement-answer fields, navigation and cancellation; no validation or publication policy. |
| `words.rs` | Exact-word textarea for Shape: editing, selection, cursor-following and original line-ending provenance; no save or agent policy. |
| `shape/command.rs` | Shape without a screen: saving a goal's exact words, showing where a seed stands as JSON and taking one explicit step; it builds the same work the screen does, runs it through the same controller and adds no ledger rule. |
| `shape/step.rs`, `shape/prompt.txt` | The one session controller the screen and the command share: who each write is attributed to, what a step reads (the folder, and Think's answers from one read of its record, whose refusal is said, not dropped), the one budget check, and one runner and outcome for every model step, confirmation and no-model change. Also the model input, prompt, reply decoding and recording through the shared session boundary. Replies contain findings with a question or named seed prose fields; `seed_me` alone owns their layout and validation. |
| `goal/mod.rs` | Goal parts, constraint normalisation and composing the goal block (shared). |
| `goal/confirmed.rs` | Rendering a confirmed goal the same way in review, receipt and tend detail, and the one wording of a run's goal or its absence for Taste and Tend (shared). |
| `think/brain_dump.rs` | Think's `BrainDump`, which routes keys, mouse and ticks to its four parts, paints them, and reports one `Exit` to the menu. |
| `think/brain_dump/conversation.rs` | Words, extracts, readings, question rules, skips, scope, unresolved review questions and notes, requests and ordered record turns. Fields are private; methods hide these rules. It knows nothing about screens or jobs. |
| `think/brain_dump/reading.rs` | The model request, prompt, wire reply and lenient decoder, including open choices linked to questions. |
| `think/brain_dump/suggestion.rs` | Optional proposal decoding and copying into an empty draft; never submission or model context. |
| `think/brain_dump/review.rs` | The decision review: its prompt, what it sees, and how its decisions join the reading's. |
| `think/brain_dump/prompts/` | The two prompts as text, one rule per line, with a note naming the finding behind each tuned rule. Notes never reach the model. |
| `think/brain_dump/host.rs` | The one model host: harness and model settings, bundled working-draft instructions over the shared `pi.rs` boundary, the one `read` (shaping then review) that every host and `think_eval` use, or practice simulation. Stateful hosts start separate histories for each Think visit. |
| `think/brain_dump/board.rs` | A verified reading; `Board` exists only after verification. |
| `think/brain_dump/checks.rs` | The per-send record shown in "Why", from F1. |
| `think/brain_dump/goal_view.rs`, `intact_view.rs`, `voice_input.rs`, `layout.rs` | The review screen, original pane, dictation and screen layout, including help, key bar and notices. |
| `think/brain_dump/help_view.rs` | Think's bounded help overlay: visual hierarchy, scrolling body, fixed navigation and the boundary against hidden actions. |
| `think/drafting.rs` | Which model requests one send makes, and the background job that runs them. |
| `think_record.rs`, `think_record/read.rs` | Confirmed-session format as one typed record that both writes and reads, exact-session attribution, one record scan shared by history and origin lookup, formatted history and typed submissions for Shape; Think records only, no Tend evidence and no phase behavior. |
| `think/canvas.rs`, `note.rs` | Extracted cards (the canvas keeps pinstar's state private, places the cards and reports their positions, selection and edit state), and the raw text box with safe display, grapheme-offset mapping and machine-sized wrapped rows. Editor storage does not own send policy. |
| `voice/` | Local dictation: the host boundary, helper process, protocol with its closed failure kinds, and owned setup. |
| `watch.rs`, `watch/state.rs`, `watch/steps.rs` | The runs in one named tink-sdlc checkout, rendered from the shared typed report: waiting runs first, one run's detail and worker attention in the worker's state words. `b` names a run for the worker screen and builds no worker identity. No API-word recognition. |
| `sdlc.rs`, `sdlc/run.rs`, `sdlc/source.rs` | Shared read-only API 1 `status(slug)` and `list()` reads with one `Run::parse` word boundary and typed verification, receipt results, validated pending judgment IDs, gates, checklist methods/results and worker steps with their cap flag, and one `StepContract` reader bound to its run and checkout. Unknown, missing and null values remain distinct for operation-specific refusals. Checklist attestation obligations (missing ownership fails closed), the canonical lowercase UUIDv4 run-ID rule, the one installed-reader check, the check that a checkout's files match its checked commit, and API 1 refusal words on top of `process`; no store, checkout discovery, phase policy or session rules. |
| `taste.rs`, `taste/read.rs` | One result's unchanged goal, revision-bound observations, the one question and external copy handoff; explicitly mapped PRs, read-only status/GitHub reads and deliberate browser navigation, never an approval, merge or agent launch. `read.rs` types each item's evidence owner and each owed gate, with their words in one place. |
| `taste/retry.rs` | The latest rejection's words and checked commit, a read-only review, and on F2 a fresh session with a detached checkout; never a worker or commit. |
| `taste/preview.rs` | Exact-head repo preview declarations, local HTTP readiness and explicitly launched temporary checkout/process ownership; no inferred commands, public URLs, agent or verdict. |
| `taste/next.rs` | Deterministic post-acceptance pause and next-dump proposal from snapshot evidence; no model, verdict, structural diagnosis or permission. The menu alone passes its unsent draft to Think. |
| `taste/label.rs` | Binding and reading the operator's pass/fail verdict for one exact head, including stale legacy records and latest rejection words; its one write through the checkout's `sdlc.py label`, only on F2, words unchanged, signed with the name shown; never an approval, merge or agent. |
| `taste/view.rs` | Title and goal excerpts with result counts, a one-question review, palette-bound headings and statuses, and details on request with verbatim hanging check wraps and evidence; no source reads, gate derivation or copy formatting. |
| `taste/links.rs`, `taste/evidence.rs` | The run's own recorded PR (API 1 `pr`), unknown when absent or malformed; source-owned log pointers and check methods as evidence references, never prose parsing, a mapping store, log reads or check execution. |

Vendored libraries are path dependencies, not phases: `vendor/pinstar/` owns
rendering and input primitives and `vendor/graf/` owns graph mechanics. Tend reaches graf
only through `tend/graph.rs`; linking and persistence do not depend on it. Graf knows
no Tinkery phase or Seed Me format. Each library's `UPSTREAM` records its source and
local edits; [Pinstar API](../../vendor/pinstar/API.md) and
[Graf API](../../vendor/graf/API.md) describe their boundaries.

Design reasons remain with [theme ownership](https://github.com/jon-devlapaz/tinkery/pull/430),
[presentation receipts](https://github.com/jon-devlapaz/tinkery/pull/451),
[history sequence](https://github.com/jon-devlapaz/tinkery/pull/458) and
[typed history](https://github.com/jon-devlapaz/tinkery/pull/462).
For Shape's typed context, read [Think history](shape-and-handoff.md#think-history).

## Design score

John Ousterhout's *A Philosophy of Software Design* supplies these eight questions.
Structural PRs name the rows they move.

| # | Question |
| --- | --- |
| 1 | Can each module be described in one sentence? |
| 2 | Are interfaces simpler than implementations? |
| 3 | Can an implementation change without touching callers? |
| 4 | Do interface comments describe the abstraction? |
| 5 | Is design discussion part of review? |
| 6 | Does each module hide an important decision? |
| 7 | Can a newcomer see the boundaries without reading code? |
| 8 | Is design improvement part of ordinary work? |

## Limits enforced by `tests/architecture.rs`

| Limit | Today |
| --- | --- |
| Phase reaches across phases | none allowed |
| `BrainDump` fields / `pub` fields | 13 / 0 |
| Long non-test source lines | See `LONG_LINE` and `LONG_LINES` in [the owning check](../../tests/architecture.rs) |
| Test calls to `Conversation`'s test-only accessors | 74 |
| Library files without a `//!` comment, `pub` items without a doc comment | none allowed |

Checks fail whenever code differs from a limit, even by improving it. Lower improved
limits in the same PR.
