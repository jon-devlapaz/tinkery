# The menu

| Key | Place | Scope |
| --- | --- | --- |
| 1 | think | Refine, review and explicitly save a goal |
| 2 | tend | Active-goal graph, needs, history and manual continuation |
| 3 | shape | Read seed status; with model/budget, answer and confirm ([Shape](shape-and-handoff.md#shape)) |
| 4 | approve | Placeholder; no sequence approval reader |
| 5 | watch | Placeholder; no live multi-run view |
| 6 | taste | Placeholder; no end-report/review integration |
| 7 | measure | Read-only installed Substrate ledger report, otherwise unknown; no verified metric promise |

Only selection reveals a description; [menu.rs](../../src/menu.rs) owns copy.
Name purpose without borrowing another place's name. [Tend](shape-and-handoff.md)
owns garden/handoff behavior. Unbuilt places authorize nothing. Goal confirmation,
intake approval and implementation permission remain separate.

The muted key bar `↑↓ move   enter open   ? help   q quit` has one blank row below.
Description/latest receipt sits two blank rows below the list, aligned with place
names. Numbers align with title/bar; Tend's muted count sits by its name only when
[discovery](#session-discovery) is complete and nonzero. Digits/aliases remain usable
without bar clutter; open places/help hide this bar.

Help overlays the upper-right (38×13), leaving choices/description visible at every
supported size. Thin muted border; jade title; bold ink headings/keys; muted action
labels. Blank row below title, aligned key/action columns, separate `Esc Close help`
at bottom, no ambiguous Leave heading, new colours or backgrounds. NO_COLOR keeps
hierarchy. Help blocks underlying digits, Enter, movement and row clicks; paste
still does nothing. Esc closes; ?/F1/corner click toggle; q/Ctrl-C/F10 retain quit
routes. Closing restores selection; other places own their help.

Inside a launch Git repo, title shows muted `tinkery  in <folder>` once. Outside, show nothing:
no creation offer/warning. Find root by `.git` folder/worktree file upward; run no
Git command or read its contents. This names location, not goal subject or permission.

Digits or arrows+Enter open; home `q` quits. [Think's exit guard](think.md#sending-and-leaving)
protects unsaved work. `--home` opens menu; otherwise Think. Installed launcher
passes `--home`.

## Measure failure and recovery

Deliberate open requests one owned ledger read, including launch, collection,
diagnosis and text preparation, off input/render paths. Accepted success preserves
report text/scroll/help. No polling, log, agent or automatic retry.

`Reading ledger…` with `esc back   ? help` keeps navigation available; work and
attention stay unknown. Help/resize never restart/cancel. Leaving invalidates
publication and requests stop; reader stays occupied through cleanup. Reopening
queues/starts nothing: `Finishing the previous read…` becomes
`Previous read stopped. Return to the menu and open Measure to read again.`
Only fresh deliberate open starts again; repeated opens per visit coalesce.

Generation and a 30-second monotonic acceptance deadline from open cover launch/preparation.
At/after deadline refuse success. Late results cannot undo cancellation, reopen
views or move selection. Timeout means unavailable/unknown, never zero activity
or proof of missing installation.

Linux/macOS reader owns a process group, null stdin, separate concurrently drained
nonblocking stdout/stderr. Stop closes owned pipes, sends TERM, then KILL after one
second before reaping the leader (no PID reuse before final group signal). Normal
EOF/leader exit uses the same path: it does not prove helpers finished. Accept the
original report/exit only after cleanup, subject to cancellation/deadline/errors;
normal completion also pays the grace. Coverage is ordinary same-group descendants,
not escaped/privilege-changing ones. Cleanup means child reaped, pipes closed,
worker finished, not signal sent. Signal/collection errors refuse further session
reads. On macOS, EPERM from zombie-only groups counts as stopped only with a
complete bounded snapshot proving all remaining members zombies; unknown/inaccessible
snapshots keep the error. No per-descendant reap/status tracking or universal
termination guarantee; OS-stalled descendants can outlast signals.

Limits:

- Retain at most 16 MiB combined raw stdout/stderr; next byte refuses and cleans up.
  No unbounded queues/spools, no partial-output success; disclose incomplete prefixes.
- During off-thread preparation, displayed report/diagnostic graphemes over 4 KiB
  refuse with a display-limit explanation, never silent truncation/partial success.
  Count UTF-8 after diagnostic escaping; success keeps existing decoding. Ordinary
  Unicode/emoji below limit stays unchanged.
- Raw cap is not total-memory cap: decoding/escaping/caches amplify within bounds.
  Success uses prepared glyph/row offsets, visiting visible glyphs only; decoded
  text stays unchanged. Index uses up to eight bytes per nonzero-width grapheme.
  These limits are policy, not measured ledger timings.

Quit/terminal-loop errors stop through the same owner; restore terminal before
draining. After three seconds pending, one stderr notice says exit time unknown;
wait rather than detach. OS launch/reap stalls may make exceptional shutdown
indefinite; forced termination/fatal runtime failure cannot guarantee cleanup.
Reader panic refuses further reads and reports unconfirmed cleanup. Timeout
help/later refusals/shutdown retain cleanup errors. No force-quit UI.

Failures say `Ledger report unavailable`, an evidence-supported plain cause, and
work/attention unknown. Distinguish missing Python, unavailable module/directory,
other launch failures and nonzero exits. Ambiguous errors get generic launch/exit
explanation, never invented installation diagnoses/zero activity. Ordinary views
omit paths, raw errors and Python details; missing prerequisites are things Tinkery
couldn't find/load. Summary wraps/scrolls above fixed `esc back   ? help`.

F1/? opens failure-specific Right now help. `h` opens How it works: attempted
command, working directory, exit status/full launch error and full stderr within
limits. Empty stderr says no details returned. Warn of private paths/output before
sharing. Visibly escape unsafe controls, bidi formatting and invalid UTF-8, never
interpret/discard; retain ordinary Unicode/newlines. Help reads/writes/uploads nothing.

| Key | Summary | Help | How it works |
| --- | --- | --- | --- |
| F1 / ? | Open help | Close help | Close help |
| h | Nothing | Details | Help |
| Esc | Menu | Close help | Help |
| q | Menu | Menu | Menu |
| ↑↓ / j k / PgUp / PgDn | Scroll summary | Scroll help | Scroll details |
| F10 / Ctrl-C | Quit | Quit | Quit |

Help owns scrolling; layers reset help position, closing restores summary position.
Fixed footer alone holds return/close instructions. Technical footer:
`esc back   h help   ?/F1 close   q menu`. Scroll/quit shortcuts appear once in body.
Resize (including 80×24) reflows content/limits; NO_COLOR retains meaning. Machine-sized
positions/indexes handle beyond 65,535 rows. Cached viewport gives Ratatui a bounded
local offset. Reflow indexes bounded chunks across frames after width/text changes;
`Preparing report…` stays until ready. Never rebuild the full diagnostic per frame.
Split long logical lines at graphemes without losing source; chunk boundaries may
add wraps. Ordinary paragraphs retain Ratatui wrapping. Other places and Measure's
quit routes are unchanged.

[Terminal tests](../../tests) use fake commands/TEST roots at 80×24, 100×30,
160×40 and 320×40 with/without NO_COLOR. Contracts cover failure diagnosis, safe
complete Unicode diagnostics, resize/recovery, no help-triggered reads, fixture
preservation, same-group cleanup before acceptance/shutdown, cancellation/deadline,
panic uncertainty, large-output scrolling and equivalence with Paragraph wrapping.
Reversion probes protect disclosure/wrapping, safe rendering and visible recovery.

## Direction, not built behavior

Future Approve is intake, Watch read-only progress, Taste evidence-based review.
Use session/run/ledger/PR artifacts, not harness self-report. Missing sources stay
unknown. No run-now, skip-review or global auto-merge switch. Future launching
requires separate Seed Me confirmation and externally enforced gates; menu
navigation launches no agents.

## Session discovery

Default session root requires absolute nonempty HOME. Missing/empty/relative HOME
means unavailable: no worker, scan, directory creation or empty-list claim.
Technical help names `--seed-session-root PATH`. Practice/snapshots still work;
live drafting refuses unsafe defaults before any host/helper. Explicit roots work
in practice too; capture relative paths against launch directory, refuse empty
paths, and give saving/discovery the same absolute destination.

`--record-root` selects saved history, not sessions, including without live drafting;
[Tend owns reading](shape-and-handoff.md#think-history).
`--link-root PATH` independently selects needs without `--shape-pi`; default is
`~/.local/share/tinkery/tend` under absolute HOME. Without absolute HOME, linking is
unavailable but discovery works. Capture relative explicit paths; refuse empty ones. Menu and
standalone receipt retain the destination. Configuration creates nothing; Tend
refreshes links on landed sessions and picking, writes only on explicit commit.
See [linking](shape-and-handoff.md#linking-needs).

Paint/count/movement use memory only. One owned read-only scanner starts on first
tick, Think return, deliberate Tend reopen or Shape return to detail. At most one
scan; pending requests coalesce into one follow-up. No watchers/index; rescan means
rescan.

| Read | Meaning | Display |
| --- | --- | --- |
| Not known | Nothing landed/new read running | No count; reading unless a nonempty garden exists |
| Complete | Every record read; missing root means nothing saved | Exact active count if nonzero |
| Incomplete | Some records unreadable | Valid goals under [Tend rules](shape-and-handoff.md#tend-garden-and-detail), quiet unreadable count, no menu count |
| Failed | Unsafe root or listing failed | Unavailable, no count |

Unknown/incomplete/failed never mean zero/no sessions. Stray files/directories
without `ledger.json` aren't sessions or incomplete reads. New reads hide stale
counts and empty-list claims until current results land.

Late results retain selection by session path, never row, and never reopen exited
views. Detail/picking freezes replacements until return. Shape return explicitly
waits for fresh same-goal detail and garden stage together. Receipt `s` waits for a
read capable of containing the save; pre-save reads cannot imply unavailable.
Measure still requires explicit opening and previous cleanup completion.

After three seconds, Tend says `Still reading your sessions.` from read start;
wording only, no extra worker/spinner/timer, navigation/Esc live. A stuck scan owns
the worker for the session: no replacement, count stays hidden, even switching
roots cannot start another read. Timeout replacement would leak threads.

## Measured scope

`scripts/bench-menu.py` is outside the zero-call suite. Its POSIX PTY benchmark
uses a temporary HOME and measures monotonic spawn-to-finished-menu-frame, then
Down-to-finished-selection-frame. Historical warm-cache fixture timings are not
portable guarantees or cold-disk claims; the historical disk label came from
`stat -f`. Deterministic blocked-source tests, not timings, gate responsiveness.
