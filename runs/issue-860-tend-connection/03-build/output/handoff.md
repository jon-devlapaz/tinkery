# Issue 860 build handoff

## Situation

Implemented the bounded Tend connection and explicit goal save. Independent review and PR delivery belong to the supervisor. Do not merge.

Candidate commits are `0fe1caf` and `ac8343d`. Review corrections are committed in `6765119`; passing verification is recorded below.

## Background

The approved first slice discovers one connection between automatically chosen active goals in the current repository. The garden and wiki remain future work. No real provider calls, operator sessions, audio, installs or launcher changes occurred.

## Assessment

- `tend/connection` owns source selection, one tool-free request, exact supporting quotes, latest-result persistence and input freshness. Its panel reviews and edits the proposal.
- `seed_me/connection` owns the typed creation evidence. `think_record::read_origins` preserves the legacy reader and keeps invalid Tend evidence distinct from unknown legacy origin.
- `seed_me/connection_save` owns frozen confirmation, private staging, writer exclusion, fixed identity, exclusive rename and explicit recovery. Existing `Config` publishes the ledger. It does not implement another ledger writer.
- The menu alone opens Shape after a verified explicit save. Shape refuses invalid Tend origin before drafting. Approve removes commands for invalid origin, including retry cases.
- Reopening and dismissal persist. Changed goal input permits an explicit fresh pass only after the prior saved destination verifies.
- Architecture diagnostics 2, 3 and 6 move: callers use one save/origin boundary, publication mechanics remain hidden, and phases retain separate ownership. The architecture tests pass.

## Evidence and limits

The terminal test uses a fake provider and temporary TEST roots. It covers 80×24 and 100×30 with color disabled, automatic sources, one request, review/edit/save, exact words, unchanged sources, reopening, same-destination resume and a fresh pass after save. Additional cases cover invalid quotes, zero budget, unknown origin, no result, fewer than two goals, more than 20 goals, oversized inputs, incomplete reads, duplicate identities, request cancellation, persisted dismissal and six freshness changes. A separate journey leaves an authorized save, verifies completion, and rejects a receipt redirected to another root. Unrelated changes use actual `version` and `revision` fields.

The nine shared saver tests reconstruct a persisted receipt, rebuild owned partial staging, reject unowned or symlinked paths, refuse concurrent writers, inspect publication before checking changed membership, and reject changed outcome or confirmation. Public Shape and Approve tests refuse malformed and removed Tend evidence. Approve first proves the seed is verified ready.

The terminal tests stop and relaunch real application processes. The post-publication/final-update-loss case deliberately removes only the exploration's final saved field before relaunch. This is a filesystem-state simulation, not an observed power loss. The partial-staging and receipt-only boundaries are reconstructed in Rust tests. Linux exclusive rename is implemented but local execution occurred on macOS; Linux execution remains for CI.

Real discovery usefulness is unobserved. The fake proposal reveals no new human insight. Whether a real connection changes desired work, review effort, and model cost remains unknown. No human acceptance is claimed.

## Reversion probes

The five `reversion-*.log` files record expected test failures with one protection temporarily removed. Every source was restored. Final probes use `tests/test_launch.py` to supply a temporary TEST HOME, preserving only tool caches. Earlier probes inherited shell HOME for their targeted Rust tests, whose fixtures used temporary roots; the final guarded rerun replaces that evidence. The executable probe helper was removed from `runs`, since run evidence is excluded from the source fingerprint.

Exact mutations and commands for review:

1. Consent: in `src/pi.rs`, replace `if used >= limit {` with `if false && used >= limit {`. Build, then call the public PTY fixture `refusal('good', 'run limit of 0 reached', budget=0)` from `tests/tend_connection_terminal.py`. It fails because a request starts despite zero allowance.
2. Originals: in `src/seed_me/connection_save.rs`, replace the `Affirmed` goal `self.origin.goal.clone()` with `"changed words".into()`. Run `cargo +stable test --lib restart_reuses_published_goal_before_rechecking_changed_membership -- --nocapture`. It fails because displayed and saved words differ.
3. Attribution: in `src/shape/step.rs`, replace `connection::read(session, goal)?` with `connection::read(session, goal).ok().flatten()`. Run `cargo +stable test --lib tend_origin_has_honest_history_and_invalid_evidence_is_distinct -- --nocapture`. It fails because invalid origin falls through to drafting and spends allowance.
4. Duplicate handoff: remove the initial `if let Some(receipt) = self.inspect()? { return Ok(receipt); }` from `Pending::resume`. Run the restart test from probe 2. It fails because the already-published session's new membership blocks recovery.

