# Think thought capture candidate

## Scope

Fresh menu Think offers F3 Save thought and open Tend. F2 still sends.
Tend t opens Think. Tend v and g retain the reader and unsent-copy path.
The goal graph and #866 display scope are unchanged.

Standalone Think has no garden-return configuration. It displays
`Open --home to save a thought in Tend.` and refuses without writing.
The installed `--home` flow has capture. Independent review must assess this boundary.

## Design

`thought` owns the unchanged repository-bound file format, exact-byte deduplication,
private temporary files and atomic publication. It distinguishes durable new and duplicate
records from uncertain persistence. Duplicate saves also sync the directory before success.
`Tend::Garden` only reads. Think owns explicit capture eligibility and its separate Garden exit.
The save owner holds raw storage separately from goal confirmation, goal review and receipts.

Architecture diagnostics: 1 (phase boundaries) improves through shared storage without phase
imports; 2 (module depth) replaces screen-owned storage with one store; 3 (interface size)
adds one internal capture configuration and an explicit exit; 6 (change coupling) keeps the
reader and graph unchanged while the entry point moves. Seven architecture checks passed.

## Checked

- `cargo +stable build --locked` passed.
- Focused Think suite: 291 passed, one existing ignored (before three new capture tests).
- New capture tests: three passed. They exercise fresh submission, voice exclusion,
  empty and oversized input, missing repository, refused storage, and exact copy-back retention.
- Architecture suite: seven passed.
- Documentation suite: 12 passed.
- Terminal thought journeys passed at 80x24 and 100x30, no colour. They cover restart,
  exact words, duplicate and edited copies, reader navigation, goal promotion, refusal,
  zero/positive configured budgets, and standalone refusal. Providers are local stubs.
- Changed-word reversion probe: replacing stored words with `words.trim()` caused
  `capture_guards((80,24))` to fail. Restored the source and rebuilt.
  Log: `/tmp/issue868-words-probe.log` (local evidence, not required by CI).

## Remaining verification

Supervisor owns the full check and independent review. Provider-call and goal-confirmation
reversion probes are still pending. Do not mark done-10 complete yet.
Uncertain directory-sync failure is handled separately in code but has no injected runtime
failure evidence. Do not claim full done-7 coverage.
The operator real-thought trial is pending. Done-11 must remain pending.
No external provider requests or operator storage were used.

## Skill decisions

`principle-build-the-lever`: the existing PTY journey now verifies the new route and is reusable.
`principle-prove-it-works`: preserve exact copy-back checks and execute a changed-word probe.
`unslop` and `writing-for-agents`: replace old editor instructions at their existing homes.
