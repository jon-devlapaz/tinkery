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

Only selection reveals a one-line description; counts appear only when nonzero.
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

The menu still eagerly reads sources; **under 100 ms**, with all items retained
and no loading screen, is [draft 6](backlog.md#6-menu-opens-in-under-100-ms), not a
current performance guarantee. Gate integrations need their own approved issues;
this documentation task does not implement them.
