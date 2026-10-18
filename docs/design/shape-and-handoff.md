# Tend and handoff

Tend holds saved goals; [Shape](#shape) runs Seed Me's interview with an explicit
model and budget. The copied prompt supports continuation outside Tinkery.

## Receipt

Explicit successful saving shows `Saved.`, the exact frozen goal block and
`Next: shape it into a seed with Seed Me.`

- `c copy prompt` sends OSC 52 with quiet, retryable write feedback, not verified
  host-clipboard success; see [clipboard support](../terminal-setup.md#clipboard-osc-52).
- `s open in tend` opens this session's detail, not the garden.
- F10 returns to the menu, including from standalone Think.

Nothing launches, opens a browser, pastes into a harness or starts implementation.
Paths belong in F1 then `h`, with the [think record](think.md#think-record) below
Seed Me's path. If recording fails, the goal stays saved; help and a muted receipt
notice disclose the gap. Notices wrap without clipping in receipt and detail.
The plain-text exit receipt may include recovery locations.

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
Seed Me sessions stay read-only; linking writes only Tend's store.

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
`Saved.`. ↑↓, `j k`, PgUp or PgDn scroll; `c` copies on permitted stages, `s`
opens Shape on those stages, and `r` opens history. Esc goes detail → garden →
menu; F10 goes to the menu. F1 opens help; `h` opens technical details. Returning from Shape
requests a fresh read and restores this goal's detail; next step and garden stage
update together. Ended or unreadable sessions disappear from active goals. Returning
writes nothing and calls no model.

| Stored state | Label | Detail next step |
| --- | --- | --- |
| Confirmed goal | goal saved | Shape it into a seed with Seed Me |
| Draft seed contract | seed drafted | Confirm the seed draft with Seed Me |
| Seed declares confirmation for intake | seed needs checking | Check the seed with Seed Me |
| Ended session | ended | None; copying unavailable |

File-only discovery never verifies seed confirmation: even a legitimately confirmed,
unchanged seed says `seed needs checking`. Missing receipts or changed seeds cannot
claim readiness. Copying and Shape remain available; only Shape's helper-backed
status read can say ready. Discovery invokes no helper or duplicate verification.
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

↑↓, `j k`, PgUp, PgDn, Home and End scroll width-wrapped history, including Unicode
beyond 65,535 rows. Help scrolls independently. Esc goes details → help → history
→ same saved goal; F10 goes to menu. A fixed footer keeps return instructions
visible. Opening, scrolling and help call no model, copy nothing and write nothing.

`--record-root` selects existing records without requiring `--shape-pi`, defaulting
to the saving folder. One background reader lasts the menu's lifetime. Pending
requests coalesce; leaving discards results. A stuck read occupies that worker,
but never blocks navigation or starts another. Reopening explicitly requests a read.

The shared reader requires one schema-2 or schema-3 record matching canonical session path,
Seed Me session ID and exact confirmed goal, then rechecks the selected session.
Duplicate matches refuse selection. Matching schema-1 records lack exact attribution;
never migrate or guess from text. Unreadable or unsupported JSON may hide another
match, so it prevents selection. Each file is limited to 16 MiB; oversized records are
unavailable, never partial. Missing, unreadable and conflicting history explain
the gap without changing the goal. Session moves and unfinished-session recording
are outside this feature.

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
requiring absolute HOME). Tend never writes Seed Me files. Navigation and copying
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

One formatter serves receipt and detail. It names the skill and existing session,
requests `session.py read` and `status` before mutation, preserves the confirmed
goal, decisions and context, and stops on inactive sessions. It asks Seed Me to draft
`seed-contract.md` there and obtain explicit confirmation of the displayed revision.
It neither initializes a replacement nor grants implementation permission. The
operator pastes it manually into an authorized harness; prompts need paths even
when ordinary UI does not.

Use official Seed Me helpers, not another schema. Capture and check the destination
before helper access, always pass `init --root`, and retain it for retries and fresh
dumps. Never use a second implicit HOME default. [Discovery](menu.md#session-discovery)
owns root selection. Recovery errors stay explicit. Tests use temporary roots;
the external CI-pinned Seed Me fixture is neither vendored nor edited here.

### SDLC handoff

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
a failing confirmed command blocks verification despite passing worker-written
checks, and a passing command completes it. Dropping seed enforcement turns the
regression red; restoring it restores blocking. Its confirmations, approvals and
attestations are TEST fixtures, never the operator's.

## Shape

Think settles intent; Shape investigates facts, then asks one decision at a time
with evidence and a suggestion, ending in an operator-confirmed seed.

- Seed Me alone owns the session, ledger and contract. Reads and writes use its helper (`read`,
  `publish`, `status`, `seed confirm`, `end`); propose Seed Me changes in its repository.
- Each agent step needs an explicitly labelled key and budget (default zero).
  Manual continuation remains available.
- Shape never reasks settled Think intent or changes the confirmed goal.
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

| Seed Me reports | Shape shows |
| --- | --- |
| Current question | Show a muted label, jade question, `What I found:` for only the first question-attached evidence entry, and italic `Suggestion:`. F5 holds all findings for this question in recorded order. Options and the case against show above the suggestion; see [options](#options-and-instinct-first). |
| No question or seed | Seed Me hasn't asked its next question yet |
| Draft seed | Draft replaces goal under muted `Seed draft rN`; see [screen](#the-screen) |
| Confirmed seed verified by `status`, active or completed | `Seed ready` and revision; `c copy intake` when both paths are usable; completed adds a muted note that nothing can change |
| Unverifiable confirmation | Seed Me's reason, e.g. changed seed |
| Practice, stopped, unrecognised | Name that state plainly. |

Open questions precede older drafts. Without `--shape-pi`, one `Next:` points to
Tend's `c` with its stage wording; with a model, the key bar names the action.
Helper refusals show Seed Me's first reason line; missing `--seed-me` and other
failures are explicit, with reasons and paths in F1. Reads stay off paint paths.
Opening, reading and scrolling write or copy nothing, call no model and leave prompts
unchanged. `status` probes the session's recorded viewer URL as a local read.

Only Seed ready offers `c copy intake`: one line,
`python3 _system/scripts/sdlc.py stage <run> 1 --seed-contract '<absolute seed path>' --seed-me-helper '<absolute helper path>'`.
Both paths are canonical and shell-quoted; the helper is scripts/session.py beside
the same Seed Me skill used to verify the seed. Run from the tink-sdlc repository
root after replacing the visible `<run>` placeholder. If either copy path cannot
be resolved or represented on one safe line, the verified seed stays readable but
offers no copy. It uses Tend's OSC 52 route and `Copied.` / `Couldn't
copy; try again.` feedback, not verified host-clipboard success. Drafts, unverifiable
seeds and ended sessions offer no intake copy. Copying writes no files, calls no
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
  All current-question findings follow, unchanged and in recorded order. F5 or Esc
  closes it; ↑↓ scrolls. The fixed last row says `↑↓ scroll · F5 close`.
  It accepts no words, appears after a read, never during a step; F1 lists it.
- Say the next step once: omit `Next:` when the key bar names the stage action;
  other states and failure notices keep exactly one.
- Words boxes use ratatui-textarea, titled `your answer`, `your words` or `Folder:`.
  Arrows, Home, End, Ctrl+Left and Ctrl+Right move the cursor; Shift selects.
  Ctrl+W or Alt+Backspace deletes a word; Enter inserts a newline except at seed confirmation; Ctrl+U undoes
  and Ctrl+R redoes. PgUp and PgDn scroll only the reading. Pasted CR and CRLF
  display as breaks; edits preserve untouched raw endings. Under seed words, say seed confirmation is not approval to build.
- Running steps keep label, question and typed words muted, with one status
  (`Saving your answer.`, then `Answer saved. Looking in NAME for the next question.`),
  elapsed seconds and `esc stop`. F1 says a step is running.
- Draft display replaces file `status:` and `revision:` lines with `Seed draft rN` and
  shows `#` headings bold. All other lines remain as written; display never edits
  `seed-contract.md`.

### Answering and confirming

With `--shape-pi`, `--model` and `--seed-me`, only these stages accept actions:

| Stage | Words | Key/action |
| --- | --- | --- |
| Current question | `your answer` | `F2 save answer and look into next`: save then one step |
| No question or seed | None | `F2 look into the next question`: one step |
| Draft replacing goal | `your words` | `F3 confirm seed rN`: confirm displayed revision with typed words |

- F2 saves exact answer bytes with `user` authority. Typing, paste, scrolling and
  help write nothing; held F2 or F3 never repeats. Letters, capitals and `?` remain words.
- Pi runs one explicitly selected model with read-only `read`, `grep`, `find`, `ls`
  tools, no commands, in the [shown folder](#grounding-in-the-repository). Input includes
  the confirmed goal, attributable Think answers, every `found_so_far` finding and
  Seed Me's editable ledger. Replies require `found` entries (checks, paths, lines
  and observations) plus a next question and suggestion, or seed summary, prose sections
  and a separate final `checks` field;
  anything else is a model error. `src/shape/prompt.txt` has one rule per line.
  It finds the blast radius and the ways to meet the goal before choosing to ask or draft;
  a small live trial on #161 favored that order, which is evidence, not an eval.
- `--shape-budget CALLS` defaults to zero; each step counts one before Pi starts.
  Exhaustion refuses F2 before saving anything and explains how to allow calls.
- Tinkery, never the model, calls helpers. Answers and questions use `read --editable`
  and `publish` with the displayed version or the version at step start. Stale versions refuse, retain
  typed words and trigger a fresh read, never overwrite.
- Seed Me's [seed contract](https://github.com/jon-devlapaz/seed-me/blob/main/skills/seed-me/references/seed-contract.md)
  owns the draft format; Shape passes the installed reference to each step, rather
  than defining another layout. Tinkery writes `status: draft`, the goal-headline
  title, `revision: rN` one past the previous draft, and a separate `date:` line.
  An older writer can still increment the revision after rollback. Never replace
  a confirmed seed. The model supplies the summary and canonical prose sections,
  excluding Acceptance criteria; reasoning, findings and questions keep their existing
  free-text transport, not a new whole-reply schema. Tinkery follows the installed
  contract's exact Full heading sequence: `session.py seed decisions` output goes
  before Knowledge map, and recorded Evidence and uncertainty before Suggested first
  slice. Host-owned heading lines use LF while prose keeps its line endings.
  Helper blocks remain verbatim, with no extra wrapper headings.
  Refuse missing, duplicate, reordered or foreign model headings, fences, metadata
  or confirmation lines. Never draft with an open decision.
- The separate final `checks` field carries every confirmed `Done when` line with
  its qualifiers as records in Seed Me's installed contract, not model-formatted
  criteria. Tinkery passes those records unchanged to `seed checks set`; the
  helper alone validates required fields, kinds, unknown fields and repository-relative
  `cwd`. Invalid records refuse drafting. There is no Markdown-criteria writing
  path or backward compatibility. Tinkery inserts `seed checks render` output
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
- Confirmation uses only `seed confirm --revision rN`, typed words as `--source`.
  Model replies cannot confirm; Seed Me refuses practice confirmation. F3 refuses
  CR or LF with `Put your confirming words on one line; nothing was confirmed.`
  It retains words and neither writes nor calls. The empty `your words` field shows
  the muted hint `a line saying you confirm, then F3`; typing hides it. Enter says
  `F3 confirms the seed; Enter does not add a line.` without changing the words.
  Pasted breaks remain intact, but confirmation is one `Confirmed by:` line.
  Seed Me owns validation and confirmation.
  Confirming is not approval to build.
- Failures name what happened and one `Next:`. Budget exhaustion points to
  `--shape-budget`; a model error or stop offers F2 retry (saved answers stay saved);
  a stale version offers the fresh stage action; a helper refusal shows Seed Me's first line. Esc stops a step; an
  already-started helper finishes, then rereads. Stopped model calls still count,
  disclosed in the notice.

### Options and instinct first

A question reply may give exactly two options, each with its trade-off and undo cost (`cheap`,
`moderate` or `hard`), the strongest case against the suggestion, and a confidence. Anything
else gives no options. Seed Me's ledger has no options field, so Tinkery records them as
`Option A:`/`Option B:` lines, then `Against my suggestion:`, under the question text, where
Seed Me's viewer also shows them, and reads them back from there. The confidence joins the
recommendation. None of it reaches the seed.

- When either option is `hard`, Seed Me's instinct-first rule applies: the question is recorded
  without its recommendation, and Shape shows the question and its finding with
  `This is hard to undo, so what's your instinct? F9 shows the options and my suggestion.`
- `F9 show me` records the withheld suggestion through `publish`, then shows the options, the case
  against and the suggestion. The withheld suggestion lives only in this run; after a restart F9
  shows the options and says no suggestion was kept.
- Cheap and moderate questions show everything at once.

### Probes

Seed Me requires at least three probes before a seed, with at least one put to the operator
before any finding is shown. Shape puts the pre-mortem to the operator itself.

- With no current question and no `probe-pre-mortem` node, a step records `Probe: pre-mortem`
  (`Imagine this shipped and turned out wrong. What is the most likely reason?`) as the current
  question, without a model call or budget. The operator answers it with F2 like any question,
  in their own words, or says skip. Asked after findings were already recorded, it carries
  `NOT independent: asked after findings were shown.`
- The step answers the counter-example and outside-view probes from evidence, asks instead of
  drafting when either raises an operator-only decision, and records every probe and a
  `Where we did not look:` line in Risks and verification.
- These are instructions and transport; whether probes improve real seeds needs a separately
  approved budgeted trial.

### Assumptions

A step may list up to five defaults it chose instead of asking, where either answer leaves the
work, risk, cost, reversibility and behavior about the same. Tinkery adds them to Seed Me's
`assumed` list with the question (once per text) and shows them under that question only, headed
`I'll assume these unless you object:`, each with its why. An assumption is never an answer.

- `F7 object` then the assumption's number takes it off `assumed` and records an open decision,
  `Instead of assuming`, which blocks drafting. Esc says `Nothing was changed.`; other keys do
  nothing while asked. A refusal shows Seed Me's first line and keeps the list.
- Seed Me's entries are exactly `{text, why}`, so "shown" lives with the question in this run,
  not in the ledger; the seed's Assumed block always lists what remains.
- With no current question, a step first makes the first ready open decision current, without a
  model call or budget, so an objection (or any recorded open decision) is asked before a new one.

### Ending a session

Ending is for good: Seed Me makes the session read-only and Tend stops listing it,
which Tend's readiness needs. It needs only `--seed-me`, never a model or budget.

- `F6 end session` on an active Seed ready completes it; `F8 stop` on a question,
  waiting or draft stops it, keeping open questions recorded and typed words unsaved.
- The first press asks and says what ending costs, including copying intake first;
  the same key again ends, Esc says `Nothing was ended.` Other keys, held keys and the other
  ending key do nothing while asked.
- Tinkery calls `end` with a stated reason. Completing a session whose viewer never started passes
  `--no-viewer` with that reason. Seed Me refuses an unverified seed or an ended session; Shape shows
  `Nothing was ended.` with its first line. Every ending is followed by a fresh read.
- A completed session's verified seed stays Seed ready with `c copy intake`, since intake checks
  only the verified handoff. Tend doesn't list ended sessions, so this stays reachable only from
  the open view.

### Grounding in the repository

- Before F2 is offered, the session read finds the folder: recorded Think repo,
  else launch repo, else launch folder. Show `The step reads files in NAME. F4 picks
  another folder.` above words; F1 has the path. F4 opens `Folder:` with the current
  path. Enter inserts newline; F4 accepts an existing absolute folder or `~/…`.
  Try exact bytes first, including breaks and trailing whitespace; only if absent,
  try trimming surrounding whitespace. Esc keeps the previous choice; missing folders are refused. Selection lasts only this Shape visit, writes nowhere; step receipts name it.
- Each `found` becomes a Seed Me receipt: `checked`, `at`, `observed`,
  `check: read-only file tools in FOLDER`. Question findings attach to its decision
  and display `What I found: PATH:LINE: what was seen`. Seed findings include blast
  radius (files, modules, tests and breakage) and go only into the seed, never the
  ledger, so each appears once and later steps don't see them in `found_so_far`.
  A seed finding that looks like a secret refuses the draft, as Seed Me refuses one in a receipt.
- `found_so_far` excludes only confirmed-goal provenance; recheck relied-on findings.
- `Grounded in:` lists deduplicated paths cited by recorded read-only findings.
  Missing read methods, absent citations and failed citation checks establish no
  read path; say when none are known. These are reported reads, not verified
  filesystem origins or an inventory of unrecorded reads; name those provenance
  gaps rather than inventing them.
- The evidence section groups verbatim findings by check method as
  `- PATH:LINE: what was seen (observed DATE)`, collapsing internal whitespace to
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
Shape compares saved drafts with Seed Me and a [pinned consumer section list](../../tests/fixtures/tink-sdlc-intake-sections.txt),
never an operator's consumer checkout.
PTYs preserve Seed Me fixture bytes and cover
80×24, 100×30, 160×40 and 320×40 where applicable, including NO_COLOR. These establish
mechanics, not operator acceptance or the quality of real goals, questions or seeds. Lock timeout
coverage does not establish pre-deadline cancellation; matching read-only no-op
preservation has no dedicated reversion probe. Static graphs are a design choice,
not a comparative user-study result.
