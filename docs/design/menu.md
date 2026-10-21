# The menu

| Key | Place | Scope |
| --- | --- | --- |
| 1 | think | Refine, review and explicitly save a goal. |
| 2 | tend | Explore active goals, needs, history and opening Shape. |
| 3 | shape | Read seed status; with a model and budget, answer and confirm ([Shape](shape-and-handoff.md#shape)). |
| 4 | approve | Read verified seeds not yet started and copy one runnable manual intake command ([Approve](approve.md)). |
| 5 | watch | Read runs with their matched goals in one named tink-sdlc checkout, waiting runs first ([Watch](watch.md)). |
| 6 | taste | Not built. No end-report or review integration exists. |
| 7 | measure | Recorded work and attention are unknown; no results source or verified metric is built. |

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

## Measure

Opening shows `Recorded work and attention are unknown.` above `esc back   ? help`.
There is no source read, subprocess, provider call, metric or durable write.
Reopening and resizing start no work. Results and effort sources require separate
approved work in [#155](https://github.com/jon-devlapaz/tinkery/issues/155) and
[#115](https://github.com/jon-devlapaz/tinkery/issues/115).

F1 or ? toggles the existing Right now help, which says `q goes back to the menu`.
Esc closes help, then returns to the menu; q returns from either view with help
closed. F10 or Ctrl-C quits Tinkery. The retired reader's failure and technical
detail layers no longer apply.

[Terminal tests](../../tests) cover unknown, help, reopening, resizing and quit
routes at 80×24, 100×30, 160×40 and 320×40, with and without NO_COLOR. Command traps
and a reversion probe protect the no-launch boundary.

## Direction, not built behavior

Approve prepares manual intake; Watch shows read-only progress. Future Taste supports
evidence-based review. Use session, run, ledger and PR artifacts, not harness
self-report. Missing sources stay unknown. Add no run-now, skip-review or global
auto-merge switch. Future launching requires separate seed confirmation and
externally enforced gates; menu navigation launches no agents.

## Session discovery

Sessions default to `~/.local/share/tinkery/sessions`;
[the one-time move](shape-and-handoff.md#continuation-contract) owns retirement of the old root.
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
