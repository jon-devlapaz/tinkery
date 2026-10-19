# Watch

Watch shows the runs in one tink-sdlc checkout, read-only, and puts the runs waiting on a
person first. It is a view, not a touch: it decides nothing, starts nothing and changes nothing.

## Source

The operator names the checkout with `--sdlc-root PATH`. Watch never discovers or scans for
checkouts: tink-sdlc's guide says opening a checkout trusts its executable code, so only an
explicitly named one is read. Without a root, Watch says how to name one and runs nothing. A
root without `_system/scripts/sdlc.py` is refused by a file check, without running anything.

Watch reads with that checkout's own `python3 _system/scripts/sdlc.py status --json`, from the
checkout's folder, when Watch opens and when the operator presses `r`. Enter on a run reads that
run's `status <slug> --json`. The screen names the checkout and the exact command behind what it
shows, with how long ago it was read, to the minute. Watch never runs `verify`, `mark`, `decide`,
`stage` or any other command, and never polls, refreshes by itself, notifies or badges anything:
unchanged state makes no new demand on the operator's attention.

## Reading

Each read runs off the paint path with a 30-second deadline, at most 4 MiB of output and its
own process group. Leaving Watch, or closing a run's detail, stops and reaps the read; a late
result is never shown. A refused read shows tink-sdlc's own `error.message`.

Watch reads only tink-sdlc API 1: `protocol` must be `tink-sdlc` and `api_version` the integer 1.
Any other interface, an unknown `verification_status` or gate state, or a run missing a field
fails the whole read closed, with what was wrong, rather than display a guess. An agent saying
it is done is never evidence; Watch shows only what tink-sdlc computes.

## Needs you first

A run needs a person when tink-sdlc reports it:

| `verification_status` | Shown as | Why it needs a person |
| --- | --- | --- |
| `blocked` | waiting on a decision | tink-sdlc reports `blocked` only while a gate isn't approved |
| `failed` | verification failed | someone has to look at the failure |
| `interrupted` | verification interrupted | the last pass didn't finish |
| `invalid` | can't be read | the run is broken; its reason is shown |

These lead, each with tink-sdlc's `next_action` (or the invalid run's reason). Every other run
(`running`, `not-run`, `current`, `stale`) gets one quiet line with its checks passed. Each group
keeps tink-sdlc's order. Owed attestations aren't in the list summary, so they show only in a
run's detail; the list rule above is the smallest honest one the summary supports.

A run's detail shows its state, next action, each gate, and checklist items passed, proven by a
check and attested, plus tink-sdlc's errors. One run at a time; Esc returns to the list.

## Verification

Unit tests in `src/watch/tests.rs` cover the ordering (a waiting run never sorts below a quiet
one), interface and state refusals, detail counts, the deadline, leaving mid-read (the process
is gone afterwards) and that opening and reading again leave the checkout byte-identical.
[The terminal journey](../../tests/watch_terminal.py) drives the real binary over a fake
`sdlc.py` at 100x30, 80x24 and with NO_COLOR.
