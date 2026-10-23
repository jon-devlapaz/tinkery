# Taste

Taste reviews one exact result against its unchanged saved goal. It never records a
verdict, posts a review, merges, signs a gate or launches an agent. Acceptance and
correction remain deliberate external decisions, not interpretations of green checks.

## Sources and selection

Menu place 6 uses the named `--sdlc-root`, never checkout discovery. Opening only
explains the read. `r` runs that checkout's API 1 `status --json`, then each valid
run's `status <slug> --json`. The shared [SDLC boundary](architecture.md#modules-today)
owns bounded execution, cancellation and the exact saved-goal join used by Watch.
Unsupported interfaces or states refuse the reading. No source writes or polling.

API 1 has no PR reference. The operator selected **run handoff links** for #114:
read `runs/<slug>/handoff.md`, bounded to 256 KiB and confined to that run, and
require exactly one distinct canonical `https://github.com/OWNER/REPO/pull/NUMBER`
URL. Repeated identical links agree; different links conflict. Never guess from
branches, titles or run names. Missing/conflicting handoffs retain an unknown-result
row and a repair step, not a positive review claim.

Only `r` reads GitHub, via `gh pr view <explicit URL> --json` using the operator's
login. Read the current head, state and PR-head checks. Only open PRs are waiting
results; closed/merged observations are not acceptance. Source gaps stay visible.
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
The scrollable evidence tail preserves recorded evidence and source explanations;
there is no aggregate done/approved status. Read time means a snapshot, not an
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

## Verification

`src/taste/tests.rs` and the reader's unit tests cover exact goal preservation,
unknown identity, missing evidence, strict link selection, statuses, source staleness,
selection identity, clipboard requests, key repeats/modifiers and small-size scrolling.
The changed-result regression must fail when the exact-head mismatch protection is
removed, then pass after restoration.

[Terminal journeys](../../tests/taste_terminal.py) use fake API 1 and `gh` in guarded
TEST homes at 100×30 and 80×24, each with and without NO_COLOR. They exercise
explicit reads/browser navigation, help isolation, both copied payloads, long evidence,
changed revisions, failed/pending checks, unknown originals, missing/conflicting links,
remote failure, exit/return and process cancellation. The checkout stays byte-identical;
no providers, verdicts or real browser launches run. These prove mechanics, not operator
dogfood or improved attention; #109's comparison method remains separate.
