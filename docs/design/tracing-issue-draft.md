# Draft issue: export one goal's work as one trace

**Draft, not published.** It follows the headings of the
[task template](../../.github/ISSUE_TEMPLATE/task.yml). Publication needs the operator's word.
The design is in [tracing.md](tracing.md).

## Primary area

shared

## Kind

enhancement

## Goal

The operator, 2026-11-28: "ideally its all self contained as one continuum somehwre, like from initial
think/intent/goal --> shape --> logs of workers --> verifier and review findings --> and my taste input and
pretty much abything that can be data for collecting data for error analysis." Then: "lets prioritize this first."

Reading (a guess): an agent doing error analysis needs every record of one goal's work in one place, keyed by
the goal, so it can code failures and count them across a batch.

## Evidence

Observed in the code at `cfb0128`. The [design note](tracing.md#what-exists-today) gives each file and line.

- Shape and Think model transcripts are deleted. Pi runs with `--print` and a temporary `--session-dir`.
- Pi Durable keeps the worker transcript in SQLite and in a mirrored `.jsonl`. It also prints `message_end` lines
  into `.pi.log`, which Measure and cap advice parse with two separate readers.
- The only join from a run to its goal is `seed_contract.source` in `run.json` and its sha256.
  Nothing joins the worker session, PR and driver chat to the goal.
- TEST HOME runs vanish unless an operator archives them by hand. This was observed in two dogfood batches.

Not verified: the exact name of the GenAI scores event (`gen_ai.evaluation.result`) in the current conventions.

## Cause

Not a bug.

## Scope and boundaries

One read-only headless export. It takes one or more `session_id` values, or the newest N, and an output folder.
For each session it writes one JSONL file of OpenTelemetry GenAI spans, events and scores, built from records that
exist today. It also writes a manifest of linked sources and one `coding.csv` sheet with empty `open_code` and
`axial_code` columns.

It changes no write path in Think, Shape, the worker or tink-sdlc. It calls no model and sends nothing over a network.
It writes only into the output folder that the caller names. Deleted sources, such as Shape transcripts and driver
chats, show as unknown.

Out of scope: keeping Shape and Think transcripts, deleting the `.pi.log` emit or its parsers, structured review
findings, a count command and any viewer.

## Done when

- [ ] For a fixture goal with a Think record, Shape revisions, one worker step with a Pi Durable mirror, a verify
      receipt, review findings, a PR record and a Taste label, the export holds one trace per stage step, all with the
      same `session.id`.
- [ ] Operator and relayed words in the export equal their source bytes, and each event names its source and sha256.
- [ ] A Tinkery reading is marked `tinkery-reading`, never `operator`.
- [ ] A missing or changed source reads as unknown in the manifest and the sheet, never as a pass or zero.
- [ ] The export reads no environment variables, credential files or `.env` files. A test proves it.
- [ ] The export refuses an output folder that already holds files, and writes nothing outside it.
- [ ] Reversion probe: removing the source-hash check makes the changed-source test fail.
- [ ] Full zero-call verification passes. The PR reports Rust, Node and Python counts and the PTY journeys separately.

## Context and dependencies

- Design: [tracing.md](tracing.md).
- Readers to reuse: `src/think_record.rs`, `src/seed_me/session.rs`, `src/seed_me/runs.rs`, `src/sdlc.rs`,
  `src/sdlc/files.rs`.
- Dependency: issue #1040 (headless resume, verify and finish) is on hold behind a planned tink-sdlc worker-admission
  redesign. The export reads `steps/<step>.json` and its `.session-dir`. If that redesign changes them, the reader
  changes with it.
- Decided by the operator: `runs/` stays in product repositories.

## Open decisions and permissions

- Design acceptance of [tracing.md](tracing.md), and implementation permission for this slice. Both are open.
- Data: whether an export can copy Pi Durable worker transcripts, which can hold secrets that a worker printed,
  or must link to them only.
- Budget: zero model or provider calls. Model-assisted coding needs a new, explicit budget.
