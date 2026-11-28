# Tracing: one continuum per goal

**Proposal, except the first slice.** The operator confirmed the tracing Tink and its defaults on 2026-11-28
("confirm", then "proceed"). The [first slice](#first-slice) ships as `tinkery seed trace`. The rest is unbuilt.
Design acceptance, each slice, data use and budget are separate operator decisions.
Read at tinkery `cfb0128` on 2026-11-27.

## Why

The operator, 2026-11-28: "ideally its all self contained as one continuum somehwre, like from initial
think/intent/goal --> shape --> logs of workers --> verifier and review findings --> and my taste input and
pretty much abything that can be data for collecting data for error analysis." Then: "lets prioritize this first."

Reading (a guess): the operator wants one place per goal that holds every record of that goal's work.
Error analysis must read that place without asking where each piece lives.

Agents, not the operator, are Tinkery's users. The continuum must be readable by a script and an agent first.

## What exists today

Each claim below was read in the code at the commit above.

- **Goal identity.** A Shape session folder is created as `session-XXXX` and its ledger records that name as
  `session_id` (`src/seed_me/ledger/storage.rs:30-42`, `src/seed_me/ledger/storage.rs:58-62`).
  The session folder holds `ledger.json`, `seed-contract.md`, `seed-checks.json` and `seed-confirmation.json`
  (`src/seed_me/files.rs:6-13`).
- **Think.** One Think record is written once, at the typed confirm (`src/think_record.rs:1-8`).
  It keeps operator words verbatim as events (`src/think_record.rs:54-130`) and names the session by
  `seed_me_session_id` (`src/think_record.rs:341-343`). Its file is `<time>_<session>.json` under the records
  root (`src/think_record.rs:365`).
- **Shape.** Each ledger transition keeps the superseded node state with `superseded_at` and `reason`
  (`src/seed_me/ledger.rs:1022-1038`). A Shape step sends one model request (`src/shape/step.rs:815-834`) and saves
  only the decoded result in the ledger (`src/shape/step.rs:836-870`).
- **Model transcripts are deleted.** Every Pi request runs with `--print` and a fresh `--session-dir`
  (`src/pi.rs:459-468`, `src/pi.rs:568-572`). Tinkery reads that folder only to count responses
  (`src/pi.rs:677-700`), then the temporary folder is dropped. Claude Code runs with `--no-session-persistence`
  (`src/pi.rs:625`). Think drafting uses the same call (`src/think/brain_dump/host.rs:130-139`).
- **Worker start.** Tinkery prepares a step with a report path, the model pin, the launcher and the cap
  (`src/worker.rs:746-783`). An agent start appends the relayed words, `run_id`, step and checkout to
  `runs/<slug>/worker-authorization.jsonl` (`src/worker/authorization.rs:236-278`).
- **Run records.** `sdlc.py new` writes `run.json` with a random `run_id` (`_system/scripts/sdlc.py:943-944`).
  Stage 1 copies the seed and records `seed_contract.source`, the session's own `seed-contract.md`
  (`_system/scripts/sdlc.py:2763-2776`). That path and its sha256 are the only join between a run and a goal
  (`src/seed_me/runs.rs:24-40`). Stage 3 commits `runs/<slug>` onto the run branch
  (`_system/scripts/sdlc.py:2355-2367`, `_system/scripts/sdlc.py:2806-2817`).
- **Steps.** Each step contract `steps/<step>.json` records `run_id`, model, launcher digest, cap and, after launch,
  the Pi session id (`_system/scripts/sdlc.py:3199-3207`).
- **Verify.** `04-test/output/verification.json` and `test-log.md` hold the candidate, the per-item proof and the log
  digest (`_system/scripts/sdlc.py:1139-1141`, `_system/scripts/sdlc.py:1289-1297`).
  Probes append to `04-test/output/probes.jsonl` (`_system/scripts/sdlc.py:1090`).
- **Gates.** Each decision is one file in `decisions/` (`_system/scripts/sdlc.py:376`, `_system/scripts/sdlc.py:989`).
- **Review.** Findings are one Markdown file, `05-deploy/output/REVIEW-findings.md`, with a `Round: <n>` line
  (`_system/scripts/sdlc.py:2686-2696`). Nothing else in it is structured.
- **PR.** `pr.json` binds the PR URL to the verified candidate head (`_system/scripts/sdlc.py:1662-1686`).
- **Taste.** A verdict is a line in `labels.jsonl` with verbatim words, kind and `source: taste@<head>`
  (`src/taste/label.rs:219-263`, `_system/scripts/sdlc.py:1621-1659`).
- **Worker transcripts are kept three times.** Pi Durable keeps one SQLite store per step and mirrors every
  message to `<session>.jsonl` (`_system/scripts/pi-durable/worker.mjs:3-9`, `_system/scripts/pi-durable/worker.mjs:41-42`,
  `_system/scripts/pi-durable/worker.mjs:77-96`). It also prints `message_end` events into the step's `.pi.log`
  (`_system/scripts/pi-durable/worker.mjs:117`, `_system/scripts/pi-durable-launcher:180-185`).
  The default session folder is under `~/.local/share/tink-sdlc/pi-durable/sessions`, outside the run
  (`_system/scripts/pi-durable-launcher:74-76`, `_system/scripts/pi-durable-launcher:140-146`).
- **Three readers parse worker logs.** The launcher's cap watcher reads the `.jsonl` (`_system/scripts/pi-launcher:117`,
  `_system/scripts/pi-launcher:410`). Measure counts `message_end` lines in `.pi.log` (`src/measure/runs/records.rs:258-290`).
  Cap advice parses the same `.pi.log` with its own types (`src/worker/orientation.rs:1-2`, `src/worker/orientation.rs:189-195`).
