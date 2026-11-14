# Worker report

BLUF: The worker start screen now shows the seed SHA-256. seed-1 is activated. Not run: seed-2 full-check (`python3 scripts/full-check.py`).

## Changes
- `src/menu/worker.rs`: the disclosure starts with `Seed SHA-256: <target.sha256>`.
- `tests/worker_terminal.py`: `sha_review('human')` and `sha_review('agent')` compare the screen with `hashlib.sha256` of `seed-contract.md`.
- `runs/the-worker-start-disclosure-shows-the/seed-checks.json`: seed-1 `activation` added (same cmd, cwd `.`). The proposal is unchanged.

## Observed
- `python3 tests/worker_terminal.py`: all 21 PASS lines, with the change.
- Reversion probe: with `src/menu/worker.rs` reverted, the file fails. The change was restored.
- `cargo fmt --check` and `cargo clippy -D warnings`: clean.

## Unknowns
- seed-2 (full-check): not run. Verify must run it.
- seed-3 (judge): needs a person.
- The brief is the empty template. The seed contract defines the work.
- Skills: no skill changed a decision.
