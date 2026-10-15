# The menu

The six places follow the operator's work:

| Key | Place | Today |
| --- | --- | --- |
| 1 | think | Dump, refine, review and explicitly save a goal |
| 2 | tend | Saved-goal garden, needs links and manual continuation |
| 3 | approve | Placeholder; no sequence approval reader |
| 4 | watch | Placeholder; no live multi-run view |
| 5 | taste | Placeholder; no end-report/review integration |
| 6 | measure | Read-only report from a separately installed substrate ledger; otherwise unknown |

Only the selected place reveals its one-line description; [the menu](../../src/menu.rs)
owns the exact copy. Name the purpose without borrowing another place's name.
[Tend and handoff](shape-and-handoff.md) owns garden and continuation behavior;
measure does not promise a verified metric.
Approve's readiness wording grants no implementation permission: intake approval
and implementation permission remain separate gates. The unbuilt places authorize
nothing today. Goal confirmation stays explicit inside think.

Tend's active-session count appears only when [discovery](#session-discovery)
is complete and the count is nonzero.
A muted bottom key bar, with one blank row beneath it, shows the immediate menu actions once:
`↑↓ move   enter open   ? help   q quit`. The selected-place description (or latest
receipt notice) sits two blank rows beneath the six-place list, aligned with the
place names. Numbers align with the title and key bar; tend's muted count sits
near its name. Empty space separates purpose from controls; help and open places
do not show the menu bar. Digits and other existing aliases remain
available without adding them to the bar.

Menu help is a compact upper-right overlay (38 columns × 13 rows), not a replacement
screen. The six choices and selected-place description stay visible at every supported
size. One thin muted border separates the panel. The Help title uses jade to mark
the active panel; headings and shortcut keys use bold ink, while action labels stay
muted. One blank row beneath the title gives the movement heading room. Aligned
key/action columns show movement and quitting without an ambiguous Leave heading;
`Esc Close help` stays separate at the bottom. No new colours or backgrounds;
no-colour mode retains the hierarchy. The ordinary menu key bar is hidden while
help is open, so instructions are not repeated.

While menu help is open, digits, Enter, movement and menu-row clicks cannot act
underneath it. Paste still does nothing. Esc closes; ?/F1 and the corner help click
toggle it; q, Ctrl-C and F10 retain their explicit quit routes. Closing restores the
same selection. Other places own their help behavior.

