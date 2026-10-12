# The menu

The six places are in operator/pipeline order:

| Key | Place | Today |
| --- | --- | --- |
| 1 | think | Dump, refine, review and explicitly save a goal |
| 2 | shape | Saved-goal/seed session list and manual continuation |
| 3 | approve | Placeholder; no sequence approval reader |
| 4 | watch | Placeholder; no live multi-run view |
| 5 | taste | Placeholder; no end-report/review integration |
| 6 | measure | Read-only report from a separately installed substrate ledger; otherwise unknown |

Only selection reveals a one-line description. Shape's count of active sessions appears
only when it is exact and nonzero (see below).
Digits or arrows plus Enter open a place. Help is deliberate. F10 leaves think for
the menu, guarded for unsaved work; home `q` quits. The binary needs `--home`;
without it, it opens think. The separate installed launcher passes `--home`.

## Direction, not built behavior

The menu is a human front door, not a settings maze or an agent control panel.
Future approve is a deliberate intake gate, watch a read-only running-work view,
and taste an evidence-based review point. Use existing session/run/ledger/PR
artifacts, not one harness's self-report. No “run now”, skip-review or global
auto-merge switch. Missing sources stay unknown. Future launch capability needs
approved shape and externally enforced gates; the current UI launches no agents.

## Session discovery

Painting, the count and navigation read in-memory state only. One owned, read-only
worker scans the sessions root (never more than one at a time; requests made while it
runs coalesce into one follow-up). It starts on the first tick, and again when think
returns to the menu and when shape is deliberately reopened. Nothing watches files or
keeps an index; a rescan is a rescan.

| Read | Meaning | Shown |
| --- | --- | --- |
| Not known | nothing landed yet, or a newer read is running | no count; shape says it is reading, unless it already shows a non-empty list |
| Complete | every record was read (a missing root counts: nothing saved yet) | exact count of active sessions, if nonzero |
| Incomplete | some records exist but could not be read | the valid sessions, a quiet “N saved records couldn't be read”, no count |
| Failed | the root could not be listed | the unavailable message, no count |

Unknown, incomplete and failed are never shown as zero or “no sessions”. Directories
without a `ledger.json` and stray files are not sessions and do not make a read
incomplete. A count is also hidden while a newer read is running, so a stale number is
not shown after saving a goal. For the same reason an empty list from an older read
is not shown as “no sessions” while a newer read runs; shape says it is reading.

In shape, a late read keeps the selected session by path, never by row, and cannot
reopen an exited view. While a saved session's detail is open nothing is replaced under
it; the list catches up on going back. From a receipt, `s` waits for a read that can
contain that session and then opens its detail; a read that began before the save is
not treated as “unavailable”. Measure's subprocess still runs only when measure is
opened.

Once a pending read has run 3 seconds, shape's line becomes “Still reading your
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
| this change (fb177c4) | 0 | 3.4 / 6.2 / 7.7 | 0.5 / 0.6 / 1.1 |
| this change (fb177c4) | 5,000 (585 KB ledgers) | 3.7 / 6.7 / 6.9 | 0.5 / 0.8 / 2.9 |

This meets the under-100 ms target for p95 launch to a usable frame on that machine and
fixture only. It is not a universal guarantee; other disks, much larger roots and
terminals were not measured. The portable gate is the deterministic blocked-source
tests, not these timings.

Gate integrations need their own approved issues.
