# C16 Phase 2 — after versus baseline

Branch `think-goal-shape`, based on main `8818e63`. The approved review changes are
in [`../PLAN.md`](../PLAN.md). No merge, launcher/install change, Seed Me repo edit
or operator-session confirmation. Exact runtime hashes match the evaluated source;
subsequent changes were test-only. See `verification.json`.

## What changed

Five optional parts and the model's replacing `open` list; one pure composer used
by reading/paper/frozen confirmation; one targeted question with an application-owned
two-ask cap and skipped-target exclusion; readiness requires all five parts and no
open/skipped/pending scope/local text. Partial goals remain reviewable/affirmable.
Quiet review shows only present parts (`when`, `I can`, `so`, `it works if`, `not`)
and one Still open section. No separate issue ledger or freeform legacy goal path.
Historical raw boards display safely without inferred parts. Exact span/history,
consent, helper/read-back/recovery, no-seed and resource guards remain.

Prompt rules are generic; no eval words/names/numbers were put in the prompt.
Application normalization only joins unquoted internal sentence stops into clauses,
keeps quoted punctuation/decimal quantities, avoids duplicating a supplied first-person
prefix, and removes syntactic trailing collection commas outside JSON strings.
No model repair/retry, semantic veto or invisible invented field value.

## Final comparison

Exact readings, part values, raw responses, questions, full source history and four
screen sizes are saved per case in `00-first.json`/paper/captures, plus quiet review
pages. Assessment here is assistant inspection, not another provider evaluation.
Intact originals remain byte-identical regardless of interpretation omissions.

| Dump | Baseline | Final after | Questions / retention |
|---|---|---|---|
| books | One imperative; current-year versus ongoing-record question. | Two composed sentences; all five parts present; ready on first board. | No question. This year, one place, short notes, remembering by December, nothing fancy represented. Audit flags the original forgetting statement despite its presence in situation: uncertain judgment, not proven loss. |
| onboarding | Five-email quantity missing; unsubscribe versus spam/equal-balance question. | Situation explicitly retains **five emails in the first week**; outcome and proof remain absent/open. | **What do you want to be able to know, decide, or do about the onboarding emails?** Avoids equal-weight choice but may over-ask a stated end experience. No killing emails / no spam retained. Marketing affinity not stated as the original fact; audit flags it. |
| trap | No gamification in notes, phone requirement softened to especially phones; understand/feel judged/both question. | **No gamification; it must work on phones** in boundaries; chat/rebuild not selected as solution. | **What observable change would show that a learner leaves a wrong answer feeling smarter?** Unverified question-4 report and expense/tone concerns remain elsewhere. **Known referent risk:** `I can leave a wrong answer feeling smarter` can cast the operator as learner, rather than helping learners. A model advisory pass does not establish correct voice/recognition. |
| ledger | No-dashboard boundary missing initially; **way off → unusually high** after answer 1. Empty queue at round 1; second fixed answer became added words. | No-dashboard boundary present from first board. Answer 1 retains **its way off**, failure override and not discovering failures days later. | First question asks whether failure belongs in the same check. Still empty at round 1; second fixed answer still becomes scope, then is skipped. Comparative **usual week / no targets is NOT incorporated**. Completion phrase withheld after pending/skipped scope. |
| voice | Both mechanisms retained, but forced priority question. | Talking faster when tired / terminal feels like work / both possible remain; outcome/proof/boundaries open. | **Do you want to investigate whether voice, starting dumps, or both is the main problem, or are you already aiming to make night-time dumping easier regardless of the cause?** Compatible possibilities retained; whether this clarification is useful requires operator judgment. |
| operator goal | Core lifecycle/future/quality aims retained; board versus end-to-end question. | Composed goal retains human-first, human taste/judgment, full lifecycle to human-review-ready PR, codebase health, compounding and next-change ease. | **What must this system never take out of the human's hands or get wrong?** Boundaries flagged open; no mandatory separation of compatible board/end-to-end aims. |

All six final first boards displayed. Four final advisory passes and three flags are
log-only model judgments; flags stayed off in the UI. This is **not six fidelity
passes**, operator recognition, native rendering or injection-immunity evidence.
The trap voice/referent risk and ledger's premature semantic completeness remain
explicit evaluation limitations for human review, not hidden by parsing/test success.

### Ledger fixed sequence

1. `a failed job should override everything. cost only matters if its way off` + F2:
   all parts present, model `open=[]`, no question, completion phrase available.
