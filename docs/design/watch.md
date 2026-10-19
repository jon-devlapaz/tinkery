# Watch

Watch shows the runs in one tink-sdlc checkout, read-only, and puts the runs waiting on a
person first. It is a view, not a touch: it decides nothing, starts nothing and changes nothing.

## Source

The operator names the checkout with `--sdlc-root PATH`. Watch never discovers or scans for
checkouts: tink-sdlc's guide says opening a checkout trusts its executable code, so only an
explicitly named one is read. Without a root, Watch says how to name one and runs nothing. A
root without `_system/scripts/sdlc.py` is refused by a file check, without running anything.

Watch reads with that checkout's own `python3 _system/scripts/sdlc.py status --json`, from the
checkout's folder, when Watch opens and when the operator presses `r`. It then reads each valid
run's `status <slug> --json`, serially, before offering the list: the summary cannot identify owed
attestations or gates during a running verification. An unreadable detail refuses the list rather
than claiming that run is quiet. Enter reads the selected run's detail again. The screen names the checkout and the exact command behind what it
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

These lead, as do runs whose detail has any unapproved gate, an owed manual attestation, or an
attestation flagged `needs_recheck`. A non-automatic item whose `status` is not `passed` is owed,
except provisional (`pending: true`) items awaiting activation. A passed attestation with
`needs_recheck` is shown as **may need recheck**, not declared invalid: upstream says to recheck
only if the change affects it. Automatic pending checks alone do not demand a person.

This interpretation follows API 1's `checklist_view` and `status_state` in
[tink-sdlc at cf737351](https://github.com/jon-devlapaz/tink-sdlc/blob/cf7373510937d5d592590cc32da57502600c9ca7/assets/_system/scripts/sdlc.py).
Watch requires the typed `automatic`, `proof`, `status`, `needs_recheck` and `mark` fields;
missing or unfamiliar fields are not quiet. Each group keeps tink-sdlc's order. Needs-you rows
retain tink-sdlc's `next_action` (or the invalid run's reason); other runs get one quiet line.

A run's detail shows its state, next action, gates, checklist counts, individual expectations and
recorded evidence, plus tink-sdlc's errors. ↑↓, PgUp/PgDn and Home/End scroll wrapped content,
including long evidence and error tails. Opening or rereading detail resets to the top. One run
at a time; Esc returns to the list. Scrolling launches nothing.

## Verification

Unit tests in `src/watch/tests.rs` cover the ordering (a waiting run never sorts below a quiet
one), interface and state refusals, detail counts, the deadline, leaving mid-read (the process
is gone afterwards) and that opening and reading again leave the checkout byte-identical.
[The terminal journey](../../tests/watch_terminal.py) drives the real binary over a fake
`sdlc.py` at 100x30, 80x24 and with NO_COLOR, including owed-attestation ordering after successful
verification, every verification state, and scrolling to long evidence/error tails. Stubs prove
mechanics, not operator dogfood.

The change moves architecture diagnostics 3 and 6: Watch hides the multi-read attention rule
behind the same menu interface. Row 7 gains the Watch module map; no cross-phase imports or
relaxed architecture limits are introduced.
