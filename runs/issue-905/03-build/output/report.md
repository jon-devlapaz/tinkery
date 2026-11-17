# Approve fixture-reuse trial

## Decision

The preregistered 20% standalone journey target was not met.
Recommend no-keep under that criterion. Preserve the candidate in an unmerged draft PR for independent human review.
Do not merge or record operator acceptance from these results.

One successful baseline took 199.691 seconds. Three candidate runs took 175.207, 174.013 and 173.541 seconds.
The candidate median was 174.013 seconds, 12.859% shorter than the single successful baseline.
Only one baseline succeeded within the approved three-attempt limit. This is limited local evidence, not a robust speedup estimate.

## Change

`tests/approve_terminal.py` generates the ordinary and tall-goal fixture families once per process through the existing real Rust ledger helpers.
Each journey copies the selected family into its own fresh TEST root using `shutil.copytree`.
Session directory names, ledger identities, confirmation receipts, stale seeds, drafts and question states remain intact.
Case-specific mutations still happen after copying. Each case keeps its own application process and writable files.

`tests/test_approve_fixtures.py` adds three focused tests and a rerunnable benchmark.
It proves the real ledger states survive copying, templates are reused, destinations must be fresh, and copy mutation/deletion cannot affect later copies.

Production source, shared fixture helpers, CI routing, tiers, worker counts and batching are unchanged.
No production architecture diagnostic changed. Fixture generation has one test-local owner.
The generated stage discipline block in the worktree's AGENTS.md is runtime setup, not part of the proposed code change.

## All benchmark attempts

| Attempt | Session | Result | Seconds |
| --- | --- | --- | ---: |
| Baseline 1 | Inherited controlling terminal | Failed | 3.289 |
| Baseline 2, diagnostic | Inherited controlling terminal | Failed | 3.239 |
| Baseline 3 | Detached | Passed | 199.691 |
| Candidate 1 | Detached | Passed | 175.207 |
| Candidate 2 | Detached | Passed | 174.013 |
| Candidate 3 | Detached | Passed | 173.541 |

The raw diagnostic frame drew through row 120 at width 200 while the journey expected 80x24.
The application used Herdr's controlling terminal instead of the test PTY.
The benchmark now starts its journey subprocess in a new session. No assertion, wait or production code changed to address this environment error.
All failures remain in the evidence. No additional performance attempts were run after the approved budget was used.

Environment: macOS 27.0.1, Apple Silicon, Python 3.13.12, 15 logical CPUs, wait scale 1, serial standalone journeys with a warm build.
Rust was 1.98.0 and Node was 26.8.1. No toolchain was changed.
The benchmark excludes its preparatory build from the journey duration.
The revisions were not interleaved. Competing system work was not measured.
These observations establish neither fleet-wide CI savings, billed cost, active human effort, useful outcomes nor acceptance.

[Comparison and limits](comparison.json) links every attempt record and raw log.
Both baseline and candidate records retain the tested journey's SHA-256, because the candidate was still uncommitted during the trial.

## Protection checks

- AST comparison confirms all 54 original assertions, all 68 journey variants and every post-setup journey statement are unchanged.
- The matrix retains 80x24 and 100x30, color and NO_COLOR, plus the existing targeted identity and checkout cases.
- Replacing `copytree` with a symlink made the isolation test fail at the same-file assertion for both template families. Restoring the copy passed all three focused tests.
- Removing only the production different-repository command refusal made the original 100x30 NO_COLOR journey fail at its unchanged `c copy command` assertion.
- Restoring production source and rebuilding made that journey pass. The production diff is empty.

Evidence: [contract comparison](contract-comparison.json), [isolation fault](isolation-probe.log),
[production fault](production-probe-broken.log), [production restoration](production-probe-restored.log).

## Verification

The local full check passed on the candidate's unchanged source before its code commit:

- Rust: 1,137 passed, zero failed, one existing ignored test.
- Node: 39 passed, zero failed. Voice engines and recorders were fake.
- Python: all 90 files passed, including the full tier and three new fixture tests.
- Rust formatting and Clippy passed.
- Approve's 68-variant journey passed in the full run at 172.4 seconds. Its focused fixture file passed at 2.2 seconds.

[Local full-check output](local-full-check.log) and [counts](local-check-counts.json) retain the results.
Committed-revision SDLC verification follows the code commit and owns the final receipt.
CI and independent human review remain separate. No real provider, microphone or operator session was used.

## Rerun

From an isolated checkout with the current documented prerequisites:

```bash
python3 tests/test_approve_fixtures.py -v
python3 tests/test_approve_fixtures.py --benchmark /tmp/tinkery-TEST-approve-trial.json --runs 3
python3 scripts/full-check.py
```

The benchmark command starts new work. Run it only under a new explicit trial allowance; this trial's budget is exhausted.
For a baseline comparison, use the original journey revision with this benchmark driver in a separate isolated checkout.

## Handoff

The requested delivery is a draft PR with evidence, not a merge or acceptance claim.
An independent human can review the protections and decide whether to reject the candidate or explicitly revise the performance criterion.
No automatic continuation, extra trial, delegated agent, model call or installation is authorized.

Skills that changed decisions: `principle-build-the-lever` led to the committed benchmark and isolation checks.
`principle-prove-it-works` led to real-ledger state checks and the production-only fault probe.
`features-and-futures` led to stopping after this bounded slice rather than starting broader test cleanup or selection work.
