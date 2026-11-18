# Change brief

Source: https://github.com/jon-devlapaz/tinkery/issues/905
Issue body sha256: 5bba21544107504dc2acf22aed08bdde771b081963bb198d7b6ee7a884461ccf
Title: Reuse Approve test fixtures without losing isolation

The approved issue below is this run's brief, unchanged. Each Done-when line is one item in `checklist.json`.

---

### Primary area
approve

### Kind
enhancement

### Goal
Reduce repeated fixture preparation in Approve's Python terminal journeys without losing behavior coverage or case isolation.

### Evidence
Inspected main at ca421e23896f61e226c9d011adccc0f4e43ce9a2 on 2026-11-17.
- PR run https://github.com/jon-devlapaz/tinkery/actions/runs/44016198767: approve_terminal.py took 174.9 seconds on Ubuntu (pr tier, four workers). The enclosing Python job took 238 seconds.
- Main run https://github.com/jon-devlapaz/tinkery/actions/runs/44017486737: approve_terminal.py took 212.4 seconds on macOS (full tier, three workers).
- The file runs 68 journey variants. In 56 variants, it creates the same five-session fixture family with the actual Rust ledger adapter. The ready case adds tall goal text. Each journey otherwise owns its temporary TEST root, application process, writable files and case-specific changes.
- Current CI logs already expose setup and per-file times. The implementation plan's optional timing patch is not needed for this target.
- These observations identify repeated setup, not its measured saving. No cost reduction or productivity improvement is claimed.
- Completed #896 retains its scope and remains closed. No open duplicate for this fixture-reuse slice was found.

### Scope and boundaries
Generate the ordinary and tall-goal fixture families once per test process using the existing real Rust ledger helpers. Copy the selected template into each journey's fresh TEST root. Keep templates private and unchanged. Preserve copied session directory names and ledger/confirmation identities.

Allowed code files: tests/approve_terminal.py and one focused test file for fixture isolation and discovery. Shared fixture helpers, production source, workflow policy, provider settings and factory templates remain unchanged.

Preserve all 68 cases, assertions, terminal sizes, color variants, fault mutations, per-case process ownership and file-unchanged assertions. Preserve the existing PR/full tiers, batching, worker counts, bounded waits, failure output and TEST HOME guards. No test deletion, automatic retry, provider calls, new dependency, install or delegated agent.

Trial budget: up to three baseline and three candidate standalone Approve journey runs on this local Mac, plus focused regressions, one relevant production-only fault probe and required full verification. Freeze the baseline revision and candidate diff. Target at least 20% lower standalone journey median with no lost contract or new unexplained failure. Report raw observations and small-sample limits; this is not a fleet-wide CI claim. Retain failed observations and stop at the budget.

### Done when
- [ ] Ordinary and tall-goal templates come from the existing real Rust ledger helpers and are copied into fresh TEST roots for each applicable case.
- [ ] All 68 original journey variants and their assertions remain; each case owns its process and writable copy.
- [ ] Focused regressions prove mutations and deletion in one copy cannot change a later copy or the template, including confirmed, stale, draft and question states.
- [ ] A relevant production-only fault still fails the unchanged journey assertions; source and executable are restored and the normal journey passes again.
- [ ] The bounded comparison records every attempted baseline/candidate run, revision, local environment, raw durations and limits; it meets the preregistered 20% journey-median target or reports an explicit no-keep conclusion.
- [ ] Required full verification passes on the committed candidate, with Rust, Node and Python results reported separately. CI passes before any merge decision.
- [ ] A PR links this issue and the evidence. Independent human review remains required; this task does not authorize merge or acceptance.

### Context and dependencies
Authoritative contracts: AGENTS.md, docs/yohaku.md, docs/design/approve.md, tests/approve_terminal.py, tests/shape_terminal.py, tests/session_fixture.py, tests/test_python_tests.py and scripts/python-tests.py.

Planning reference: the operator's local document `tinkery-ci-evidence-efficiency.md`, Slices 0 and 2. No operator sessions or transcripts are published. Measurement issues #820, #828, #830, #831, #832, #834 and #835 keep their separate approval boundaries.

### Open decisions and permissions
The operator requested implementation and selected “Issue and PR (Recommended)” in this Pi session on 2026-11-17 after seeing the target, trial and delivery scope. That choice authorizes issue publication, the isolated implementation, the bounded local trial and required checks, staging/committing, pushing and opening a PR.

No merge, installs, delegated agents or model calls are authorized. Product/provider call budget: zero. No historical #896 budget or permission is reused. Human review and acceptance remain pending.

---

## Verification and delivery order

The checklist covers the five slice-specific conditions above.
The sixth condition is enforced by the existing configured full check through `sdlc.py verify`, bound to the committed candidate, rather than self-attested in the checklist.
CI must still pass before any merge decision.
The seventh condition is a delivery obligation after passing committed-revision verification, not a prerequisite for opening that PR.
It remains required by the issue: link the PR and evidence, retain independent human review, and do not merge or record acceptance.
This ownership and ordering changes no product protection or approved delivery scope.

