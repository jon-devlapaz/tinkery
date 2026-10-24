# Taste

Taste reviews one exact result against its unchanged saved goal. It never records a
verdict, posts a review, merges, signs a gate or launches an agent. Acceptance and
correction remain deliberate external decisions, not interpretations of green checks.
An explicit report of external acceptance can reveal an ephemeral next-dump guess.

## Sources and selection

Menu place 6 uses the named `--sdlc-root`, never checkout discovery. Opening only
explains the read. `r` runs that checkout's API 1 `status --json`, then each valid
run's `status <slug> --json`. The shared [SDLC boundary](architecture.md#modules-today)
owns bounded execution, cancellation and the exact saved-goal join used by Watch.
Unsupported interfaces or states refuse the reading. No source writes or polling.

API 1 has no PR reference, and its [SDLC contract](https://github.com/jon-devlapaz/tink-sdlc/blob/main/assets/_system/SDLC.md#L232-L240)
says handoff prose is not a machine interface. Supply the relationship deliberately
at launch with repeatable `--taste-result RUN_ID=PR_URL` inputs: a canonical lowercase
UUIDv4 run ID and a full `https://github.com/OWNER/REPO/pull/NUMBER` URL. For example,
using synthetic identities:

```text
tinkery --operator agent --home --sdlc-root /TEST/checkout \
  --taste-result 00000000-0000-4000-8000-000000000114=https://github.com/TEST/example/pull/1
```

Inputs live only in this process: no mapping store, config file, editor or upstream
change. Identical repeated declarations agree; different PRs for one ID leave a
sticky conflict, never last-wins. Malformed IDs/URLs refuse launch before any read.
Missing/conflicting inputs retain an unknown-result row and ask for corrected launch
inputs. Neither a slug, branch, title, handoff file nor another run's mapping can
supply the link; recreating a slug with a new run ID cannot inherit the old PR.

Only `r` reads GitHub, via `gh pr view <explicit URL> --json` using the operator's
login. Read the current head, state and PR-head checks. Only open PRs are waiting
results, including green PRs with no source gates owed: checks never remove the need
for human review. Closed/merged observations are not acceptance. Source gaps stay visible.
Each bounded helper has a 30-second deadline and 4 MiB output limit; helpers run
serially off paint/input. Leaving cancels and reaps the owned process group; late
results cannot reopen a view. Refresh removes old rows and copy requests first.

Keep source order, select the first result initially, and preserve selection across
refresh by run-instance ID plus PR URL. A vanished identity returns to the list
with a reason. No complete empty claim survives a failed read.

## One result

The list gives each result its goal's first line, PR, commit, individual states and
owed gates. Enter reveals the whole saved goal, unchanged and untrimmed, including
its completion conditions and constraints. The shared source/path-and-digest join
supplies it; changed, moved, missing or unmatchable originals stay unknown. Escape
unsafe terminal controls only for display, not storage or copying.

The PR, commit and read time stay fixed while the goal and evidence scroll.
Show the commit **once per result**, not evidence codes beside every check. Each
source expectation retains its exact wording, method and pass/fail/pending/stale
state. A local pass needs both source-current verification and its receipt's exact
head matching the displayed PR head. A different head makes prior passes stale even
if SDLC still reports tree-equivalent verification as current. Missing result
identity stays pending/unknown. Provisional checks stay pending. Failed matching
observations say fail; absent evidence never passes. Passed human items with a
source `needs_recheck` retain “may need recheck,” not an invented invalidation.

GitHub checks concern the PR head, not the merge commit. Missing, skipped, neutral
or unfamiliar outcomes are not passes. Independent review remains pending for
external inspection because API 1 supplies no revision-bound independent review
receipt. This is separate from automated proof and operator acceptance.

Owed gates use plain actions: “approve the plan” or “re-approve the plan,” plus
owed manual expectations in their source words, never checklist IDs or seed jargon.
The existing evidence tail and copied request retain each automatic item's API 1
`run.log.path`, resolved lexically against the named checkout within the selected
run, its exact checking `argv` and `cwd` (with the API's default `.` labelled when
omitted), and optional `verification.log` SHA-256. Checking directories such as
`./`, `./src`, `src/` and repeated separators retain their original source spelling;
only their displayed resolved directory is normalized. Absolute, parent-traversing
and control-bearing cwd values remain unresolved; stricter log-pointer validation
is not loosened. That receipt digest is not a path.
No log is opened or dumped, no argv is executed, and no evidence URL is invented.
Pointers are source references, not existence claims. Missing/unsafe pointers or
methods leave matching pass claims pending; older heads/source-stale receipts still
stay stale. Human marks retain their own evidence separately.
There is no aggregate done/approved status. Read time means a snapshot, not an
unchanged-since guarantee. No preview URL contract exists in these sources, so no
play link is invented.

## Keys and handoff

- List arrows choose; Enter reviews. In review, arrows, PgUp/PgDn and Home/End
  scroll the whole goal and evidence. `n`/`p` or Right/Left change result, clamped.
- `o` alone uses `gh pr view <explicit URL> --web` to open the PR. Navigation records
  nothing and rereads no checks. Browser failure names the retry. Repeated or modified
  action keys never open, reread or copy.
- `a` / `c` copy acceptance-review / correction requests through the existing clipboard
  transport, not executable approval commands. Copy feedback claims transport only.
  Each request retains the exact goal, run ID, seed digest, PR and full commit,
  condition evidence, gaps and owed gates. Unknown goal/result identity refuses copy.
- `?`/F1 help blocks underlying actions. Esc closes help, then review, then returns
  Home; F10 returns Home. The footer always names the next action and exit.

The copied request asks the external process to recheck current head, inputs and
receipts, resolve gates, inspect that exact result, and add the operator's requested
changes there before recording any decision. Copy remains available with failed or
missing checks but carries those gaps; it cannot approve around them. External
expected-input binding is still not guaranteed; changed material needs rereview.

## After external acceptance

In review, `l` means “I accepted this exact result externally.” It reports the
operator's decision for this snapshot only; it neither verifies acceptance nor
records a verdict. Copying `a`, browser navigation, green checks and merged/closed
PR states never imply acceptance. Unknown goal, run or result identity refuses the
report. Report before leaving the snapshot: refreshed merged results are not waiting
results and are not resurrected as accepted ones.

The report reveals one italic block labelled **Tinkery's guess**, with Outcome,
Evidence, Learning, Futures, Decision and Next. Outcome stays unobserved without
real feedback; changeability stays unknown without code-level evidence. No model
calls, extra reads, numerical score or structural obstacle is invented. Failed or
stale checks win, then source gaps, owed actions and other unmet conditions, in
source order. These recommend stop and repair before stacking features. The API's
unavailable independent-review receipt remains an evidence limit, not a fabricated
unmet completion condition. With nothing unmet, proceed means observe the accepted
behavior, not invent a new feature. Consolidation and re-scoping require evidence
these sources do not supply, so neither is guessed from check counts.

`t` opens the single Next suggestion in a fresh Think visit as an editable, unsent
draft. It names the behavior/uncertainty, unchanged boundaries and a disproof check,
including the exact original goal as labelled context. No host, source, save or run
starts from opening it. Only F2 submits the edited text through Think's usual budget
and preflight boundary. [Think](think.md#taste-drafts) owns separate provenance and
submitted-word attribution. Esc dismisses the guess; correction, result selection,
rereading and leaving discard it. Neither the suggestion nor the report is a new
gate or authorization. The operator still merges externally.

## Verification

`src/taste/tests.rs` and the reader's unit tests cover exact goal preservation,
unknown identity, missing/unsafe evidence, strict run-ID/URL inputs, statuses, source staleness,
selection identity, clipboard requests, key repeats/modifiers and small-size scrolling.
The changed-result regression must fail when the exact-head mismatch protection is
removed, then pass after restoration.

[Terminal journeys](../../tests/taste_terminal.py) use fake API 1 and `gh` in guarded
TEST homes at 100×30 and 80×24, each with and without NO_COLOR. They exercise
explicit reads/browser navigation, help isolation, both copied payloads, long evidence,
changed revisions, failed/pending checks, green open PRs, unknown originals,
missing/conflicting/wrong-run/recreated-run inputs, ignored handoff prose, usable API 1
log pointers/methods in both clipboard requests, remote failure, exit/return and cancellation. The checkout stays byte-identical;
no providers, verdicts or real browser launches run. The [API-shaped fixture](../../tests/fixtures/taste-api1.json)
preserves the reviewer's log/method/digest example without a private seed locator.
Next-dump journeys additionally report acceptance, edit and send only with F2,
correct without acceptance, and handle nothing unmet, at both sizes with and without
NO_COLOR. Unit regressions distinguish copying from the explicit report, prioritize
failed/stale evidence, discard stale drafts and keep provenance out of requests.
The report-boundary regression must fail with its acceptance protection reverted.
These prove mechanics, not operator
dogfood or improved attention; #109's comparison method remains separate.