Digits or arrows plus Enter open a place; home `q` quits. Think's
[exit contract](think.md#sending-and-leaving) guards unsaved work. The binary
opens this menu with `--home`; without it, think opens directly. The separate
installed launcher passes `--home`.

## Measure failure and recovery

Measure still requests the installed ledger report synchronously, only when deliberately
opened. This slice changes failure presentation, not subprocess lifecycle: a stuck read
can still block input until it returns ([#120](https://github.com/jon-devlapaz/tinkery/issues/120)).
Successful reports retain their existing text, scrolling and help behavior.

A failed read says “Ledger report unavailable”, gives a plain-language cause when
supported by the evidence, and says recorded work and attention are unknown. Missing
Python, an unavailable target module, an unavailable directory, other launch failures
and unsuccessful exits stay distinguishable. Ambiguous diagnostics get the generic
launch/exit explanation, not an installation diagnosis or a zero-activity claim.
The ordinary view contains no paths, raw errors or Python implementation details.
Missing prerequisites are described as something Tinkery couldn’t find or load.
Its fixed muted key bar is `esc back   ? help`; complete summary text wraps and
can scroll if needed.

F1 or ? opens failure-specific “Right now” help, not menu instructions. `h` opens
“How it works”: the attempted command, working directory, exit status or complete
launch error, and complete stderr. Empty stderr explicitly says no error details
were returned. Details warn that paths/output may be private before sharing. Unsafe
control/bidirectional formatting characters and invalid UTF-8 bytes are visibly escaped,
not interpreted or silently discarded; ordinary Unicode and line breaks remain readable.
Opening help starts no read, writes no log and uploads nothing.

| Key | Failure summary | Help | How it works |
| --- | --- | --- | --- |
| F1 / ? | Open help | Close help | Close help |
| h | Nothing | Technical details | Return to help |
| Esc | Menu | Close help | Return to help |
| q | Menu | Menu | Menu |
| ↑↓ / j k / PgUp / PgDn | Scroll summary | Scroll help | Scroll details |
| F10 / Ctrl-C | Quit Tinkery | Quit Tinkery | Quit Tinkery |

Help owns its scroll position; it cannot move the hidden summary. Switching help
layers resets help scrolling; closing restores the summary position. Each failure/help
view keeps its return/close instructions only in the fixed footer. Technical help
uses `esc back   h help   ?/F1 close   q menu`, naming back and help separately.
Scroll and quit shortcuts appear once in the help body, not again in the footer.
Wrapped content and scroll limits follow resizing, including 80×24; NO_COLOR
retains meaning. Failure scroll positions and wrapped-row
indexes use machine-sized integers, so diagnostics can extend past 65,535 rows. A
cached viewport indexes source segments and gives Ratatui only a bounded local
offset. Reflow follows width/text changes; logical lines that themselves exceed
Ratatui's row range are segmented at grapheme boundaries, retaining all source text.
Other places' navigation and Measure's existing F10/Ctrl-C quit routes are unchanged.

Measure's [terminal journeys](../../tests) use fake commands and
temporary TEST roots at 80×24, 100×30, 160×40 and 320×40, with and without NO_COLOR.
They cover missing executable/module/directory, failed launch, nonzero exit, empty and
multiline stderr, complete long Unicode diagnostics, safe controls, resize, recovery,
no extra read from help and unchanged source fixtures. Rust tests also cover ambiguous
diagnoses, invalid bytes, wide Unicode, help ownership and quit routes. Large-output
regressions reach the end of 70,000-line diagnostics and a Unicode line wrapping
past 65,535 rows, cross segment boundaries, scroll back and resize. Ordinary wrapped
output is compared against the original Paragraph renderer. Reversion
probes protect disclosure/wrapping, safe rendering and visible working recovery.

## Direction, not built behavior

The menu is a human front door, not a settings maze or an agent control panel.
Future approve is a deliberate intake gate, watch a read-only running-work view,
and taste an evidence-based review point. Use existing session/run/ledger/PR
artifacts, not one harness's self-report. No “run now”, skip-review or global
auto-merge switch. Missing sources stay unknown. Future launch capability needs
a separately confirmed Seed Me seed and externally enforced gates; the current UI
launches no agents.

## Session discovery

Session defaults require an absolute, nonempty `HOME`. Missing, empty or relative
`HOME` leaves discovery unavailable: no worker, scan, directory creation or empty-list
claim. Tend's technical help names `--seed-session-root PATH`. Practice and snapshots
remain usable; live drafting refuses the unsafe default before starting a host or helper.
An explicit session root also works in practice. Relative explicit roots are captured
against the launch directory and an empty one is refused; discovery and saving receive the
same absolute destination.
The separate `--record-root` requirement is unchanged and cannot select a session root.
`--link-root PATH` selects Tend's separate needs store and does not require `--shape-pi`.
Its default is `~/.local/share/tinkery/tend` under an absolute HOME; without one,
linking is unavailable with a reason and session discovery still works. An explicit
relative links root is captured against the launch directory; an empty one is refused.
Both menu entry and standalone think's receipt handoff retain that destination.
Configuring the destination creates nothing. Tend reads links when sessions land
and when `l` starts picking; only an explicit commit writes them. See
[linking needs](shape-and-handoff.md#linking-needs) for keys and refusal wording.

Painting, the count and garden movement read in-memory state only. One owned, read-only
worker scans the sessions root (never more than one at a time; requests made while it
runs coalesce into one follow-up). It starts on the first tick, and again when think
returns to the menu and when tend is deliberately reopened. Nothing watches files or
keeps an index; a rescan is a rescan.

| Read | Meaning | Shown |
| --- | --- | --- |
| Not known | nothing landed yet, or a newer read is running | no count; tend says it is reading, unless it already shows a non-empty garden |
| Complete | every record was read (a missing root counts: nothing saved yet) | exact count of active sessions, if nonzero |
| Incomplete | some records exist but could not be read | the valid goals under [Tend's display rules](shape-and-handoff.md#tend-garden-and-detail), a quiet “N saved records couldn't be read”, no count |
| Failed | no safe root was configured, or the root could not be listed | the unavailable message, no count |

Unknown, incomplete and failed are never shown as zero or “no sessions”. Directories
without a `ledger.json` and stray files are not sessions and do not make a read
incomplete. A count is also hidden while a newer read is running, so a stale number is
not shown after saving a goal. For the same reason an empty list from an older read
is not shown as “no sessions” while a newer read runs; tend says it is reading.

In tend, a late read keeps the selected session by path, never by row, and cannot
reopen an exited view. While a saved session's detail is open or a need is being
picked, nothing is replaced underneath it; the garden catches up afterward. From a receipt, `s` waits for a read that can
contain that session and then opens its detail; a read that began before the save is
not treated as “unavailable”. Measure's subprocess still runs only when measure is
opened.

Once a pending read has run 3 seconds, tend's line becomes “Still reading your
sessions.” This is wording only, measured from when that read started: no new worker,
spinner or timer, and Esc and navigation stay live.

A read that never returns occupies the one worker for the rest of the session. Until it
returns no new read starts, so the count stays hidden and switching to another sessions
root does not read it. A timeout that started a replacement would leak stuck threads, so
there is deliberately none.

## Measured scope

`scripts/bench-menu.py` (not part of the zero-call suite) spawns the release binary in a
POSIX PTY with a temporary `HOME`, from a monotonic clock read immediately before spawn,
to the first finished synchronized frame showing all six places, then one Down arrow to
a finished frame showing the selection moved. One run on a 4-vCPU Intel Xeon @ 2.80 GHz
cloud VM (Linux 6.18, `stat -f` reports ext2/ext3 for an ext4-family disk), 100×30,
warm page cache (fixtures written just before; no cache dropping, so not a cold-disk
claim), 30 fresh processes per row:

| Binary (revision) | Sessions | First frame median / p95 / max (ms) | Input median / p95 / max (ms) |
| --- | --- | --- | --- |
| before (e07f920) | 0 | 3.2 / 4.4 / 4.5 | 0.5 / 0.7 / 1.0 |
| before (e07f920) | 5,000 (585 KB ledgers) | 76.4 / 112.2 / 115.6 | 69.7 / 92.5 / 110.6 |
| after (fb177c4) | 0 | 3.4 / 6.2 / 7.7 | 0.5 / 0.6 / 1.1 |
| after (fb177c4) | 5,000 (585 KB ledgers) | 3.7 / 6.7 / 6.9 | 0.5 / 0.8 / 2.9 |

This meets the under-100 ms target for p95 launch to a usable frame on that machine and
fixture only. It is not a universal guarantee; other disks, much larger roots and
terminals were not measured. The portable gate is the deterministic blocked-source
tests, not these timings.
