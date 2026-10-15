# Tend and handoff

Tend replaces the shape list after the operator chose Replace on 2026-10-14
([#123](https://github.com/jon-devlapaz/tinkery/issues/123)). [Shape](#shape) returns
as its own place in the first release (operator decision, 2026-10-15). Until it is
built, the Seed Me step happens outside Tinkery through the copied continuation prompt.

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
ambiguous or ended ends draw no line. A straight line leaves a gap around unrelated
rings rather than implying a junction; there is no path routing. The selected line below still names those needs.
There are no known sizes or merges. Seed Me sessions stay read-only; explicit linking
writes only Tend's own links.
Ended sessions are left out; `N ended not shown` shares the muted incomplete-read
note row with `Links couldn't be read. F1 then h for details.` when links are
unavailable, including when there are no active goals. Applicable notes join with
` · ` and remain through navigation. An unreadable-links refusal replaces that
note rather than saying the same thing twice.
If every readable session is ended, tend says `No active goals. N ended not shown.
Confirm a goal in think to start one.` rather than claiming no sessions exist.

Labels use only the first saved-goal line, including old unstructured goals. They
sit above or below their rings, clipped with `…` to avoid overlapping labels. The
selected goal's label gets room first and always shows; colliding others are clipped
or omitted, never interleaved. The selected label starts with `› `; its label and ring
are bold, and jade with colour. Other labels are muted and rings are ink. No label
is italic.

Below the garden, the selected line shows the first goal line in ink, then status
and relative time in muted text, then `needs Quiet garden, Local records · not ready`
or `needs Local records · ready` when it has needs. Unresolved needs say `1 need not found`
(or `N needs not found`); no needs adds nothing. Short needs keep the usual status/time
suffix. If that suffix would crowd out the headline, the footer shows the headline,
readiness and `F1 needs` instead; Help provides the unabridged needs summary with
PgUp/PgDn scrolling. Ambiguous source identity says `readiness unknown` and attributes
no needs to that goal. A muted refusal uses the otherwise blank row between that
line and the key bar.

Right / Down / `j` select the next goal in discovery's newest-first order;
Left / Up / `k` select the previous one, clamping at either end. Up/Down do not
follow screen rows or graph neighbours. Selection pans an offscreen goal into view
without resetting zoom. The garden key bar is
`↑↓←→ move   enter open   l link   esc back`, which fits at 80 columns.
`+` / `=` zoom in, `-` zooms out; uppercase `H J K L` pan left/down/up/right.
A left press on a ring or label selects that exact session path; another press on
the same path within 500 ms opens detail. Drag a ring or label to move the goal,
drag empty space to pan, and scroll to zoom. Help, detail, history and reading/empty/unavailable
states ignore graph input.

Tend places goals in deterministic, identity-ordered slots shaped to the available
canvas, with initial slot spacing of at least 12 columns and 6 rows instead of
squeezing every ring into view. Crowded gardens need selection-following, pan or zoom;
not every label is visible at once. Graf draws the graph without a physics thread.
Moving goals, panning and zooming write nothing. An unchanged graph keeps positions and viewport through
refresh. Changed nodes, labels or links rebuild the static layout, resetting manual
goal positions while retaining zoom and pan; following selection may pan again.
Resizing reflows the slots to the new canvas, retaining relative zoom and pan.

Enter opens the exact stored goal block line by line, using the receipt's shared
renderer without `Saved.`. In detail, ↑↓ / `j k` or PgUp/PgDn scroll; `c` copies the
continuation prompt when the stored stage permits it; `r` opens Think history.
Esc goes detail → garden →
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

## Think history

[#143](https://github.com/jon-devlapaz/tinkery/issues/143) adds one action inside a
saved goal's detail: `r history`. History shows the original words, questions,
answers, skipped/restored questions and exact confirmed goal, in recorded order. When
the record names a repository, `Made in` and its folder name follow; outside one, or
for schema-2 records that predate it, nothing is added. F1 then `h` gives the full
path, `none` or `not recorded`. Labels are muted; the person's words remain in ink.
Technical paths, model information and checks appear only in F1 then `h`.
Display escapes unsafe terminal controls; stored words remain unchanged.

↑↓ / j k, PgUp/PgDn and Home/End scroll. Wrapping follows the terminal width,
including long Unicode history beyond 65,535 rows. Help has its own scroll position.
Esc goes details → help → history → the same saved goal; F10 returns to the menu.
The fixed footer keeps the way back visible. Opening, scrolling and help make no
model calls, copy nothing and write nothing.

`--record-root` selects existing records for reading even without `--shape-pi`.
It defaults to the same folder used for saving. One background reader serves the
menu's lifetime. Requests coalesce while a read is pending; leaving discards its
result. A stuck read keeps that one worker occupied, but never blocks navigation
or starts a replacement worker. Reopening explicitly requests another read.

The shared record reader requires one schema-2 or schema-3 record with the exact canonical
session path, Seed Me session ID and confirmed goal. It rechecks the selected
session after scanning. Duplicate matches refuse selection. A matching older
schema-1 record has no exact connection and remains unavailable; no migration or
text-based guess runs. Unreadable or unsupported JSON could conceal another match,
so it also prevents selection. Each file is limited to 16 MiB; oversized files
are unavailable, never partially displayed. Missing, unreadable and conflicting
history explain the gap while leaving the saved goal intact. Moving sessions or
recording unfinished Think sessions is outside this feature.

## Linking needs

In the garden, `l` starts “this goal needs …”. The source keeps its `›` mark;
movement picks another goal, marked `→`. Enter or one left press on that goal
adds the need, or removes an existing need. Commit preserves the displayed link or
unlink intent even if another instance changed the file. An already satisfied intent
keeps the file intact and says `Already linked.` or `Already unlinked.` Picking the source does nothing.
The key bar becomes `↑↓←→ pick   enter link   esc cancel` (`enter unlink` for an
existing need); the bottom line names the source and picked goal. Before commit,
Esc cancels picking without writing. Commit or cancellation returns selection to the source. Session
refreshes wait until picking ends rather than replacing the goals underneath it. Detail, history, help and
reading states ignore `l`.

Missing or ambiguous identity and unavailable links change nothing. Ordinary
refusals are one muted line: `This goal can't be linked yet.` or
`Links couldn't be read, so nothing changed. F1 then h for details.` A write failure
says `Links couldn't be saved, so nothing changed. F1 then h for details.`
The exact reason appears only in technical help beside the session path and survives
a pending discovery refresh while the refusal is shown. Waiting for another writer's
lock is synchronous and bounded to about two seconds; a timeout uses the save-failure
refusal and technical help says `another Tinkery is saving links`. Esc/F10 queued
during that wait do not cancel the in-flight save: if the lock becomes available
before the deadline, the save can complete before navigation is handled. A timeout
refuses the save outright; releasing the lock later does not retry it.

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
remain unreadable, with a persistent note or a plain refusal; an edit cannot overwrite
them. A requested change to an existing read-only file or a file this process cannot
open for writing is refused without changing bytes or permissions. A matching no-op
needs no write access and preserves the file's exact bytes, permissions and timestamp.

An edit rejects self-links and unreadable stores, then acquires an exclusive lock on a
sibling `needs.lock` within the deadline, across a fresh read, the intended add/remove and atomic replacement through
a same-folder temporary file. The empty lock file stays in the links folder so
cooperating simultaneous writers always lock the same file; navigation creates neither
file. This preserves other successful saves and changes since a previous read, but
does not lock out external tools that ignore the lock.

Readiness is derived from links and all readable sessions, including ended ones. It lists
found needs by their first goal line and counts unresolved ids without guessing. Duplicate
session ids are unresolved too: duplicate source ids yield no attributable needs or
positive readiness. A goal is ready only with at least one need and every need
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

## Shape

**Planned, not built.** The operator put Shape in the first release on 2026-10-15.
Its design below is a proposal awaiting acceptance; it grants no implementation
permission or provider budget.

Think settles what the operator wants; Shape settles how to build it. From a goal's
detail in Tend, a key that says it starts Shape opens the goal's existing Seed Me
session and runs Seed Me's interview there: investigate facts first, then one
decision at a time with its evidence, options and suggestion, ending in a seed the
operator confirms. Journey: think → tend → shape → approve.

- **Seed Me owns the session.** Shape reads and writes it only through Seed Me's
  session helper (`read`, `publish`, `status`, `seed confirm`). Tinkery keeps no copy
  of the ledger or a second contract; Seed Me changes are proposed in its repository.
- **Nothing runs by itself.** Each agent step, including investigating the
  repository, starts from a key that names it and runs within an explicit budget
  (default zero). The copied continuation prompt stays available.
- **Three separate decisions.** Goal confirmation, seed confirmation and approval
  stay distinct on screen; Shape never asks again what Think settled.
- **Honest states.** A failed call, an unknown stage or a seed that no longer matches
  its confirmation (`status`) is said plainly, with one next step.

Open before building: the model host, how Shape avoids writing a session another
tool is working on, and confirming that Seed Me's stale-confirmation fix
([seed-me #3](https://github.com/jon-devlapaz/seed-me/issues/3)) is complete.

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
duplicate-id refusal, derived readiness, cancel/self-pick and add/remove. A matching
read-only no-op regression checks bytes, permissions and timestamp; that preservation
check has not had a dedicated reversion probe. Reversion-probed regressions cover
read-only and deleted-file refusals, 32 rounds of eight concurrent writers, same-edge
stale intents in both orders and concurrent instances, bounded lock refusal with
retained technical details, single unreadable-link wording and empty-view notes.
Readiness regressions cover ambiguous source attribution and long mixed-Unicode
needs: source and readiness stay visible at 80×24, 100×30 and 160×40, while Help
retains the full titles. Graph probes cover straight-line
gaps in the three-goal repro and seeded larger cases, minimum ring spacing for 40
short goals (unlinked and chained) at 80×24, 100×30, 160×40 and 320×40, and every
selected label without overlapping cells for 40 short goals at 40×12 and 60 long
mixed-Unicode goals at 100×30 and 160×40. The older full-fit label-count expectation
is replaced by spacing and navigable identity, not a claim that every label fits.
The F10 detail footer remains covered. Discovery,
help, copying and receipt tests retain their state and handoff guards.

PTY journeys show keyboard and mouse linking, restart persistence, keyboard and mouse
unlinking, and a needs line between two rings at 80×24, 100×30 and 160×40. Selection,
click/double-click, keyboard zoom/pan, drag and scroll also run at 320×40, alongside
exact detail text, repeatable prompt bytes and exit routes. A 30-goal journey checks
selection without paging. A held-lock journey keeps the lock past the deadline and
checks timeout refusal, technical help, queued Esc/F10 exits and no save after later
release. It does not establish cancellation before the deadline. All use stubs and
temporary TEST roots, make no provider calls and check Seed Me fixture bytes stay
unchanged. These prove mechanics, not
operator acceptance or goal quality.