Run each command through `test_launch.run(..., env=test_launch.environment())`. Restore the source before the next probe. A compile error does not count as a successful probe.

5. Privacy: replace staging `.mode(0o700)` with `.mode(0o777)` and receipt `.mode(0o600)` with `.mode(0o666)`. Run `cargo +stable test --lib published_goal_and_receipt_remain_private -- --nocapture` through the TEST launch guard. The saved permissions assertion fails. Restore both modes.

## Failures resolved

The first broad Rust run exposed stricter-schema probing of legacy ledgers. Marker-first validation restored legacy behavior. The first terminal save exposed the shared confirmation validator's Think-only action string. It now accepts the explicit Tend save action. Initial full verification then found old key-bar expectations. The connection key now appears only in the repository scope where it is available. The Approve regression fixture required a fresh decisions block, removal of its old confirmation line, and a matching draft revision before reconfirmation. The supervisor inspected the repeated fixture failure before its final correction.

The supervisor found private-file modes missing in the first candidate. A fake-provider PTY reproduced directory 0755 and receipt 0644. Creation now requests 0700 and 0600, checked after publication and by reversion. The supervisor also found that pending review showed model wording instead of edited confirmation. It now shows the frozen goal. Bracketed paste reaches the editor. Oversized-save refusal retains the previous durable result and the edited draft, and a corrected retry preserves its words without another request.

## Skills

`principle-build-the-lever` led to a repeatable terminal journey and reversion evidence. `principle-prove-it-works` required real process launches and durable file inspection. `writing-for-agents` kept behavior in the existing design document. `unslop` kept the handoff and product text direct.

## Recommendation

Review the bounded implementation and final verification below. Keep real-provider usefulness and acceptance separate from fake-provider mechanics. Open a PR after independent review; do not merge.

## Final candidate and full check

Code commits: `0fe1caf`, `ac8343d`, `6765119`.

The configured full check on `6765119` passed 1120 Rust tests (one ignored), 39 Node tests, and all 84 Python test files. Formatting and Clippy passed. The factory receipt correctly remained awaiting-attestation until done-15 was marked. Final factory verification passed on evidence commit `264a81a` after all 16 attestations. No code changed. The final receipt records 1120 Rust tests passed (one ignored), 39 Node tests passed, all 84 Python files passed, formatting passed and Clippy passed. The receipt is `04-test/output/verification.json`, and its complete command output is `04-test/output/test-log.md`.

Changed product files: `README.md`, `docs/design/architecture.md`, `docs/design/shape-and-handoff.md`; `src/approve.rs`, `src/approve/tests.rs`, `src/menu.rs`, `src/menu/menu_tests.rs`, `src/pi.rs`; `src/seed_me/connection.rs`, `src/seed_me/connection_save.rs`, `src/seed_me/ledger.rs`, `src/seed_me/ledger/storage.rs`, `src/seed_me/mod.rs`, `src/seed_me/save.rs`; `src/shape.rs`, `src/shape/command.rs`, `src/shape/step.rs`, `src/shape/step/typed_tests.rs`; `src/tend/connection.rs`, `src/tend/connection/panel.rs`, `src/tend/connection-prompt.txt`, `src/tend/discovery.rs`, `src/tend/history.rs`, `src/tend/mod.rs`; `src/think_record/confirmation.rs`, `src/think_record/read.rs`, `src/think_record/read/origins.rs`; `tests/goal_confirmation_terminal.py`, `tests/tend_connection_terminal.py`.

Evidence is in this run's `03-build/output` and `04-test/output`. Generated stage rules remain in the worktree's AGENTS.md but are not part of the product commit. Transient run locks remain untracked. No PR or merge was performed by the worker.
