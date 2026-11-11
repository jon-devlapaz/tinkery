status: confirmed for intake
# Seed: The worker start disclosure shows the seed SHA-256, for people and agents who start a worker.
revision: r1
Confirmed by: "confirm; caller-declared declared: agent; typed confirm at this terminal; declaration is not verified identity" (revision r1, 2026-11-11). Not approved for implementation.
date: 2026-11-11

## You are confirming
Goal: the worker start disclosure shows the seed SHA-256, for people and agents who start a worker.
What gets built: one line in the worker start disclosure (Approve F2 and the worker screen) showing the full SHA-256 of the seed the worker will import, for human and agent starts, plus a test that drives the public worker screen and checks the shown value equals the seed's digest.
What does not: the binding rules, the start keys, the recorded files and every other screen stay unchanged.
Proposed first slice: add a seed SHA-256 line to the disclosure text built in Panel::review in src/menu/worker.rs, and assert it in tests/worker_terminal.py (proposal only).
Unknowns: whether the 64-character line fits the 100x30 screen without wrapping badly, and which worker_terminal.py journey is the best place for the assertion.

## Problem statement
The worker start disclosure does not show the seed SHA-256. Panel::review in src/menu/worker.rs builds the text from the Pi model, response cap, checkout, retry commit and consent wording, plus the agent relay block from Gate::disclosure. It never prints target.sha256, although SeedTarget in src/seed_me/ready.rs:18 carries it and the agent binding in src/worker/authorization.rs:83 already compares it. A person therefore cannot confirm the shown SHA-256 matches sha256sum of the seed the worker will import. That confirmation is the stated reason for the change.

## Proposed outcome
The worker start disclosure displays the full seed SHA-256 for human starts and for agent starts. A person can run sha256sum on the seed file and compare it with the screen. Success means the Done when line holds, a test drives the public worker screen and asserts the shown value equals the seed's digest, and python3 scripts/full-check.py passes. A person opening the screen on a fixture seed and comparing it with sha256sum is the taste check.

## Acceptance criteria

The worker start disclosure displays the seed's SHA-256 for human starts; the worker start disclosure displays the seed's SHA-256 for agent starts.
not yet runnable: tests/worker\_terminal.py has no assertion that the screen shows the seed's SHA-256 for human or agent starts; add one in each start path comparing to hashlib.sha256 of the seed file

python3 scripts/full-check.py passes
check: `python3 scripts/full-check.py`

A person opens the worker start screen on a fixture seed and confirms the shown SHA-256 matches sha256sum of the seed file
you judge: Open Approve, press b on a fixture seed, read to the end, and compare the displayed SHA-256 with sha256sum on the seed file; inspect that the line is readable and not wrapped confusingly.

## Affected users and systems
People and agents who start a worker from Approve (b to review, F2 to start). The code path is src/menu/worker.rs, Panel::review (lines 46-100). The disclosure goes into one string that both human and agent starts display. Gate::disclosure in src/worker/authorization.rs only appends agent-only relay text. The SeedTarget.sha256 field in src/seed_me/ready.rs is the likely source. Tests in tests/worker_terminal.py use real keys through a pty with a fake SDLC. The reading at src/menu/worker.rs:397 and :410 shows where the disclosure is rendered; the existing-run branch has no seed target, so it likely shows no hash.

## Constraints and boundaries
Must not change binding rules (the relay-to-seed/model/cap comparison in Gate::bind), start keys, recorded files (worker-authorization.jsonl and run files), or any screen other than Approve F2 and the worker screen. Zero model calls in tests. Builder rules from the operator's standing rule: fewer, deeper modules and smaller interfaces; check data once where it comes in; write one-off logic where it is used rather than as a separate function; skip try/catch where possible. No rejected alternatives were recorded.

## Decisions (from the ledger)
- goal [delegated] The worker start disclosure shows the seed SHA-256, for people and agents who start a worker. Why: so a person can confirm the shown SHA-256 matches sha256sum of the seed the worker will import. Done when: The worker start disclosure displays the seed's SHA-256 for human starts; the worker start disclosure displays the seed's SHA-256 for agent starts. Must not: change binding rules. Must not: change start keys. Must not: change recorded files. Must not: change other screens beyond Approve F2 and the worker screen. (source: Goal confirmed in Tinkery with typed confirm and Enter; caller-declared attribution: declared: agent; typed confirm at this terminal; declaration is not verified identity; goal only, not seed or implementation approval.)
  Delegation (caller-declared, not authenticated): {"agent": "agent"}
