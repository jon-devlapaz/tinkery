# Tend and handoff

Tend replaces the shape list after the operator chose Replace on 2026-10-14
([#123](https://github.com/jon-devlapaz/tinkery/issues/123)). Shape returns later as
the Seed Me step, turning a confirmed goal into a seed; until then that step happens
outside Tinkery through the copied continuation prompt.

## Receipt

Successful explicit goal saving shows `Saved.`, the exact frozen goal block and
one next step: `Next: shape it into a seed with Seed Me.`

- `c copy prompt` sends OSC 52 with quiet write success/failure feedback and can
  be retried. This does not verify the host clipboard; see
  [clipboard support](../terminal-setup.md#clipboard-osc-52).
- `s open in tend` opens **this session's detail**, not the garden.
- F10 returns to the menu. Standalone think also transitions into the menu.

Nothing launches, opens a browser, pastes into a harness or starts implementation.
Storage paths live in technical help (F1, then `h`), not the ordinary receipt or
detail. Help shows the [think record](think.md#think-record)'s path under Seed Me's.
If the record wasn't written, help and the receipt's muted notice say so; the goal
stays saved. A notice wraps whole rather than clipping, in the receipt and tend's
detail. The plain-text terminal exit receipt may include recovery locations.

## Tend: garden and detail

Tend shows all discovered active saved goals as one graph, with no paging or node cap. Every goal has the same hollow Braille ring: size is unknown.
Quiet lines join needs whose two ends are active and uniquely identified; missing,
ambiguous or ended ends draw no line. The selected line below still names those needs.
There are no known sizes or merges. Seed Me sessions stay read-only; explicit linking
writes only Tend's own links.
Ended sessions are left out; `N ended not shown` shares the muted incomplete-read
note row with `Links couldn't be read. F1 then h for details.` when links are
unavailable. Applicable notes join with ` · ` and remain through navigation.
If every readable session is ended, tend says `No active goals. N ended not shown.
Confirm a goal in think to start one.` rather than claiming no sessions exist.

Labels use only the first saved-goal line, including old unstructured goals. They
sit above or below their rings, clipped with `…` to avoid overlapping labels; a label
with no room is omitted. The selected label starts with `› `; its label and ring
are bold, and jade with colour. Other labels are muted and rings are ink. No label
is italic.

Below the garden, the selected line shows the first goal line in ink, then status
and relative time in muted text, then `needs Quiet garden, Local records · not ready`
or `needs Local records · ready` when it has needs. Unresolved needs say `1 need not found`
(or `N needs not found`); no needs adds nothing. The headline truncates before the suffix.
A muted refusal uses the otherwise blank row between that line and the key bar.

Right / Down / `j` select the next goal in discovery's newest-first order;
Left / Up / `k` select the previous one, clamping at either end. Up/Down do not
follow screen rows or graph neighbours. Selection pans an offscreen goal into view
without resetting zoom. The garden key bar is
`↑↓←→ move   enter open   l link   esc back`, which fits at 80 columns.
`+` / `=` zoom in, `-` zooms out; uppercase `H J K L` pan left/down/up/right.
A left press on a ring or label selects that exact session path; another press on
the same path within 500 ms opens detail. Drag a ring or label to move the goal,
drag empty space to pan, and scroll to zoom. Help, detail and reading/empty/unavailable
states ignore graph input.

Tend places goals in deterministic, identity-ordered slots shaped to the available
canvas; Graf draws the graph without a physics thread. Moving goals, panning and
zooming write nothing. An unchanged graph keeps positions and viewport through
refresh. Changed nodes, labels or links rebuild the static layout, resetting manual
goal positions while retaining zoom and pan; following selection may pan again.
Resizing reflows the slots to the new canvas, retaining relative zoom and pan.

Enter opens the exact stored goal block line by line, using the receipt's shared
renderer without `Saved.`. In detail, ↑↓ / `j k` or PgUp/PgDn scroll; `c` copies the
continuation prompt when the stored stage permits it. Esc goes detail → garden →
menu; F10 goes to menu. F1 opens help; `h` shows technical details.

| Stored state | Label | Detail next step |
| --- | --- | --- |
| Confirmed goal | goal saved | Shape it into a seed with Seed Me |
| Draft seed contract | seed drafted | Confirm the seed draft with Seed Me |
| Seed confirmed for intake | seed ready | The seed is ready for intake |
| Ended session | ended | None; copying is unavailable |

Unknown and practice stages remain distinct, never mislabeled ready. The menu's
[discovery contract](menu.md#session-discovery) owns refresh and completeness rules.
While waiting for a read with nothing to show, tend says it is reading or still
reading. A failed read says the saved session is unavailable and directs the reader
to F1 then `h`; only a complete, current empty read says there are no sessions yet.
Incomplete reads keep valid goals and put the unreadable-record count above the
selected line, warning that the view may be missing sessions. If none can be read,
tend says nothing can be listed, not “no sessions”. Reading, unavailable and empty
states offer only `esc back`.

## Linking needs

In the garden, `l` starts “this goal needs …”. The source keeps its `›` mark;
movement picks another goal, marked `→`. Enter or one left press on that goal
adds the need, or removes an existing need. Picking the source does nothing.
The key bar becomes `↑↓←→ pick   enter link   esc cancel` (`enter unlink` for an
existing need); the bottom line names the source and picked goal. Esc cancels
without writing. Commit or cancellation returns selection to the source. Session
refreshes wait until picking ends rather than replacing the goals underneath it. Detail, help and
reading states ignore `l`.

Missing or ambiguous identity and unavailable links change nothing. Ordinary
refusals are one muted line: `This goal can't be linked yet.` or
`Links couldn't be read, so nothing changed. F1 then h for details.` A write failure
says `Links couldn't be saved, so nothing changed. F1 then h for details.`
The exact reason appears only in technical help beside the session path.

## Needs store

Tend owns a separate `needs.json` under `--link-root PATH` (default:
`~/.local/share/tinkery/tend`, requiring an absolute HOME). It never writes Seed Me
files. Navigation and copying create no links. Landed session reads refresh the
in-memory links; starting a pick reads them again. Painting uses only that cached result.

The version-1 format is `{"version":1,"needs":[{"goal":"id","needs":"id"}]}`.
Links use only the ledger's nonempty, control-free `session_id`, never a title or
folder name. A missing or invalid id leaves the session readable but unlinkable.
Entries are sorted by goal id then needed id, so equivalent sets write identical bytes.

A missing file is normally empty. If this view last read a nonempty link set and the
file disappears, reads and saves refuse rather than recreating it and losing known
links. Bad JSON, unsupported versions, invalid or duplicate entries and IO errors
remain unreadable, with a persistent note and a plain refusal; a toggle cannot overwrite
them. Existing read-only files or files this process cannot open for writing are
refused without changing their bytes or permissions.

A toggle rejects self-links and unreadable stores, then holds an exclusive lock on a
sibling `needs.lock` across a fresh read, the add/remove and atomic replacement through
a same-folder temporary file. The empty lock file stays in the links folder so
cooperating simultaneous writers always lock the same file; navigation creates neither
file. This preserves other successful saves and changes since a previous read, but
does not lock out external tools that ignore the lock.

Readiness is derived from links and all readable sessions, including ended ones. It lists
found needs by their first goal line and counts unresolved ids without guessing. Duplicate
session ids are unresolved too. A goal is ready only with at least one need and every need
uniquely found and ended. No needs is not ready; unknown stages or missing needs are not ready.

## Continuation contract

One formatter owns the receipt/detail prompt. It names the skill and **existing**
session, requests `session.py read` and `status` before mutation, preserves the
confirmed goal and decisions/context, and stops if the session is inactive.
It asks Seed Me to draft `seed-contract.md` in that session and obtain explicit
confirmation of the displayed seed revision. It does not initialize a replacement
session or grant implementation permission. An operator manually pastes it into
an authorized harness. The prompt needs paths, even though the ordinary UI does not.

Confirmation uses the official Seed Me helpers, not a second schema. Tinkery captures
and checks the session destination before any helper access, always passes `init --root`,
and retains that destination for retries and fresh dumps. Seed Me never chooses a second
implicit HOME default. See [session discovery](menu.md#session-discovery) for root selection.
Recovery
errors remain explicit; tests exercise helpers only under temporary roots. The
Seed Me fixture is external and pinned by CI, not vendored or edited here.

## Rationale and verification

[#123](https://github.com/jon-devlapaz/tinkery/issues/123) records the calm-view
research: few nodes and few link types; a force-directed whole-graph view was mostly
decorative. [#127](https://github.com/jon-devlapaz/tinkery/issues/127) uses a static
graph so needs are visible without automatic motion. This is a design choice,
not a comparative user-study claim.

Rust regressions cover deterministic layout and overlapping-ring paint order, hollow
rings, needs lines, palette and no-colour selection weight, no physics worker, uncapped
goals, clamped selection, pointer identity, double-click timing, drag/zoom/pan and refresh
preservation. Store and linking tests cover unreadable-byte preservation, exact ids,
duplicate-id refusal, derived readiness, cancel/self-pick and add/remove. Reversion-probed
regressions cover read-only and deleted-file refusals, 32 rounds of eight concurrent
writers, persistent unreadable-link notes, the F10 detail footer and distinct labels
for 40 short goals (unlinked and chained) at 80×24, 100×30, 160×40 and 320×40. Discovery,
help, copying and receipt tests retain their state and handoff guards.

PTY journeys show keyboard and mouse linking, restart persistence, keyboard and mouse
unlinking, and a needs line between two rings at 80×24, 100×30 and 160×40. Selection,
click/double-click, keyboard zoom/pan, drag and scroll also run at 320×40, alongside
exact detail text, repeatable prompt bytes and exit routes. A 30-goal journey checks
selection without paging. All use stubs and temporary TEST roots, make no provider
calls and check Seed Me fixture bytes stay unchanged. These prove mechanics, not
operator acceptance or goal quality.
