# Tend and handoff

Tend holds saved goals; [Shape](#shape) asks its questions with an explicit
model and budget. There is no skill continuation prompt; continue the saved goal in Shape.

## Receipt

Explicit successful saving shows `Saved.`, the exact frozen goal block and
`Next: shape it into a seed in Shape.`

- `s open in tend` opens this session's detail, not the garden.
- F10 returns to the menu, including from standalone Think.

Nothing launches, opens a browser, pastes into a harness or starts implementation.
Paths belong in F1 then `h`, with the [think record](think.md#think-record) below
the session path. If recording fails, the goal stays saved; help and a muted receipt
notice disclose the gap. Notices wrap without clipping in receipt and detail.
The plain-text exit receipt may include recovery locations. Its next step is read
from the session at exit, through the shared verifier: a verified confirmed seed
points to Approve, a draft to confirming in Shape, and a goal without a seed to
shaping. An unverifiable or unreadable session points to checking in Shape, never
claims readiness; other ended sessions are named as ended.

## Tend: garden and detail

Inside a repository, Tend starts with only active goals whose attributed Think
record names that exact repository root. A muted `this repo · a all` line offers
`a` to show every active goal; `all · a this repo` toggles back. Outside a repo,
all goals show without a toggle. Unknown, missing, older or conflicting records
never guess a repo; those goals remain under all. An empty repo view still offers
`a`. If any active goal lacks attributable repo information, say `N goals have no
readable repo record. Press a for all goals.`, not that this repo has no goals.
Explicit no-repo records are known non-matches; older records without repo
information are unknown. Only with no unknown active goals or unreadable sessions
may an empty repo view say `No active goals are recorded for this repo.` With
unreadable sessions, say `N saved records couldn't be read; this repo may have
active goals. Press a for all goals.` Ended counts remain visible. Reopening Tend
starts with this repo again; explicit receipt or Shape returns still open the
exact saved goal, switching to all if necessary.

Filtering reads existing records on discovery's worker, never during painting,
and writes nothing. Each discovery enumerates the Think record root once and
reads each JSON record at most once, rebuilding an in-memory attribution index;
refreshes never reuse an earlier index. It shares history's strict validation and
rechecks session identity and goal across the scan. Malformed or unsupported files
make every origin unknown; duplicate paths and matching schema-1 records refuse
attribution, never choose a winner. The index keeps paths, identities, exact goals
and origins, not turns or checks; the 16 MiB per-file limit remains, with no total
index-size cap. Links, identity checks and readiness still use every readable
session, including other repos and ended goals. Use all to link across repos;
`a` does nothing during linking, detail, history, help or pending/failed reads.

Tend displays the visible goals in one static graph, without paging or node cap.
Equal hollow Braille rings mean size is unknown; no sizes or merges are known.
Needs lines join only active, uniquely identified endpoints. Missing, ambiguous or
ended endpoints draw no line; the selected-line summary still names them. Straight
lines leave gaps around unrelated rings, never implied junctions or routed paths.
Saved sessions stay read-only; linking writes only Tend's store.

Ended sessions are omitted. `N ended not shown` and
`Links couldn't be read. F1 then h for details.` share the muted incomplete-read
row, joined by ` · `, even with no active goals, and survive navigation. An
unreadable-links refusal replaces, rather than repeats, the note. When an empty
view's body reports unreadable sessions, the footer keeps ended and link notes
without repeating that count. This applies to repo and all-goals views, including
outside a repository. An incomplete empty all-goals view says `N saved records
couldn't be read, so nothing can be listed. Esc goes back.`, never `No active goals`.
Only when every session was readable and ended, say `No active goals. N ended not
shown. Confirm a goal in think to start one.`, not that no sessions exist.

Labels use the first saved-goal line, including unstructured goals. They sit above
or below rings, clipped with `…` or omitted rather than overlapping or interleaving.
The selected label gets space first and always appears, prefixed `› `; its label
and ring are bold, jade with colour. Other labels are muted and rings use ink;
none is italic.

The selected-line footer shows the first goal line in ink, status and relative time
muted, then needs and readiness, e.g. `needs Local records · ready`.
Unresolved ids say `1 need not found` or `N needs not found`; no needs adds nothing.
Short summaries retain status and time. If these crowd the headline, show the
headline, readiness and `F1 needs`; scrollable Help retains full titles. Ambiguous source
identity says `readiness unknown` and attributes no needs. Refusals use the blank
row before the key bar.

| Input | Action |
| --- | --- |
| Right, Down or `j`; Left, Up or `k` | Select next or previous in discovery's newest-first order, clamped; not graph neighbours. |
| Enter | Open detail. |
| `+` or `=`; `-` | Zoom in; zoom out. |
| `H J K L` | Pan left, down, up or right. |
| Left press on a ring or label | Select its exact session path; a second press within 500 ms opens it. |
| Drag a ring or label; drag empty space; scroll | Move the goal; pan; zoom. |

The key bar is `↑↓←→ move   enter open   l link   esc back` (fits 80 columns).
Selection pans offscreen goals into view without resetting zoom. Help, detail,
history, reading, empty and unavailable states ignore graph input.

Graf uses deterministic identity-ordered slots adapted to the canvas, initially
at least 12 columns × 6 rows apart, without a physics thread. Crowded gardens
require navigation, panning or zooming, not simultaneous labels. Moving, panning
and zooming write nothing. Refresh preserves positions and viewport for an unchanged
graph. Changed nodes, labels or links rebuild slots and reset manual positions but
keep zoom and pan. Resize reflows slots with relative zoom and pan retained.
Following selection may pan again.

Detail uses the receipt's shared renderer for the exact stored goal, without
`Saved.`. ↑↓, `j k`, PgUp or PgDn scroll (a page moves the view less one row); `s` opens Shape on permitted stages, and `r` opens history. Esc goes detail → garden →
menu; F10 goes to the menu. F1 opens help; `h` opens technical details. Returning from Shape
requests a fresh read and restores this goal's detail; next step and garden stage
update together. Ended or unreadable sessions disappear from active goals. Returning
writes nothing and calls no model.

| Stored state | Label | Detail next step |
| --- | --- | --- |
| Confirmed goal | goal saved | Shape it into a seed in Shape |
| Draft seed contract | seed drafted | Confirm the seed draft in Shape |
| Seed declares confirmation for intake | seed needs checking | Check the seed in Shape |
| Ended session | ended | None; copying unavailable |

File-only discovery never verifies seed confirmation: even a legitimately confirmed,
unchanged seed says `seed needs checking`. Missing receipts or changed seeds cannot
claim readiness. Shape remains available; only Shape's Tinkery-owned
status verifier can say ready. Discovery invokes no helper or duplicate verification.
Unknown and practice stages stay distinct.

[Session discovery](menu.md#session-discovery) owns refresh and completeness.
Pending empty views say reading or still reading; failed reads say unavailable with F1 then
`h`. Only complete current empty reads say no sessions. Incomplete reads retain
valid goals with an unreadable-record count and missing-sessions warning. If none
are readable, say nothing can be listed, not “no sessions”. Reading and unavailable
views offer only `esc back`; an empty repo view also offers `a toggle goals`.

## Think history

Detail's `r history` shows original words, questions, answers, skips, restores and
the exact confirmed goal in recorded order. A recorded repository adds `Made in`
and its folder; outside repositories or schema-2 records, omit it. F1 then `h`
shows the full path, `none` or `not recorded`, plus model information and checks.
Labels are muted; operator words use ink. Display escapes unsafe terminal controls
without changing stored words.

↑↓, `j k`, PgUp, PgDn (a view less one row), Home and End scroll width-wrapped history, including Unicode
beyond 65,535 rows. Help scrolls independently. Esc goes details → help → history
→ same saved goal; F10 goes to menu. A fixed footer keeps return instructions
visible. Opening, scrolling and help call no model, copy nothing and write nothing.

`--record-root` selects existing records without requiring `--shape-pi`, defaulting
to the saving folder. One background reader lasts the menu's lifetime. Pending
requests coalesce; leaving discards results. A stuck read occupies that worker,
but never blocks navigation or starts another. Reopening explicitly requests a read.

The shared reader requires one schema-2 or schema-3 record matching the ledger session ID
and exact confirmed goal, then rechecks the selected session and live-ID uniqueness.
Duplicate matches refuse selection. Matching schema-1 records lack exact attribution;
never migrate or guess from text. Unreadable or unsupported JSON may hide another
match, so it prevents selection. Each file is limited to 16 MiB; oversized records are
unavailable, never partial. Missing, unreadable and conflicting history explain
the gap without changing the goal. The one-time move rewrites no Think record. A recorded ledger ID remains the authority,
not the full path or the live folder name. For a path-only record whose old folder is missing,
compare its final folder name against live ledger IDs. Two live sessions with the same ID
refuse plainly rather than choose one; current-root identity scans happen off the paint path.
An existing path without a recorded ID cannot prove attribution. Unfinished-session recording
remains outside this feature.

## Linking needs

Garden `l` starts “this goal needs …”. The source retains `›`; movement marks a
target `→`. Enter or one left press adds or removes the need; picking the source does nothing.
The key bar becomes `↑↓←→ pick   enter link   esc cancel` (`enter unlink` for an
existing edge); the bottom line names both goals. Commit preserves the displayed
intent to add or remove despite concurrent changes. Already satisfied intent preserves
the file and says `Already linked.` or `Already unlinked.` Esc before commit writes
nothing. Committing or cancelling reselects the source; discovery refresh waits until picking
ends. Detail, history, help and reading states ignore `l`.

Missing or ambiguous identity, or unavailable links, changes nothing. Muted refusals are:
`This goal can't be linked yet.`,
`Links couldn't be read, so nothing changed. F1 then h for details.`, or
`Links couldn't be saved, so nothing changed. F1 then h for details.`
Technical help retains the exact reason and session path through pending refresh.
Lock waiting is synchronous, bounded to about two seconds. Timeout reports a save
failure with technical reason `another Tinkery is saving links`. Queued Esc or F10
cannot cancel an in-flight save that obtains the lock before deadline; navigation
follows it. Timeout refuses outright; later lock release never retries.

## Needs store

`--link-root PATH` selects `needs.json` (default `~/.local/share/tinkery/tend`,
requiring absolute HOME). Tend never writes session files. Navigation and copying
create no links; landed session reads and starting a pick refresh cached links.
Painting reads only the cache.

Version 1 is `{"version":1,"needs":[{"goal":"id","needs":"id"}]}`. IDs are nonempty,
control-free ledger `session_id`s, never titles or folders. Missing or invalid IDs
leave sessions readable but unlinkable. Sorting by goal id then needed id makes equivalent
sets byte-identical.

Missing files normally mean empty. After this view read nonempty links, disappearance
refuses reads and saves rather than losing known links. Bad JSON, unsupported
versions, invalid or duplicate entries and IO errors remain unreadable; edits cannot
overwrite them. Changes to read-only or unwritable existing files refuse without changing bytes
or permissions. Matching no-ops need no write access and preserve exact bytes,
permissions and timestamp.

Edits reject self-links and unreadable stores, acquire an exclusive sibling
`needs.lock` within the deadline, reread, apply intent and atomically replace
through a same-folder temporary file. The empty lock file remains so cooperating writers lock the same
file. Navigation creates neither file. This preserves concurrent saves, not changes
from external tools ignoring the lock.

Readiness uses links and all readable sessions, including ended ones. Found needs
use first goal lines; unresolved or duplicate IDs are counted without guessing.
Duplicate source IDs have no attributable needs or positive readiness. Ready means
at least one need, all uniquely found and ended. No needs, missing needs and unknown
stages are not ready.

## Continuation contract

The former skill-based copy prompt is retired. Receipt and Tend detail open Shape;
navigation launches nothing and writes no session. Approve and verified Shape seeds
still copy manual intake commands through [OSC 52](../terminal-setup.md#clipboard-osc-52).

Default sessions live in `~/.local/share/tinkery/sessions`. On the first default-root
launch, move the old sessions folder once and show a notice. If both roots exist,
refuse rather than merge or read the old root. Explicit roots never trigger the move.
The migration renames only the sessions folder, never parses its ledgers and never reads or
writes Think records. Old-format session folders move byte-for-byte too. History uses the
identity rule above; no old-root session reader remains.

### SDLC handoff

Approve and Watch share a read-only seed-to-run join, not a binding store. API 1's
`seed_contract.source` must equal the seed's canonical path and `sha256` must
match its current bytes. Names, slugs and goal text never establish a connection.
Edited originals are unknown, not started; moved or missing originals cannot lend
a goal to a run. A match supplies the session's exact saved goal (article 3);
[Watch](watch.md#reading) owns its compact list and full-detail presentation.
An explicit null record means no confirmed seed was used, distinct from an
unmatchable or malformed record (article 7). This is a read-time snapshot, not
permission or a live freshness claim.

Only an explicitly named `--sdlc-root` is read. Missing configuration, unreadable
runs or absent/unsupported seed records mean Approve cannot tell which seeds have
started and hides none (article 7). A readable null record establishes no join.
The shared `sdlc` boundary owns status reads and matching; `seed_me` alone reads
the session's files. There is no checkout discovery, source mutation or new store.

Goal and seed confirmation, SDLC definition approval and implementation permission
are separate decisions. External tink-sdlc intake preserves the confirmed seed and
imports its acceptance checks as locked definitions, reported separately from
worker-written supporting checks. Editing or dropping an imported check stales the
definition gate. Settled commands must succeed; prose `expect:` also needs operator
attestation. Provisional checks stay pending until activated with prerequisite
evidence and renewed definition approval; `you judge:` stays manual.

Locked checks protect the agreement but don't prove the checks are strong.
Hash-bound local receipts detect changed definitions; they are not authenticated
operator approval.

The [handoff regression](../../tests/shape_sdlc_handoff.py) carries Shape's real
seed writer through confirmation, a fresh TEST-only SDLC install and intake:
Approve's combined command, pasted unchanged from outside the checkout, starts a real run, Approve hides the matched seed, and
Watch shows its goal's first line under needs-you runs and the full goal in detail,
while a run with no seed stays distinct from an unknown match. A failing confirmed command blocks
verification despite passing worker-written checks, and a passing command completes it.
Dropping seed enforcement turns the regression red; restoring it restores blocking. Its confirmations, approvals and
attestations are TEST fixtures, never the operator's.

## Shape

Think settles intent; Shape investigates facts, then asks one decision at a time
with evidence and a suggestion, ending in an operator-confirmed seed.

- Tinkery owns Shape's ledger, seed checks, confirmation, verification and ending;
  [the accepted design](shape-ledger.md) records the completed migration. Draft instructions
  and the seed-contract reference are bundled, with their pinned source recorded in code.
- Each agent step needs an explicitly labelled key and budget (default zero).
  Manual continuation remains available.
- Shape never reasks settled Think intent or ledger answers, including delegated pre-mortem
  choices, or changes the confirmed goal. Seed drafting treats these choices as settled, not unknown.
- Failed reads, unknown stages and stale confirmations get honest reasons and one
  next step.

### Reading where a seed stands

Tend detail offers `s shape` beside copying on goal-saved, seed-drafted and
seed-needs-checking stages; Esc returns there. Menu place 3 lists only goal-saved
or seed-drafted sessions. Enter opens one; Esc returns to the list. Held `s` or Enter
opens nothing.

The menu passes discovery's unreadable-record count with its readable sessions.
Both empty and populated incomplete lists show the count and `F1 shows why`.
F1 lists up to eight actual failed paths and their read/parse reasons, then says
how many more unreadable records are not listed. Reasons retain at most 512
characters plus an ellipsis; display escapes controls without changing paths.
PgUp/PgDn scroll the wrapped help, with close keys always visible. Inspect the
listed files, then return to the menu and reopen Shape to read again. Fresh or
failed reads discard earlier details. Seed-read failures still mean an unknown
stage, not an unreadable session. Partial empty reads never say no goals are waiting.
A complete empty read keeps `No goals are waiting for a seed.` Session eligibility
and refresh rules are unchanged; list navigation writes nothing and calls no helper or model.

The screen starts with the confirmed goal's first line in ink:

| Tinkery reads | Shape shows |
| --- | --- |
| Current question | Show a muted label, jade question, `What I found:` for only the first supported finding (or legacy question receipt), and italic `Suggestion:`. F5 holds all findings for this question in recorded order. Options and the case against show above the suggestion; see [options](#options-and-instinct-first). |
| No question or seed | Shape hasn't asked its next question yet |
| Draft seed | Draft replaces goal under muted `Seed draft rN`; see [screen](#the-screen) |
| Confirmed seed verified by `status`, active or completed | `Seed ready` and revision; `c copy intake` when both paths are usable; completed adds a muted note that nothing can change |
| Unverifiable confirmation | Tinkery's reason, e.g. changed seed |
| Practice, stopped, unrecognised | Name that state plainly. |

Ledger review flags precede open questions and older drafts: Shape shows the first
flagged node as the current question, its unchanged answer and the ledger's reasons.
`F6 keep with reason` publishes the operator's typed justification without a model call.
`F2 revise` opens replacement words only for an operator-answered decision; see
[answering](#answering-and-confirming). Each remaining flag holds confirmation and completion.
Open questions precede older drafts. Without `--shape-pi`, one `Next:` points to
the next Shape setup or confirmation action; with a model, the key bar names the action.
Ledger refusals say `Tinkery refused` with the first reason line; other failures
are explicit, with reasons and paths in F1. Reads stay off paint paths.
Opening, reading and scrolling write or copy nothing, call no model and leave prompts
unchanged. Status ignores viewer records and snapshots; it opens no network connection.
`tinkery seed status --json DIR` prints the intake status without `viewer`, `snapshot` or
`snapshot_current`; `tinkery seed checks DIR` prints the strict check records. Both are read-only.

Only Seed ready with an external-intake-compatible confirmation offers `c copy intake`: one line,
`python3 _system/scripts/sdlc.py stage <run> 1 --seed-contract '<absolute seed path>' --tinkery '<absolute Tinkery executable>'`.
Both paths are canonical and shell-quoted; the executable is this running Tinkery binary.
Tinkery verifies the seed itself. Run from the tink-sdlc repository
root after replacing the visible `<run>` placeholder. If either copy path cannot
be resolved or represented on one safe line, the verified seed stays readable but
offers no copy. It uses Tend's OSC 52 route and `Copied.` / `Couldn't
copy; try again.` feedback, not verified host-clipboard success. Drafts, unverifiable
seeds and ended sessions offer no intake copy. Agent-confirmed seeds remain readable;
Shape and Approve withhold external human-only intake and explain why. Copying writes no files, calls no
model, runs no intake and grants no approval to build; choosing or creating an SDLC
run stays outside Shape.

### The screen

- Under `tinkery › shape` and `?`, content begins at the body top, at most 76 columns
  wide. The goal headline or draft, then stage, precedes words, notices and `Next:`.
  Overflow scrolls while the words box and cursor, or running status, stay visible,
  occupying at most half the body. Cursor following survives long or wrapped
  words and resize. Only overflow offers `pgup/dn read` while editing or `↑↓ scroll`
  otherwise; overflowing questions shorten the save key to `F2 save + look`.
  Seed `Nothing found` stays beside words. Crowded bars reduce gaps to two spaces,
  never cut Esc or F10.
- F5 **So far** is a full help-like page without a key bar: the entire confirmed
  goal unchanged, then settled `label: answer` decisions in ledger order. Only
  `user` authority shows `(you)`; others show their authority, e.g. `(delegated)`.
  Open questions follow in ledger order; held questions say `waits for:` with the earlier
  answer labels they need. All current-question findings follow, unchanged and in recorded order. F5 or Esc
  closes it; ↑↓ scrolls. The fixed last row says `↑↓ scroll · F5 close`.
  It accepts no words, appears after a read, never during a step; F1 lists it.
- Say the next step once: omit `Next:` when the key bar names the stage action;
  other states and failure notices keep exactly one.
- Words boxes use ratatui-textarea, titled `your answer`, `your words`, `your teach-back` or `Folder:`.
  Arrows, Home, End, Ctrl+Left and Ctrl+Right move the cursor; Shift selects.
  Ctrl+W or Alt+Backspace deletes a word; Enter inserts a newline except at seed confirmation; Ctrl+U undoes
  and Ctrl+R redoes. PgUp and PgDn scroll only the reading, a view less one row. Pasted CR and CRLF
  display as breaks; edits preserve untouched raw endings. Under seed words, say seed confirmation is not approval to build.
- Running steps keep label, question and typed words muted, with one status
  (`Saving your answer.`, then `Answer saved. Looking in NAME for the next question.`),
  elapsed seconds and `esc stop`. F1 says a step is running.
- Draft display replaces file `status:` and `revision:` lines with `Seed draft rN` and
  shows `#` headings bold. All other lines remain as written; display never edits
  `seed-contract.md`.

### Answering and confirming

With `--shape-pi` and `--model`, only these stages accept actions:

| Stage | Words | Key/action |
| --- | --- | --- |
| Flagged node | `your reason` | `F6 keep with reason`: keep the answer unchanged and revalidate, no model call |
| Flagged operator-answered decision | `replacement answer` after `F2 revise` | `F2 save revision and look`: replace it, reopen settled dependents, then look |
| Current question | `your answer` | `F2 save + look`: save then one step |
| Current question | Reason, then revisit condition after `F11 defer` | `F11 save deferral`: publish deferred, then one step |
| Current question | None required | `F12 delegate`: accept the grounded suggestion, then one step |
| No question or seed | None | `F2 look into the next question`: one step |
| Draft replacing goal | `your words` | `F3 confirm seed rN`: confirm displayed revision with typed words |
| Draft replacing goal | `your teach-back` after F7 | `F7 save + compare`: capture words and compare with the draft, one call; Esc restores confirmation words |

- **Before confirmation:** the ledger computes the risk summary above F3, including
  reversals detected from earlier settled answers, delegated and assumed choices,
  exact acceptance of suggestions, and accepted-versus-chosen-otherwise counts.
  Shape displays these words without its own checks or formatter.
- **Teach-back:** draft-only F7 opens optional words about what will and won't be built,
  prefilled with any confirmation words already typed, editable in a separate box.
  F7 again saves them verbatim, then compares in one call behind F2's shared budget,
  with no tools or repository access;
  opening, typing, help and cancelling do not save or call. Confirmation words remain
  untouched. The latest completed comparison for these draft bytes appears as guesses
  above the risk summary; mismatches are recorded verbatim and never block F3.
  Failed comparisons retain captured words. F3 without a completed comparison records
  `Skipped teach-back.` verbatim, after ordinary confirmation refusals. These append-only
  confirmed-goal receipts are bound to the revision and SHA-256 of draft bytes;
  confirmation digests cover them. They retain the existing receipt format and are
  interview records, not evidence changes or permission.
  F1 explains F7 only where it applies; draft bars leave F5 and F8 in help to fit 80 columns.
- **Defer:** F11 opens the reason, then advances to the revisit condition; Tab switches
  between them. The form shows the question's gate: the operator alone decides it can wait.
  F11 on the condition saves both fields verbatim as `deferred`, never confirmed or settled.
- **Delegate:** F12 is one press, quoting that key as the authority source, saving the
  recorded suggestion unchanged with `delegated` authority and its supporting facts as evidence.
  Reply confidence is not appended to the recommendation or answer; the ledger never parses
  suggestion text.
  Missing facts or suggestion refuses without writing; type an answer instead.
- **Revise:** only flagged decisions answered by the operator (including delegated answers)
  offer F2 revise. Flagged findings (facts and inferences) and the confirmed goal offer only
  keep-with-reason; Revise is not shown. There is no settled-answer picker: F5 remains read-only.
  F2 saves exact replacement words with user authority; the ledger reopens every settled
  dependent with a reason naming the changed answer. Shape asks ready dependents one at a time
  before a new model question. Saving a revision uses F2's budget gate and next-step path:
  an already-recorded dependent is asked without a model call; otherwise one step investigates
  or replaces the older draft.
- Esc cancels a form without changing the original answer editor. Ledger refusals keep the
  question and both typed fields, show the first reason line and allow correction and retry.
  Required fields, delegated evidence, revision eligibility and cascading reopen belong only to
  the ledger; Shape adds no parallel rules. F1 explains applicable keys; question bars
  leave F5 so far and F8 stop in help.
- F2 saves exact answer bytes with `user` authority. Typing, paste, scrolling and
  help write nothing; held action keys never repeat. Letters, capitals and `?` remain words.
- Pi runs one explicitly selected model with read-only `read`, `grep`, `find`, `ls`
  tools, no commands, in the [shown folder](#grounding-in-the-repository). Input includes
  the confirmed goal, attributable Think answers, every `found_so_far` finding and
  a model-only ledger view: findings appear only in `found_so_far`, not again as
  ledger fact nodes or copied decision evidence. Settled pre-mortem questions and answers stay
  once in their ledger decision nodes, not a second answer list. Goal provenance, including Think's
  original context, stays unchanged; stored ledgers are never trimmed. Replies require typed `found` entries (see
  [grounding](#grounding-in-the-repository)) plus a next question and suggestion, or seed summary, prose sections
  and separate typed `review` and final `checks` fields;
  anything else is a model error. Either reply may include optional `resolves`, a list of
  `{"unknown":"FACT-ID","resolved_by":"DECISION-ID"}` links to an already-settled answer.
  Absence means no links; malformed or invalid links refuse the reply plainly, saving no
  findings, question or resolution. An answer saved before that step stays saved.
  [The ledger](shape-ledger.md#2-what-shape-writes) owns eligibility, history and reopening;
  model coverage judgments are not semantic proofs. A question may include optional `prerequisites`, a list
  of existing decision IDs, preserved unchanged (absent means `[]`). Facts belong in `supported_by`,
  not prerequisites. The ledger alone refuses unknown IDs, cycles and invalid lists, saving no
  findings, question or assumptions from a refused reply. A held question stays open without
  becoming current; the ledger selects the next ready decision. Once its prerequisites settle,
  the held question is promoted before any model call. No reply can settle, skip or reorder
  earlier decisions. `src/shape/prompt.txt` has one rule per line.
  It finds the blast radius and the ways to meet the goal before choosing to ask or draft;
  a small live trial on #161 favored that order, which is evidence, not an eval.
- `--shape-budget CALLS` defaults to zero; each step counts one before Pi starts.
  Exhaustion refuses answer, defer, delegate, revision and teach-back before saving anything and explains how to allow calls.
- Tinkery, never the model, publishes answers and questions with the displayed version
  or the version at step start. Stale versions refuse, retain
  typed words and trigger a fresh read, never overwrite.
- Tinkery's bundled copy of the [seed contract source](https://github.com/jon-devlapaz/seed-me/blob/c9d766760dbeac0c05c92108165f9e968bec0a66/skills/seed-me/references/seed-contract.md)
  defines the artifact. Tinkery's ledger owns its canonical Full heading sequence;
  the reply fields and writer derive from that one layout. The model supplies
  `confirming` as an object: `You are confirming` contains exactly five non-empty
  summary lines, and each model-written prose section has its own string field named
  for the section, without heading syntax. Field order is immaterial. The summary
  covers goal, what gets built, what does not, the proposed first slice, and unknowns.
  Counts and reversal indicators belong to the ledger-computed confirmation summary.
  Drafting instructions retain names supplied
  in the confirmed goal or Think answers, including in the summary and knowledge
  map; a supplied tool name does not settle its executable path or invocation.
  Missing, blank, non-string or foreign fields
  refuse; prose cannot inject headings, fences, metadata or confirmation lines.
  Refusals name the rule and field, plus the offending line for structural text.
  A historical Markdown reply beginning `## You are confirming` is refused naming
  `confirming`, never with a generic format error.
  Shape attaches the bundled reference's section content guidance and check schema,
  not its instructions to write or paste artifact headings. The prompt defers to the
  generated field instructions, so there is no competing model-written layout.
  Tinkery writes every heading, `status: draft`, the goal-headline title,
  `revision: rN` one past the previous draft, and a separate `date:` line with the
  operator's local date (receipts stay UTC). An older writer can still increment the
  revision after rollback. Never replace a confirmed seed or draft with an open decision.
  Acceptance criteria, Decisions, Assumed and Evidence and uncertainty come only from
  recorded checks, decisions, assumptions and findings, in their canonical positions.
  Host-owned headings use LF; prose keeps its line endings. Ledger-owned blocks remain
  verbatim, with no extra wrappers. Findings and question transport are unchanged.
- **Completion review runs within drafting**, with no extra call. Before producing a seed,
  the step compares settled decisions for contradictions, separates choices (`decided`)
  from observed completed work (`done`), and supplies one failure-case string per check,
  in `checks` order (including provisional and judge checks). Tinkery pairs each string
  with the check's exact statement; the model never copies statements. A new blocker
  returns as a question, not a seed. The typed `review` object requires
  `contradictions`, `decided`, `done` and `failure_cases`. Historical statement/case
  objects still require exact, unique statement coverage; they are never guessed into
  positional matches. Missing or malformed reviews refuse; the ledger refuses nonempty
  contradictions, blank review entries and incomplete failure-case coverage before seed writes. This
  validates transport and coverage, not whether the model found every real conflict.
  The validated review is preserved verbatim in append-only confirmed-goal receipts
  (`tinkery:review/completed`), using teach-back's revision and draft SHA-256 binding
  and recording path. These receipts are interview records, not changed goal evidence.
  With the risk summary above F3, Shape marks the review as `Drafting step guess`:
  settled choices, observed done (or `Nothing observed done.`), and each failure
  case as `Must fail: <case>` followed by an indented `Check: <statement>` line.
  Both strings stay verbatim; the label above them marks them as drafting guesses.
  Only the review for these exact draft bytes is shown. No check is run and F2's
  existing call budget is unchanged.
- The separate final `checks` field carries every confirmed `Done when` line with
  its qualifiers as records in the bundled contract, not model-formatted
  criteria. Tinkery's ledger stores those records unchanged and
  validates required fields, kinds, unknown fields and repository-relative
  `cwd`. Invalid records refuse drafting. There is no Markdown-criteria writing
  path. Existing legacy JSON acceptance sections remain verifiable but cannot be newly confirmed.
  Tinkery inserts its ledger's rendered acceptance block
  verbatim before Affected users and systems, then displays that saved card.
  No check executes during the interview.
  Copy settled commands exactly. Repository-grounded proposals name source paths
  and lines in `source`; behavior-specific regressions name the assertion expected
  to fail before the change or under targeted reversion. Inspection is not observed
  red or green. Routine setup or an unperformed reversion probe does not make an
  existing check provisional; missing assertions, tests or invocations do, with
  `missing` naming the gap. Never invent commands or substitute a generic runner
  for a missing regression. Taste stays `judge` with what the operator inspects.
  If requested behavior appears present, propose verification and repairs only for
  demonstrated gaps. Settled and explicitly deferred decisions stay settled;
  deferrals remain operator prerequisites, not chosen alternatives.
  Seed confirmation v2 binds canonical records, rendered seed bytes and decisions.
  Missing, changed or stale records invalidate readiness; renewed displayed drafting
  and explicit confirmation are required. A failure after storing records can leave
  unconfirmed records, never a verified handoff.
  These are drafting instructions, not a semantic completeness validator: stubs prove
  transport and preservation, not model quality or operator dogfood.
- Agent actions require the [named confirmation context](think.md#confirmation-attribution-185).
  Records name the acting agent and authorizing operator; seed confirmation never claims a human keypress.
- Confirmation uses Tinkery's ledger with the displayed revision and typed words as its source.
  Model replies cannot confirm; Tinkery refuses practice confirmation. Until the
  draft revision's last line has been on screen during this Shape visit, F3 says
  `PgDn to 'End of seed' first. Nothing was confirmed.`; reaching the display-only
  `End of seed` marker after the last section clears that. The saved seed is unchanged.
  The exact revision and draft bytes retain that satisfaction through typing, box growth,
  resize and rereads, including teach-back. A new revision or changed bytes require reading again.
  F3 refuses CR or LF with `Put your confirming words on one line; nothing was confirmed.`
  Changing the words clears a refusal about them.
  It retains words and neither writes nor calls. The empty `your words` field shows
  the muted hint `a line saying you confirm, then F3`; typing hides it. Enter says
  `F3 confirms the seed; Enter does not add a line.` without changing the words.
  Pasted breaks remain intact, but confirmation is one `Confirmed by:` line.
  Tinkery owns validation and confirmation.
  Confirming is not approval to build.
- Failures name what happened and one `Next:`. Budget exhaustion points to
  `--shape-budget`; a model error or stop offers F2 retry, or F7 for teach-back
  (saved words stay saved);
  a stale version offers the fresh stage action; a ledger refusal shows Tinkery's first line.
  Review refusals name the mismatched check or case/check counts; a missing check source
  explains what can establish it without inventing evidence. F2 retries explicitly,
  retaining saved answers; nothing retries automatically.
  An oversized step sends nothing and keeps the words, asking the operator to start a
  smaller goal with this session as a reference. Esc stops a step; an
  already-started save finishes, then rereads. Stopped model calls still count,
  disclosed in the notice.

### Options and instinct first

A question reply may give exactly two options, each with its trade-off and undo cost (`cheap`,
`moderate` or `hard`), the strongest case against the suggestion, and a confidence. Anything
else gives no options. The ledger has no options field, so Tinkery records them as
`Option A:`/`Option B:` lines, then `Against my suggestion:`, under the question text, and reads them back from there.
The ledger has no separate confidence field. Reply confidence is not stored or shown;
Shape renders the recorded suggestion, not the transient reply. No schema field is added.
None of the options or case against reaches the seed.

- When either option is `hard`, The instinct-first rule applies: the question is recorded
  without its recommendation, and Shape shows the question and its finding with
  `This is hard to undo, so what's your instinct? F9 shows the options and my suggestion.`
- `F9 show me` records the withheld suggestion through `publish`, then shows the options, the case
  against and the suggestion. The withheld suggestion lives only in this run; after a restart F9
  shows the options and says no suggestion was kept.
- Cheap and moderate questions show everything at once.

### Probes

Drafting requires at least three probes before a seed, with at least one put to the operator
before any finding is shown. Shape puts the pre-mortem to the operator itself.

- With no current question and no `probe-pre-mortem` node, a step records `Probe: pre-mortem`
  (`Imagine this shipped and turned out wrong. What is the most likely reason?`) as the current
  question, without a model call or budget. The operator answers it with F2 like any question,
  in their own words, or says skip. Asked after findings were already recorded, it carries
  `NOT independent: asked after findings were shown.`
- The step investigates counter-example and outside-view probes as typed inference or unknown
  findings, asks instead of drafting when either raises an operator-only decision, and records
  consequences and a `Where we did not look:` line in Risks and verification. Only asked and
  answered ledger questions appear as operator answers in Decisions. Seed prose refuses the old
  `Probe:` / `— answer:` template, including inline in Knowledge map; unasked probes are readings,
  not answers. This format guard is not a semantic verifier of arbitrary model prose.
- These are instructions and transport; whether probes improve real seeds needs a separately
  approved budgeted trial.

### Assumptions

A step may list up to five defaults it chose instead of asking, where either answer leaves the
work, risk, cost, reversibility and behavior about the same. Tinkery adds them to the ledger's
`assumed` list with the question (once per text) and shows them under that question only, headed
`I'll assume these unless you object:`, each with its why. An assumption is never an answer.

- `F7 object` then the assumption's number takes it off `assumed` and records an open decision,
  `Instead of assuming`, which blocks drafting. Esc says `Nothing was changed.`; other keys do
  nothing while asked. A refusal shows Tinkery's first line and keeps the list.
- The ledger's entries are exactly `{text, why}`, so "shown" lives with the question in this run,
  not in the ledger; the seed's Assumed block always lists what remains.
- With no current question, a step first makes the first ready open decision current, without a
  model call or budget, so an objection (or any recorded open decision) is asked before a new one.

### Ending a session

Ending is for good: Tinkery makes the session read-only and Tend stops listing it,
which Tend's readiness needs. It needs no model or budget.

- `F6 end session` on an active Seed ready completes it; `F8 stop` on a question,
  waiting or draft stops it, keeping open questions recorded and typed words unsaved.
- The first press asks and says what ending costs, including copying intake first;
  the same key again ends, Esc says `Nothing was ended.` Other keys, held keys and the other
  ending key do nothing while asked.
- Tinkery ends through its ledger with a stated reason, the shared writer lock and atomic save.
  Completion needs the verified seed, not a viewer or `--no-viewer`. Tinkery refuses an unverified
  seed or a conflicting ended status; an identical ending is a no-op. Shape shows the ledger's
  refusal or uncertain-save message without claiming nothing changed. Every ending is followed by a fresh read.
- A completed session's verified seed stays Seed ready with `c copy intake`, since intake checks
  only the verified handoff. Tend doesn't list ended sessions, so this stays reachable only from
  the open view.

### Not adopted from Seed Me

[#235](https://github.com/jon-devlapaz/tinkery/issues/235) compared Shape with Seed Me's interview.
Delegation (#239), deferral (#240) and typed findings (#241) are shipped through
[the ledger](shape-ledger.md). These stay out until their revisit condition holds:

| Seed Me feature | Why Shape leaves it out | Revisit when |
| --- | --- | --- |
| Conditional answers | Shape records "only if X" as a settled answer, not a condition to investigate; supporting it needs predicate tracking and a re-ask loop. | Dogfood shows conditional answers. |
| Browser viewer | Shape and Tend already show the ledger; status never opens a connection. | A separately approved viewing need cannot be met in Tinkery. |
| Lean path | Lean is one Markdown file co-edited by hand; Shape has a ledger and Think already shapes the goal. | Full seeds prove too heavy for small goals. |
| Agent mode | It tests the skill; Tinkery's stubs cover that, and a simulated seed never hands off. Agent work uses `--operator agent`. | A simulated run is needed in Tinkery. |
| Frontier progress | F5 lists open and held questions; a progress count adds no next action. | Dogfood shows a need beyond the open-question list. |

### Grounding in the repository

- Before F2 is offered, the session read finds the folder: recorded Think repo,
  else launch repo, else launch folder. Show `The step reads files in NAME. F4 picks
  another folder.` above words; F1 has the path. F4 opens `Folder:` with the current
  path. Enter inserts newline; F4 accepts an existing absolute folder or `~/…`.
  Try exact bytes first, including breaks and trailing whitespace; only if absent,
  try trimming surrounding whitespace. Esc keeps the previous choice; missing folders are refused. Selection lasts only this Shape visit, writes nowhere; step receipts name it.
- Each `found` becomes a settled `fact` on `evidence` authority, with a new `id`,
  `claim` and a receipt (`checked`, `at`, `observed`, `check: read-only file tools in FOLDER`).
  `claim.type` is `observation`, `inference` or `unknown`, and `claim.scope` names what
  was examined. Inferences cite observations through `supported_by` and state `limits`;
  unknown says what this investigation has not established, never that it does not exist.
  A question cites its facts through `supported_by`; a fact may name a settled node it
  `contradicts`. The ledger alone validates these rules and receipts; a refused reply
  saves no findings or question and shows its reason. An answer saved before the step stays saved.
- Question findings display `What I found: PATH:LINE: what was seen`; F5 holds all.
  Seed-step findings also become facts, including blast radius, and later steps receive
  them with IDs and claims in `found_so_far`. Model replies cannot revalidate, confirm
  or settle a decision. Untyped `checked`/`observed` replies are refused, not guessed
  into types; existing sessions' untyped receipts still read and render unchanged.
- `found_so_far` excludes confirmed-goal provenance, superseded facts and resolved unknowns;
  recheck relied-on active findings. Resolved unknowns remain in ledger history, not active
  context or the seed's Decisions and Evidence blocks, including copied receipts.
- `Grounded in:` lists deduplicated paths cited by recorded read-only findings.
  Missing read methods, absent citations and failed citation checks establish no
  read path; say when none are known. These are reported reads, not verified
  filesystem origins or an inventory of unrecorded reads; name those provenance
  gaps rather than inventing them.
- The evidence section groups findings by check method, with claim type, scope,
  inference limits and support IDs for typed facts. Legacy receipts retain
  `- PATH:LINE: what was seen (observed DATE)`. Each finding appears once in this section;
  the ledger's canonical Decisions block also omits resolved unknowns. Collapse internal whitespace to
  prevent heading injection. With none:
  `- Nothing was recorded: no step said what it checked, so this seed isn't grounded in a repository.`
- A finding citing `PATH:LINE` or `PATH:FIRST-LAST` that the folder lacks (missing, outside
  it, or too few lines) is flagged, not refused: its check ends `; citation not found: WHY`,
  questions show `(citation not found: WHY)` and the seed groups it under that check.
  This proves the lines exist, not what they say.
- Questions or seeds with no findings display `Nothing found: this question records nothing
  checked in the repository.` (seed wording for seeds), flagged rather than refused.

## Rationale and verification

[Tests](../../tests) cover these contracts with stubs and temporary TEST roots.
Shape checks saved drafts against the bundled contract and a [pinned consumer section list](../../tests/fixtures/tink-sdlc-intake-sections.txt),
never an operator's consumer checkout. PTYs use Tinkery's real Rust ledger in temporary TEST roots and cover
80×24, 100×30, 160×40 and 320×40 where applicable, including NO_COLOR. These establish
mechanics, not operator acceptance or the quality of real goals, questions or seeds. Lock timeout
coverage does not establish pre-deadline cancellation; matching read-only no-op
preservation has no dedicated reversion probe. Static graphs are a design choice,
not a comparative user-study result.