- **Data roots.** Sessions, Think records and Tend sit under `$HOME/.local/share/tinkery` (`src/data_roots.rs:21-27`).
  A TEST HOME takes all of them away when it is removed.

Gaps observed in two dogfood batches, and confirmed by the code above:

1. Shape and Think model transcripts are deleted.
2. The worker transcript has three copies and three parsers.
3. No record joins the goal, run, worker session, PR and driver chat. The run-to-goal join is a file path and hash.
4. TEST HOME runs vanish unless an operator archives them by hand.

## Model

**One session per goal.** The session key is the goal's `session_id`. It is already in the ledger, the Think record,
and, through `seed_contract.source`, in every run built from that goal. This design adds no new identity.

**One trace per stage step.** A trace is one unit of work:

| Trace | Starts at | Ends at |
| --- | --- | --- |
| `think` | first operator words | typed goal confirm |
| `shape` (one per step) | operator F2 or headless `seed step` | ledger write or refusal |
| `seed-confirm` | confirm key | `seed-confirmation.json` |
| `worker` (one per sdlc step) | `step prepare` | step finished, stopped or capped |
| `verify` (one per run) | `sdlc.py verify` | `verification.json` result |
| `review` (one per round) | `stage <slug> 5` | `REVIEW-findings.md` round |
| `hand-off` | `sdlc.py pr` | `close` and labels |

**Spans follow the OpenTelemetry GenAI conventions.** A trace's root span is `invoke_agent <agent>`.
A model request is a child `chat <model>` span. A tool call is a child `execute_tool <tool>` span.
The conventions moved to the semantic-conventions-genai repository at v1.42.0. They are still "Development".
The store must pin one convention version and record it in each file.

Span attributes Tinkery sets:

- `session.id`: the goal's `session_id`.
- `gen_ai.operation.name`, `gen_ai.agent.name`, `gen_ai.request.model`, `gen_ai.provider.name` and usage counts,
  when the source records them. Unknown stays absent, never zero.
- `gen_ai.conversation.id`: the Pi or Pi Durable session id, when one exists.
- `tinkery.run_id`, `tinkery.run.slug`, `tinkery.step`, `tinkery.checkout`, `tinkery.commit`: where the work ran.

**Words are events.** Operator words, confirmations and relayed words are span events with the text verbatim.
Each event carries `tinkery.words.source`: `operator`, `relayed` or `agent`. A Tinkery reading carries
`tinkery.words.source = tinkery-reading`, so a reader can never take it for operator words.

**Judgments are scores.** A score is a record bound to one span and one commit:

