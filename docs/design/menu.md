# The menu

| Key | Place | Scope |
| --- | --- | --- |
| 1 | think | Refine, review and explicitly save a goal. |
| 2 | tend | Explore active goals, needs, history and manual continuation. |
| 3 | shape | Read seed status; with a model and budget, answer and confirm ([Shape](shape-and-handoff.md#shape)). |
| 4 | approve | Not built. No sequence approval reader exists. |
| 5 | watch | Not built. No live multi-run view exists. |
| 6 | taste | Not built. No end-report or review integration exists. |
| 7 | measure | Read an installed Substrate ledger report, otherwise show unknown; no verified metric is promised. |

Only selection reveals a description; [menu.rs](../../src/menu.rs) owns the copy.
Name the purpose without borrowing another place's name. [Tend](shape-and-handoff.md)
owns the garden and handoff. Unbuilt places authorize nothing. Goal confirmation,
intake approval and implementation permission remain separate.

The muted key bar `↑↓ move   enter open   ? help   q quit` has one blank row below.
The description or latest receipt sits two blank rows below the list, aligned with
place names. Numbers align with the title and bar. Tend's muted count appears by
its name only when [discovery](#session-discovery) is complete and nonzero.
Digits and aliases still work without bar clutter. Open places and help hide this bar.

Help overlays the upper-right (38×13), leaving choices and descriptions visible at
all supported sizes. It has a thin muted border, jade title, bold ink headings and
keys, and muted action labels. A blank row follows the title. Key and action columns
align, with `Esc Close help` separate below. There is no ambiguous Leave heading,
new colour or background. NO_COLOR keeps the hierarchy. Help blocks underlying
digits, Enter, movement and row clicks; paste still does nothing. Esc closes help;
?, F1 or the corner click toggles it. q, Ctrl-C and F10 retain quit routes.
Closing restores selection; other places own their help.

Inside a launch Git repository, the title shows muted `tinkery  in <folder>` once.
Outside, show nothing: no creation offer or warning. Find the root by looking upward
for a `.git` folder or worktree file; run no Git command or read its contents.
This names location, not the goal's subject or permission.

Digits or arrows followed by Enter open a place; home `q` quits.
[Think's exit guard](think.md#sending-and-leaving) protects unsaved work.
`--home` opens the menu; otherwise Think opens. The installed launcher passes `--home`.

## Measure failure and recovery

A deliberate open requests one owned ledger read. Launch, collection, diagnosis
and text preparation run off input and render paths. Accepted success preserves
report text, scrolling and help, above `esc back   ? help`; that help says
`q goes back to the menu`. There is no polling, log, agent or automatic retry.

`Reading ledger…` and `esc back   ? help` keep navigation available; work and
attention stay unknown. Help and resizing never restart or cancel reads. Leaving
invalidates publication and requests stop; the reader stays occupied through cleanup.
Reopening starts or queues nothing: `Finishing the previous read…` becomes
`Previous read stopped. Return to the menu and open Measure to read again.`
Only a fresh deliberate open starts again; repeated opens per visit coalesce.

Each request has a generation and a 30-second monotonic acceptance deadline from
open, including launch and preparation. Refuse success at or after the deadline.
Late results cannot undo cancellation, reopen views or move selection. Timeout
means unavailable and unknown, never zero activity or proof of missing installation.

On Linux and macOS, the reader owns a process group, null stdin and separate,
concurrently drained nonblocking stdout and stderr. Stop closes owned pipes,
sends TERM, then KILL after one second before reaping the leader. This prevents
PID reuse before the final group signal. Normal EOF or leader exit uses the same
path; neither proves helpers finished. Accept the original report or exit only
after cleanup, subject to cancellation, deadline and errors. Normal completion
also pays the grace. Coverage includes ordinary same-group descendants, not those
that escape or change privileges. Cleanup means the child was reaped, pipes closed
and worker finished, not merely signalled. Signal or collection errors refuse
further session reads. On macOS, EPERM counts as stopped only when a complete,
bounded snapshot proves all remaining members are zombies. Unknown or inaccessible
snapshots retain the error. There is no per-descendant reap or status tracking or
universal termination guarantee; OS-stalled descendants can outlast signals.

Limits:

- Retain at most 16 MiB combined raw stdout and stderr; the next byte refuses the
  report and starts cleanup. Use bounded queues and no disk spools. Partial output
  never succeeds. Disclose incomplete prefixes.
- During off-thread preparation, displayed report or diagnostic graphemes over
  4 KiB refuse with a display-limit explanation, never silent truncation or partial
  success. Count UTF-8 after diagnostic escaping; success keeps existing decoding.
  Ordinary Unicode and emoji below the limit stay unchanged.
- The raw cap is not a total-memory cap: decoding, escaping and caches amplify
  within bounds. Success uses prepared glyph and row offsets, visiting visible
  glyphs only; decoded text stays unchanged. The index uses up to eight bytes per
  nonzero-width grapheme. These limits are policy, not measured ledger timings.

Quitting and terminal-loop errors stop through the same owner. Restore the terminal
before draining. After three seconds pending, one stderr notice says exit time is
unknown; wait rather than detach. OS launch or reap stalls can make exceptional
shutdown indefinite. Forced termination or fatal runtime failure cannot guarantee
cleanup. Reader panic refuses further reads and reports unconfirmed cleanup.
Timeout help, later refusals and shutdown retain cleanup errors. Add no force-quit UI.

Failures say `Ledger report unavailable`, give an evidence-supported plain cause,
and say work and attention are unknown. Distinguish missing Python, unavailable
modules or directories, other launch failures and nonzero exits. Ambiguous errors
get a generic launch or exit explanation, never invented installation diagnoses
or zero activity. Ordinary views omit paths, raw errors and Python details.
Missing prerequisites are things Tinkery couldn't find or load. The summary wraps
and scrolls above fixed `esc back   ? help`.

F1 or ? opens failure-specific Right now help. `h` opens How it works: the attempted
command, working directory, exit status or full launch error, and full stderr within
limits. Empty stderr says no details were returned. Warn that paths and output may
be private before sharing. Visibly escape unsafe controls, bidi formatting and
invalid UTF-8; never interpret or discard them. Retain ordinary Unicode and newlines.
Opening help starts no read, writes nothing and uploads nothing.

| Key | Summary | Help | How it works |
| --- | --- | --- | --- |
| F1 or ? | Open help | Close help | Close help |
| h | Nothing | Details | Help |
| Esc | Menu | Close help | Help |
| q | Menu | Menu | Menu |
| ↑↓, j k, PgUp or PgDn | Scroll summary | Scroll help | Scroll details |
| F10 or Ctrl-C | Quit | Quit | Quit |

Help owns scrolling. Switching layers resets help position; closing restores the
summary position. Only the fixed footer holds return and close instructions.
The technical footer is `esc back   h help   ?/F1 close   q menu`. Scroll and quit
shortcuts appear once in the body. Resize, including 80×24, reflows content and
limits; NO_COLOR retains meaning. Machine-sized positions and indexes handle more
than 65,535 rows. The cached viewport gives Ratatui a bounded local offset. Reflow
indexes bounded chunks across frames after width or text changes; `Preparing report…`
stays until ready. Never rebuild the full diagnostic per frame. Split long logical
lines at graphemes without losing source; chunk boundaries may add wraps. Ordinary
paragraphs retain Ratatui wrapping. Other places and Measure's quit routes are unchanged.

[Terminal tests](../../tests) cover these contracts with fake commands and TEST roots
at 80×24, 100×30, 160×40 and 320×40, with and without NO_COLOR. Reversion probes
protect disclosure, wrapping, safe rendering and visible recovery.

## Direction, not built behavior

Future Approve handles intake, Watch shows read-only progress, and Taste supports
evidence-based review. Use session, run, ledger and PR artifacts, not harness
self-report. Missing sources stay unknown. Add no run-now, skip-review or global
auto-merge switch. Future launching requires separate Seed Me confirmation and
externally enforced gates; menu navigation launches no agents.

## Session discovery

Default roots require absolute, nonempty HOME. Otherwise discovery is unavailable:
no worker, scan, directory creation or empty-list claim.
Technical help names `--seed-session-root PATH`. Practice and snapshots still work;
live drafting refuses unsafe defaults before any host or helper. Explicit roots
also work in practice. Capture relative paths against the launch directory, refuse
empty paths, and give saving and discovery the same absolute destination.

`--record-root` selects saved history, not sessions, even without live drafting;
[Tend owns reading](shape-and-handoff.md#think-history). `--link-root PATH` selects
needs without `--shape-pi`. Its default is `~/.local/share/tinkery/tend` under absolute
HOME. Without absolute HOME, linking is unavailable but discovery works. Capture
relative explicit paths and refuse empty ones. The menu and standalone receipt
retain the destination. Configuration creates nothing. Tend refreshes links when
sessions land and picking starts; only explicit commits write them.
See [linking](shape-and-handoff.md#linking-needs).

Painting, counts and movement use memory only. One owned read-only scanner starts
on the first tick, Think return, deliberate Tend reopen or Shape return to detail.
At most one scan runs; pending requests coalesce into one follow-up. There are no
watchers or index; a rescan is a new scan.

| Read | Meaning | Display |
| --- | --- | --- |
| Not known | Nothing landed, or a new read is running. | Hide the count; show reading unless a nonempty garden exists. |
| Complete | Every record was read; a missing root means nothing was saved. | Show the exact active count if nonzero. |
| Incomplete | Some records are unreadable. | Show valid goals under [Tend rules](shape-and-handoff.md#tend-garden-and-detail) and a quiet unreadable count, but no menu count. |
| Failed | The root is unsafe or listing failed. | Show unavailable, without a count. |

Unknown, incomplete or failed reads never mean zero or no sessions. Stray files
and directories without `ledger.json` aren't sessions or incomplete reads.
New reads hide stale counts and empty-list claims until current results land.

Late results retain selection by session path, never row, and never reopen exited
views. Detail and picking freeze replacements until return. Shape return waits
for fresh same-goal detail and garden stage together. Receipt `s` waits for a read
capable of containing the save; pre-save reads cannot imply unavailable.
Measure still requires explicit opening and completed cleanup of the previous read.

After three seconds from read start, Tend says `Still reading your sessions.`
Only the wording changes; no extra worker, spinner or timer starts. Navigation and
Esc stay live. A stuck scan occupies the worker for the session. No replacement
starts, the count stays hidden, and switching roots cannot start another read.
Timeout replacement would leak threads.

## Measured scope

`scripts/bench-menu.py` owns a POSIX PTY benchmark with temporary HOME, outside
the zero-call suite. It measures from monotonic spawn to the finished menu frame,
then from Down to the finished selection frame. Warm-cache timings are not portable
guarantees or cold-disk claims; the disk label came from `stat -f`.
Deterministic blocked-source tests, not timings, gate responsiveness.