2. `better than my usual week. no targets` + F2: because there is no question, the
   current addition path asks **Should these added words be part of this goal, or
   stay separate?** No provider call; comparative text is not in the reading.
3. F8 skip: no active question; scope remains unresolved/excluded; **not ready**.

First empty-question round is **1 before and after**, not evidence that the entire
intent was understood. Final empty queue is after round 3, but the new readiness
predicate is false. All three actions were performed in this final live journey;
no checkpoint continuation was needed. No confirm/helper call.

## Preserved initial after attempt and bounded rerun

`initial/` retains the first implementation's five displayed first boards and ledger
failure, including raw output, source hashes, screenshots and decisions. Exact ledger
error: **`Unreadable board response: trailing comma at line 8 column 3`**. Its three
fixed actions were not performed because there was no usable board. That run also
exposed internal extra sentences and `I can I want…` from full-person clauses.

Those observations led to generic clause instructions and deterministic surface
normalization (tested for strings/escapes/decimals, not a provider repair). The failed
attempt was not promoted to a success or erased. Final six-input rerun used current
source and the same low-thinking model. No source-specific examples were added.
Historical old-board equality tests were replaced by safe raw-display tests; they do
not infer parts. A separate final-capture replay test exactly reproduces all seven
final native boards without a provider or helper.

## Provider accounting

| Batch | Shaping | Log-only advisory | Total |
|---|---:|---:|---:|
| Phase 1 baseline | 7 | 7 | 14 |
| Initial Phase 2 attempt | 6 | 5 | 11 |
| Final Phase 2 rerun | 7 | 7 | 14 |
| **Shared total** | **20** | **19** | **39 / 40** |

**One call remains unspent; no further provider work is planned.** Both Phase 2 runs
used 25 of the permitted remaining 26 calls. `../call-trace/calls.jsonl` is cumulative,
indices 0–38; first fourteen arguments/inputs are exact copies of baseline trace,
not a reset. All actual calls verify Luna and explicit `--thinking low` with isolated
Pi flags. The wrapper enforces the absolute shared ceiling before executing Pi.
The final run needed fewer calls than its maximum because ledger answer 2 was local
scope, so no advisory was denied by the budget. These are Pi/provider invocation
counts, not claims about invisible transport-level retries inside the provider.

## Validation and commands

CI's prior main run `38018152733` resolved **Rust 1.99.0**, not local default 1.98.0.
Installed the explicit 1.99.0 toolchain without changing default or installed Tinkery.
Strict verification uses it; `single_element_loop` is covered by `-D warnings`.

```
git switch -c think-goal-shape
cargo +1.99.0 fmt --all --check
cargo +1.99.0 test --workspace --all-targets --locked
cargo +1.99.0 test --workspace --doc --locked
cargo +1.99.0 clippy --workspace --all-targets --locked -- -D warnings
cargo +1.99.0 build --locked
SEED_ME_TEST_SKILL=<official pinned SKILL.md> python3 tests/<each suite>.py
python3 docs/evidence/2026-10-09-goal-shape/reversion-probes.py
git diff --check
```

`build-command.txt`/`run-command.txt` contain the exact external live-eval compile/run
commands and temporary `--seed-session-root`. Evaluation uses real current `PiHost`
and submit/apply plus Ratatui test-backend captures, not a native operator walkthrough.
Official-helper confirmation tests use isolated TEST sessions only; live evals do
not confirm anything. Saved before/after hashes verify the real ledger, launcher and
installed binary unchanged. Initial/final eval TEST roots remain empty.

Validation logs are under `../validation/`; fresh-copy reversion logs/results under
`../probes/`. Resolved compile/test/Clippy/format failures are retained, including
`error[E0616]`, delimiter errors from fixture migration, `useless_borrows_in_formatting`
and `field_reassign_with_default`. No tests were removed to hide those failures.
The retired equality/old-freeform-count assertions were replaced for the approved
no-legacy/one-targeted-question contract; source/history/consent checks remain.
The full Python rerun also exposed `AssertionError: 2 != 0` in the census's global
P=0 assertion after the new genuine syntax-failure artifacts arrived. Scoped that
assertion to its original historical corpus; preserved the specific readable-JSON
S classification checks and all-corpus no-unclassified guard. The native trace
contains one failed ledger shaping attempt, not two provider failures.

Final local verification and exact-head CI status are reported in the PR handoff.
No merge or installed rollout is authorized. Pause after opening the PR.