- probe-real-sample [delegated] Source: Operator's relayed line, #840, and the supervisor's standing rules for this run. No sample beyond the fixture seed named in #840 Taste. Scope: the worker start disclosure (Approve F2 and the worker screen) shows the full SHA-256 of the seed the worker will import; unchanged: the binding rules, the start keys, the recorded files and every other screen; zero model calls in tests. Done when: (1) The worker start disclosure shows the seed's full SHA-256, for human and agent starts. (2) A test drives the public worker screen and asserts the shown SHA-256 equals the seed's digest. (3) python3 scripts/full-check.py passes. (4) Taste: a person opens the worker start screen on a fixture seed and confirms the shown SHA-256 matches sha256sum of the seed file. Builder rules (operator's standing rule): Fewer, deeper modules, smaller interfaces. Check data once where it comes in, then just use it. Write one-off logic right where it is used instead of splitting it into a function ahead of time. Skip try/catch wherever you can. (source: The agent answered with these words in Tinkery Shape.; caller-declared declared: agent; typed confirm at this terminal; declaration is not verified identity)
- probe-pre-mortem [delegated] skip (source: The agent answered with these words in Tinkery Shape.; caller-declared declared: agent; typed confirm at this terminal; declaration is not verified identity)
- obs-worker-disclosure-1 [evidence] Panel::review formats the disclosure from model, response cap, cap advice, checkout, retry commit and consent text, then appends gate.disclosure(). No SHA-256 appears. The same string serves human and agent starts. (source: read-only investigation in Tinkery Shape)
- obs-seedtarget-1 [evidence] SeedTarget has a pub(crate) sha256: String field. (source: read-only investigation in Tinkery Shape)
- obs-gate-1 [evidence] Gate::bind compares relay.seed_sha256 to target.sha256 (line 83). Gate::disclosure returns an empty string for non-agent starts and otherwise appends only relay words and attribution. It prints no hash. (source: read-only investigation in Tinkery Shape)
- obs-tests-1 [evidence] Worker screen journeys drive the real binary through a pty (send b'b' to 'Review worker start'), and one already computes hashlib.sha256 of the session seed-contract.md. python-tests.py runs every tests/*.py file. The full check is the AGENTS.md block, run with python3 scripts/full-check.py. (source: read-only investigation in Tinkery Shape)
- inf-blast-1 [evidence] The change most likely touches only src/menu/worker.rs (one format line) and tests/worker_terminal.py (one assertion). The authorization file and Gate::bind stay untouched. Risk: line wrapping or reading-to-end shifts in existing terminal assertions. (source: read-only investigation in Tinkery Shape)
- unk-import-bytes-1 [evidence] This investigation has not established that the imported file bytes match target.sha256; the import step in tink-sdlc was not read. (source: read-only investigation in Tinkery Shape)
- inf-probe-counter-1 [evidence] A retry start uses the same disclosure path; the shown hash should be the seed's, not the checkout's, and agent retry starts are refused by Gate::bind. (source: read-only investigation in Tinkery Shape)
- inf-probe-outside-1 [evidence] An experienced reviewer would ask whether the digest is of raw file bytes. Existing code hashes raw bytes with SHA-256, and the test should compare against hashlib.sha256 of the file. (source: read-only investigation in Tinkery Shape)

## Assumed (not confirmed by you)
- none

## Knowledge map
What we know, with proof (observed 2026-11-11; re-check if src/menu/worker.rs or src/seed_me/ready.rs change): Panel::review builds the disclosure without any hash (src/menu/worker.rs:56-85). SeedTarget has sha256 (src/seed_me/ready.rs:18). Gate::bind compares relay.seed_sha256 with target.sha256 (src/worker/authorization.rs:83). ready.rs:207 verifies bytes against target.sha256. The full check is the block in AGENTS.md:110-126, run by python3 scripts/full-check.py. Python journeys under tests/ run via scripts/python-tests.py.
What we know we don't know: whether the digest at SeedTarget is of exactly the seed bytes the worker will import (ready.rs:207 suggests the bytes are verified against it, but the import step itself was not read); the implementer owns this check by reading the import path. Whether a 64-character line wraps at 100 columns; check by running the screen in the test.
What we have not read: the rest of src/menu/worker.rs (lines 130 onward, including the render at 395-410), src/approve.rs, the tink-sdlc import code, and the other tests in tests/worker_terminal.py.
What could surprise us: Where we did not look: the tink-sdlc import code and any other screen that reuses the disclosure text. Counter-example: a retry start (target.retry is Some) uses the same panel; the hash shown should still be the seed's, and an agent start is refused there already. Outside view: someone who has done this would ask whether the digest shown is of the file bytes or a normalized form; the test must compare with a real sha256 of the file. A signal we were wrong: the shown value differs from sha256sum of the seed file.

## Evidence and uncertainty
Grounded in:
- AGENTS.md
- scripts/python-tests.py
- src/menu/worker.rs
- src/seed_me/ready.rs
- src/worker/authorization.rs
- tests/worker_terminal.py

Provenance unavailable: reported reads do not verify filesystem origins or unrecorded reads. Some recorded findings have no attributable read path.

Checked:
- Caller-declared authorization, not authenticated identity: declared: agent; typed confirm at this terminal; declaration is not verified identity (observed 2026-11-11)
Checked by read-only file tools in /Users/jondev/dev/worktrees/tinkery/seed-sha-disclosure-2:
- src/menu/worker.rs:56-85: Panel::review formats the disclosure from model, response cap, cap advice, checkout, retry commit and consent text, then appends gate.disclosure(). No SHA-256 appears. The same string serves human and agent starts. (observation; scope: src/menu/worker.rs lines 56-85) (observed 2026-11-11)
- src/seed_me/ready.rs:11-19: SeedTarget has a pub(crate) sha256: String field. (observation; scope: src/seed_me/ready.rs lines 11-19) (observed 2026-11-11)
- src/worker/authorization.rs:73-130: Gate::bind compares relay.seed_sha256 to target.sha256 (line 83). Gate::disclosure returns an empty string for non-agent starts and otherwise appends only relay words and attribution. It prints no hash. (observation; scope: src/worker/authorization.rs lines 73-130) (observed 2026-11-11)
- tests/worker_terminal.py:333 and :534; scripts/python-tests.py:37; AGENTS.md:110-126: Worker screen journeys drive the real binary through a pty (send b'b' to 'Review worker start'), and one already computes hashlib.sha256 of the session seed-contract.md. python-tests.py runs every tests/*.py file. The full check is the AGENTS.md block, run with python3 scripts/full-check.py. (observation; scope: tests/worker_terminal.py lines 1-30, 333, 480-560; scripts/python-tests.py lines 1-60; AGENTS.md 99-126) (observed 2026-11-11)
- Observations on Panel::review, SeedTarget, Gate and the tests: The change most likely touches only src/menu/worker.rs (one format line) and tests/worker_terminal.py (one assertion). The authorization file and Gate::bind stay untouched. Risk: line wrapping or reading-to-end shifts in existing terminal assertions. (inference; scope: Blast radius of the change; limits: Does not cover tink-sdlc import code, src/approve.rs beyond the grep hits, or the rendering at 395-410 beyond the grep hit.; supported by: obs-worker-disclosure-1, obs-seedtarget-1, obs-gate-1, obs-tests-1) (observed 2026-11-11)
- src/seed_me/ready.rs:207 and tests/worker_terminal.py:111-119: This investigation has not established that the imported file bytes match target.sha256; the import step in tink-sdlc was not read. (unknown; scope: Whether target.sha256 is the digest of the exact seed bytes the worker imports) (observed 2026-11-11)
- src/menu/worker.rs:56-70 and src/worker/authorization.rs:90-92: A retry start uses the same disclosure path; the shown hash should be the seed's, not the checkout's, and agent retry starts are refused by Gate::bind. (inference; scope: Counter-example probe; limits: Retry-start rendering was not observed, only read in code.; supported by: obs-worker-disclosure-1, obs-gate-1) (observed 2026-11-11)
- tests/worker_terminal.py:111-119 and src/seed_me/ready.rs:207: An experienced reviewer would ask whether the digest is of raw file bytes. Existing code hashes raw bytes with SHA-256, and the test should compare against hashlib.sha256 of the file. (inference; scope: Outside view probe; limits: No operator-only decision raised; the digest definition was not read in the import path.; supported by: obs-seedtarget-1, obs-tests-1) (observed 2026-11-11)

## Suggested first slice
Proposal only, non-binding. From the repository root (working directory .): python3 scripts/full-check.py is the full verification; the first edit is a single added line in the disclosure format string in Panel::review (src/menu/worker.rs) showing target.sha256, with an assertion in tests/worker_terminal.py comparing the screen text to hashlib.sha256 of the seed file.

## Risks and verification
Risks: showing a hash that is not of the bytes the worker imports would mislead the person, so the test compares against a hash computed from the file. The extra line could wrap or push the consent text off the first screen, and the reading-to-end gate (End/PgDn) could shift. Another risk is that a change touches the agent relay block or the recorded authorization file; the Must not lines forbid it, and existing worker_terminal.py admission assertions should still pass. Proposed checks are below; none has been run. Where we did not look: tink-sdlc import code, other screens that might reuse the disclosure text, and the real terminal rendering at 100x30 beyond the test fixture.
Intended red: the new assertion fails on the current code because the screen has no SHA-256 line.

## Open questions and deferrals
None blocking. The exact wording and position of the SHA-256 line is an implementation detail for the agent to decide within the goal. The settled ledger answers carry the operator's goal, constraints and builder rules.

## Owners and next steps
No open blockers. Known and settled: the goal, the Must not limits and the done criteria. Watch: line wrapping at 100 columns; the implementer reads the render at src/menu/worker.rs:395-410 and the test shows it. Nothing is to be read by anyone else. No repeat surprise has been recorded yet.

## Downstream handoff
The session folder is the handoff: seed-contract.md with ledger.json, seed-confirmation.json and seed-checks.json. A consumer checks tinkery seed status --json on the session; handoff.verified is true only while the seed, decisions and check records are exactly what was confirmed. It proves content, not who confirmed it. This is discovery input, not an implementation plan, approved specification or review receipt. Preserve the accepted constraints when deriving plans and surface conflicts rather than replacing decisions.

