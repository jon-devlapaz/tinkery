# Tend and handoff

Tend replaces the shape list after the operator chose Replace on 2026-10-14
([#123](https://github.com/jon-devlapaz/tinkery/issues/123)). [Shape](#shape) returns
as its own place in the first release (operator decision, 2026-10-15). It reads where a
seed stands and, with a model and a budget, runs the Seed Me interview to a confirmed seed;
the copied continuation prompt remains the way to continue outside Tinkery.

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
continuation prompt when the stored stage permits it; `s` opens [Shape](#shape) on the
same stages; `r` opens Think history.
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

Think settles what the operator wants; Shape settles how to build it, through Seed Me's
interview: investigate facts first, then one decision at a time with its evidence and a
suggestion, ending in a seed the operator confirms. Journey: think → tend → shape → approve.
The operator accepted this design and approved both slices on 2026-10-15.

- **Seed Me owns the session.** Shape reads and writes it only through Seed Me's session
  helper (`read`, `publish`, `status`, `seed confirm`). Tinkery keeps no copy of the ledger
  or a second contract; Seed Me changes are proposed in its repository.
- **Nothing runs by itself.** Each agent step starts from a key that names it and runs
  within an explicit budget (default zero). The copied continuation prompt stays available.
- **Three separate decisions.** Goal confirmation, seed confirmation and approval stay
  distinct on screen; Shape never asks again what Think settled.
- **Honest states.** A failed read, an unknown stage or a seed that no longer matches its
  confirmation is said plainly, with one next step.

### Reading where a seed stands

Built in [#149](https://github.com/jon-devlapaz/tinkery/issues/149). In a goal's detail in
Tend, `s shape` sits beside `c copy prompt` and appears on the same stages (goal saved, seed
drafted, seed ready). Esc returns to that same detail. Menu place 3 lists only goals waiting
for a seed (goal saved or seed drafted); Enter opens the same screen and Esc returns to the
list. A held `s` or Enter opens nothing.

The screen shows the confirmed goal exactly as the ledger records it, in ink, then one of:

| Seed Me reports | Shape says |
| --- | --- |
| A current question | Its label and question in jade, `What I found:` for the first evidence entry and `Also found:` for each later one (a label per entry, so wrapped lines never fake an indent), and `Suggestion:` in italic. The ledger keeps no options, so none are shown. |
| No question, no seed | Seed Me hasn't asked its next question yet |
| A draft seed | The draft and its revision wait for confirmation |
| A confirmed seed that `status` verifies | `Seed ready`, with the confirmed revision |
| A confirmed seed that `status` can't verify | Seed Me's own reason, such as the seed changing after confirmation |
| A practice seed, an ended session or an unrecognised state | That, plainly |

An open question comes before a seed, so a reopened decision is never hidden behind an
older draft. Each state ends with one `Next:` line. Without a model (no `--shape-pi`) it
points to tend's `c`, in the words Tend's detail uses for the same stage; with one, it names
Shape's own key (below). When Seed Me's helper refuses a session, Shape shows Seed Me's first
line of reason, such as a ledger saved in another format. A missing `--seed-me` or another
failure says so; reasons and paths are under F1. Reads run off the paint path. Opening,
reading and scrolling write nothing, copy nothing, call no model and leave the continuation
prompt unchanged. `status` probes a viewer URL that the session records, which is a local read.

### Answering and confirming

Built in [#150](https://github.com/jon-devlapaz/tinkery/issues/150). With `--shape-pi`,
`--model` and `--seed-me`, three stages take one key each; every other stage stays read-only.

| Stage | Words box | Key | What the key does |
| --- | --- | --- | --- |
| A current question | `Your answer:` | `F2 save answer and look into next` | Saves the answer, then one agent step |
| No question, no seed | none | `F2 look into the next question` | One agent step |
| A seed draft (shown in place of the goal) | `Your words:` | `F3 confirm seed rN` | Confirms that revision with the typed words |

- **One key per question.** `F2` saves the operator's answer exactly as typed (settled with
  `user` authority) and starts the agent step, so a question costs one key. Typing, paste,
  scrolling and help write nothing; held `F2`/`F3` don't repeat. Letters, capitals and `?` are
  words while a box is shown.
- **The agent step** runs Pi with an explicit model and Pi's read-only tools (`read`, `grep`,
  `find`, `ls`): it reads files and runs no commands. It starts in the folder shown on screen
  (see [Grounding](#grounding-in-the-repository)). Its input is the confirmed goal, the questions
  the operator answered in Think (from the [think record](think.md#think-record), when it can
  be attributed), `found_so_far` (every finding already saved) and Seed Me's editable ledger.
  Every reply carries `found` entries (what it checked, with paths and lines, and what it saw),
  with the next question and a suggestion, or with the seed's `You are confirming` text;
  anything else is a model error. The prompt lives in `src/shape/prompt.txt`, one rule per line,
  and is not yet tuned by an eval.
- **Budget.** `--shape-budget CALLS` sets how many steps this run may make; the default is zero.
  Each step is one call, counted before Pi starts. With none left, `F2` refuses before saving
  anything and says how to allow calls.
- **Tinkery makes every helper call.** The model never calls the helper. Answers and new
  questions go through `read --editable` and `publish` with the version the operator was shown
  (or the step started from); a stale version is refused by the helper, Shape says so, keeps the
  typed answer and reads again. It never overwrites.
- **The seed.** Tinkery writes `seed-contract.md` as a draft: `status: draft`, a title from the
  goal's first line, `revision: rN` (one past the previous draft; a confirmed seed is never
  replaced), the model's text under `## You are confirming`, then `## What's true today` from the
  saved findings, then `session.py seed decisions` output word for word. The model never writes
  either section; text that would add a heading, fence, status, revision or confirmation line is
  refused. No seed is drafted while a decision is open.
- **Seed confirmation** goes only through `seed confirm --revision rN`, with the operator's
  typed words as `--source`. Nothing the model replies can confirm, and Seed Me refuses a
  practice session's confirmation. Confirming is not approval to build.
- **Failures** each say what happened, then one `Next:`: no budget (start with
  `--shape-budget`), a model error or stop (`F2` tries the step again; a saved answer stays
  saved), a stale version (the fresh read's own next step), or a helper refusal (Seed Me's first
  line). Esc stops a running step; a helper call already started finishes, and Shape reads again.
  A stopped model call still counts against the budget, and the notice says so.

### Grounding in the repository

Built in [#159](https://github.com/jon-devlapaz/tinkery/issues/159), so the building agent
doesn't start blind. Operator defaults (2026-10-16): file tools only, no git commands; a step
that found nothing is flagged, not refused; the operator may pick the folder in Shape.

- **Which folder.** While a step is offered (a current question, or none asked yet), Shape shows
  `The step reads files in NAME. F4 picks another folder.` above the words box; F1 gives the full
  path. The folder is where Think recorded the goal was made, else the repository Tinkery started
  in, else where it started. It is found with the session read, so `F2` is never offered first.
  `F4` opens a `Folder:` box holding the current path; Enter uses an existing absolute folder (or
  `~/…`), Esc keeps the one shown, and a missing folder is refused with the shown one kept. The
  pick lasts while this Shape stays open and is written nowhere; each step's receipts name it.
- **Findings are saved in the ledger.** Each `found` entry becomes a Seed Me receipt (`checked`,
  `at`, `observed`, and `check: read-only file tools in FOLDER`). A question's findings go on its
  decision node and show beside it as `What I found: PATH:LINE: what was seen`. The seed step's
  findings, which the prompt asks to include the blast radius (files, modules and tests the change
  touches, and what could break), become settled facts (`authority: evidence`, an observation
  claim), so `seed decisions` lists them too. No Seed Me contract change was needed: its ledger
  holds facts with receipts, and its seed contract already asks for evidence with receipts and
  dates beyond the required sections.
- **Later steps build on them.** `found_so_far` lists every saved finding except the confirmed
  goal's own provenance, and the prompt asks the step to re-check only what it relies on.
- **`## What's true today`** lists the same findings word for word, grouped by how they were
  checked: `- PATH:LINE: what was seen (observed DATE)`. Whitespace inside a finding collapses to
  one line so no finding can start a heading. With none it says
  `- Nothing was recorded: no step said what it checked, so this seed isn't grounded in a repository.`
- **Nothing found is flagged.** A question with no findings shows
  `Nothing found: this question records nothing checked in the repository.`; a seed whose section
  records none shows the same for the seed. Neither is refused.

The three confirmations stay separate screens with separate words: Think's typed `confirm`
for the goal, Shape's `F3` with the operator's own words for the seed, and approval (not built).
Shape never changes the confirmed goal.

## Rationale and verification

Shape answering (#150) is covered by `tests/shape_answer_terminal.py`: a stub Pi and real Seed
Me helper sessions in TEST roots go from Think's confirmed goal to a confirmed seed at 100×30
and 80×24, with `status` reporting the seed confirmed and `handoff.verified: true`, and the
first step's input carrying Think's answered question. Cases cover the zero budget, a stale
version, a model failure, a helper refusal, a reply that tries to confirm or to write the
decisions, and a practice session's confirmation. Reversion probes went red for: a step started
without the key, answering from the current rather than the shown version, a changed goal, a
step that confirms its own draft, decisions not from `seed decisions`, and saving before the
budget check. These prove mechanics with stubs, not the quality of real questions or seeds.

Grounding (#159) extends the same journeys: before the first `F2` the folder is shown, F1 names
its path and `F4` picks another, whose path both steps then start in; the second step's input
carries the first step's saved finding; and the seed's `What's true today` section equals the
saved receipts word for word, directly above the `seed decisions` block that `seed confirm`
accepts. A case with no findings shows both flags. Rust tests cover the section's grouping,
one-line collapse, the goal's provenance left out, and the folder refusals.

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