- A Taste verdict: label `pass` or `fail`, the verbatim words, the signer, and the commit from `taste@<head>`.
- A review finding: severity, the finding text, the round and the reviewed commit.
- A verify result: each checklist and seed-check item with `proven`, `attested` or `pending`, and the candidate head.

The GenAI conventions name a `gen_ai.evaluation.result` event for scores. That name was not checked in this read.
The implementation must check it against the pinned version before using it.

## Storage

- **Local only.** One trace store per data root. Nothing is sent over a network by default
  ([Yohaku article 2](../yohaku.md#2-action-stays-within-explicit-authority)).
- **Links first.** A span points at the existing record by path and sha256. It copies a record only when no other
  copy survives, for example a Shape model transcript.
- **Format.** One JSON object per line, in the OpenTelemetry span data model, so an OTLP file exporter or a
  viewer can read it later. Rust needs no new database dependency.
- **A viewer is optional.** Phoenix, Langfuse or another self-hosted viewer can read an export. Choosing or running
  one is out of scope.

## Every artifact, and what it becomes

| Artifact | Writer | Becomes | Deletable after |
| --- | --- | --- | --- |
| Think record turns | `src/think_record.rs:277` | `think` trace: events per turn, link to the record | nothing |
| Think model requests | `src/think/brain_dump/host.rs:139` | `chat` spans, transcript kept (slice 2) | nothing |
| `ledger.json` node history | `src/seed_me/ledger.rs:1022` | `shape` spans: one per revision, link by node id and version | nothing |
| Shape Pi session | `src/pi.rs:459-468` (deleted today) | `chat` and `execute_tool` spans, transcript kept (slice 2) | nothing |
| Teach-back request | `src/shape/step/teach_back.rs:36` | `chat` span under its `shape` trace | nothing |
| `seed-confirmation.json` | `src/seed_me/files.rs:13` | `seed-confirm` event, link | nothing |
| `worker-authorization.jsonl` | `src/worker/authorization.rs:249` | relayed-words event on the `worker` trace | nothing |
| `run.json` | `_system/scripts/sdlc.py:943` | attributes `tinkery.run_id`, the goal join | nothing |
| `decisions/*.json` | `_system/scripts/sdlc.py:989` | gate events, link | nothing |
| `steps/<step>.json` | `_system/scripts/sdlc.py:3199` | `worker` root span attributes | nothing |
| Pi Durable SQLite store | `_system/scripts/pi-durable/worker.mjs:41` | stays, the authority for resume | nothing |
| Pi Durable `<session>.jsonl` mirror | `_system/scripts/pi-durable/worker.mjs:95` | `chat` and `execute_tool` spans, read once | stays: the launcher's cap watcher reads it |
| `message_end` lines in `.pi.log` | `_system/scripts/pi-durable/worker.mjs:117` | nothing new | the emit, once Measure and cap advice read the mirror |
| Measure `.pi.log` parser | `src/measure/runs/records.rs:258` | replaced by the trace reader | this parser |
| Cap advice `.pi.log` parser | `src/worker/orientation.rs:189` | replaced by the trace reader | this parser |
| `.cap-stop` evidence | `_system/scripts/pi-launcher:376` | event on the `worker` span | nothing |
| `verification.json`, `test-log.md` | `_system/scripts/sdlc.py:1296` | `verify` trace, item scores bound to the candidate | nothing |
| `probes.jsonl` | `_system/scripts/sdlc.py:1090` | reversion-probe events | nothing |
| `REVIEW-findings.md` | stage 5 reviewer | `review` trace, link (scores need a structured format) | nothing |
| `pr.json` | `_system/scripts/sdlc.py:1683` | `hand-off` span link to the PR and candidate | nothing |
| `labels.jsonl` | `_system/scripts/sdlc.py:1658` | Taste scores bound to `taste@<head>` | nothing |
| Driver chat (Claude Code, Pi operator) | outside Tinkery | link only, when the agent names its session | nothing |

`runs/` stays in product repositories, as the operator decided. The trace links to those files at a commit.

## Privacy

- **Stored.** Operator words, readings, questions, answers, model prompts and replies, tool calls and their results,
  commits and verdicts. All of it is already on this machine today, or was until a temporary folder was dropped.
- **Verbatim words.** Operator and relayed words are stored byte for byte as event attributes, never trimmed.
  Each event also stores the sha256 of its source record, so a reader can check the words against the original.
- **Never stored.** Environment variables, Pi or Claude Code credential files, provider API keys, `auth.json`,
  tokens and the contents of `.env` files. Tinkery reads no host credentials today (`src/pi.rs:9`), and the trace
  reader must keep that refusal.
- **Unknown risk.** A worker's shell output can contain a secret that the worker printed. The trace links to the Pi
  Durable transcript and does not copy it, unless the operator allows copying for an export.
- **Nothing leaves.** An export is a local folder that the caller names. Sending it anywhere is a separate decision.

## Error-analysis export

A batch exports N sessions into one folder:

1. The caller names sessions by `session_id`, or asks for the newest N.
2. For each session, the exporter writes `<session_id>.jsonl` with every span, event and score, and a manifest of each
   linked source with its path, sha256 and whether it was readable.
3. The exporter writes one coding sheet, `coding.csv`, with one row per trace: session, trace, stage, outcome,
   operator words, last model reply, scores, and two empty columns, `open_code` and `axial_code`.
4. A person or an agent fills `open_code` with free notes on the first failure in each trace (open coding).
   Then they group the notes into a failure taxonomy in `axial_code` (axial coding).
5. A count command reads the filled sheet and prints each axial code with its count and its session ids.

The exporter calls no model. Model-assisted coding needs an explicit budget, and the default is zero.
A missing or changed source shows as unknown in the manifest and the sheet, never as a pass.
This export replaces hand archiving of TEST HOME runs: the dogfood harness exports before it removes the HOME.

## Subtract first

This lets us delete:

- The `message_end` emit in `.pi.log` and its two Rust parsers, after Measure and cap advice read the mirror through
  one trace reader.
- Hand archiving of TEST HOME runs.
- Ad-hoc joins between a goal, its run, its worker session and its PR. One reader owns the join.

This keeps unchanged:

- The ledger, the Think record, every `runs/` file and the Pi Durable store. They remain the authorities.
- The launcher's live cap watcher and Tinkery's response counter (`src/pi.rs:677`). They count during a request, and a
  trace reader runs after it.

### First slice

**Shipped:** `tinkery [--record-root DIR] [--sdlc-root ROOT] seed trace DIR`
([Headless commands](../terminal-setup.md#headless-commands)). It reads one session and prints one JSON line per
record, in reading order: goal, Think, Shape, confirmation, then each run built from the session (gates, worker
authorization, steps and transcripts, verify items as scores, review, PR, Taste verdicts as scores bound to
`taste@<head>`). `src/trace.rs` owns the join. It changes no write path in Think, Shape, the worker or tink-sdlc.

Each line has `session`, `kind` (`event`, `score` or `unknown`), `stage`, `name`, `at` and `actor`, plus the
record's own fields. Its format is `tinkery.trace/1`, named on the first line.

- **Runs.** A run belongs to the session when its `seed_contract.source` is the session's `seed-contract.md`.
  `seed_matches` says whether the bytes still match the intake hash. Runs come from `--sdlc-root`, else from the
  repository that the Think record names.
- **Retries.** A retry is a new session. Its goal evidence holds the Taste rejection words and the rejected
  `run_id` (`src/seed_me/retry.rs`). The trace prints both, then the prior session, or `unknown` when the run is
  not in the checkout.
- **Content.** Operator words are verbatim. A transcript, ledger, log or review file is only a path and SHA-256.
- **Unknown lines today:** Think model transcript, Shape model transcript, driver session, and any worker
  transcript whose step records no session folder. These are the scope of slice 2.

Narrower than first proposed: one session per call on stdout instead of an export folder, no `coding.csv`, and no
OpenTelemetry names yet. `gen_ai.evaluation.result` is still unchecked, so no line uses it.

### Later slices, each a separate decision

2. Keep Shape and Think model transcripts in the session folder instead of deleting them.
3. Move Measure and cap advice onto the trace reader, then delete the `.pi.log` emit and both parsers.
4. A structured review-findings format in tink-sdlc, so findings become scores.
5. The coding sheet's count command.
