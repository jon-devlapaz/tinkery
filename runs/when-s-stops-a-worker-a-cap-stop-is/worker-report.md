# Worker report: #813 s records cap only with the marker

## Changes (attempted, observed)
- `src/worker.rs`: `Action::StopOperator` and `StopCap` became one `Action::Stop`. In `act()`, `--stopped-by cap` is sent only if the newest `build.{launch,resume}-<ts>.pi.log` has a line starting `pi-launcher: response cap N reached;` (N from `steps/build.json` `max_responses`) and assistant `message_end` events across all build logs are >= N. Missing or unreadable data gives `operator`. No new pub item.
- `src/menu/worker.rs`: `c` key, key bar entry and help text removed; help now explains the rule.
- `src/worker/tests.rs`: Rust test uses `Action::Stop`; the cap case moved to Python.
- `tests/worker_terminal.py`: `cap` journey presses `s` over a marker fixture log (`write_logs` helper).
- New `tests/worker_stop_marker_terminal.py`: marker -> cap, no marker -> operator, marker only in an older log -> operator, `c` does nothing and is not on the screen or in help.
- `docs/first-task.md`, `docs/design/approve.md`: `c` mention replaced.

## Evidence
- Observed: new test prints 4 PASS. Probe: with the cap branch forced off, the marker case failed (stopped_by operator); then restored.
- Observed: `cargo fmt --check` and clippy clean; `cargo test worker` 34 passed; full-check Rust tests passed and its build and snapshot steps ran.
- Failed: `scripts/full-check.py` Python step, 5 files: first_task_path, test_first_task_check, taste_label_terminal, taste_verdict_head_terminal, taste_retry_terminal. They check Pi login and taste labels, not the worker. Baseline observed: the same 5 files also fail with my src/docs/tests changes stashed, so they are not caused by this change (likely this machine's Pi login state). Full check is NOT green. Node and the fmt/clippy steps of the block were not reached in that run; fmt and clippy were run alone afterwards.
- Unknown: real launcher logs may differ from fixtures. When a step has several logs, the marker must be in the newest one; responses count across all.

## Activation
seed-2 and seed-3 got an `activation` (proposed cmd, cwd `.`) in `seed-checks.json`. Not proof: verify must run them.

## Not done
No commit made (not asked in this task). Skills: none changed a decision. Architecture diagnostics: one fewer Action variant and one fewer key; logic is local to `act()`.
