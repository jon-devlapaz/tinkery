status: confirmed for intake
# Seed: When s stops a worker, a cap stop is recorded only if the worker's log shows the cap marker, and the c key is removed, for the Tinkery operator who presses s on a worker.
revision: r1
Confirmed by: "confirm; caller-declared declared: agent; typed confirm at this terminal; declaration is not verified identity" (revision r1, 2026-11-11). Not approved for implementation.
date: 2026-11-10

## You are confirming
Goal: when s stops a worker, record a cap stop only if the worker's log shows the cap marker, and remove the c key, for the Tinkery operator who presses s on a worker (Fix #813).
Builds: s decides the recorded reason from the worker's log, and c is removed from the key bar, help and key handling.
Does not build: no change to tink-sdlc's --stopped-by interface, to what the word means, to how orientation reads it, or to Watch's wording; no model calls.
Proposed first slice: replace the cap journey's c press in tests/worker_terminal.py with s over fixture logs with and without the marker, then make that pass.
Unknowns: which log s reads when a step has several launch or resume logs, and what s records when the log is unreadable; neither is settled.

## Problem statement
Today the worker panel has two stop keys. s records a stop by the operator. c records a stop by the response cap whether or not the worker's log shows a cap stop (src/menu/worker.rs:232-233). An operator can therefore record a cap stop that never happened. Orientation and Watch then read that wrong reason as fact. This is #813.

## Proposed outcome
Pressing s records the stop reason from the worker's log. A log showing the cap marker is recorded as stopped by cap, and a log without it is recorded as stopped by operator. c is gone from the key bar and help, and pressing it does nothing. The change is a subtraction. The operator's supplied rules say the marker is 'pi-launcher: response cap N reached;' with responses >= cap. Those rules also say tests use fixture logs only, with zero model calls.

## Acceptance criteria

a worker whose log shows the cap marker is recorded as stopped by cap on s; a worker whose log does not show the cap marker is recorded as stopped by operator on s; c is gone from the key bar and help and does nothing; a regression test drives the panel input against fixture logs with and without the marker and fails when reverted; python3 scripts/full-check.py passes
check: `python3 scripts/full-check.py`

pressing s on a fixture worker whose log has the marker records stopped\_by cap, and pressing s on one without it records stopped\_by operator, driven through panel input
not yet runnable: tests/worker\_stop\_marker\_terminal.py does not exist yet; it must reuse the worker\_terminal fixtures, press s against fixture logs with and without the marker, and assert the recorded stopped\_by. The setup mirrors scripts/python-tests.py and is not a verified invocation; run it under a temporary HOME as the full check does.

pressing c does nothing and c appears in neither the worker key bar nor the help
not yet runnable: The same new test file has no c-key assertion yet; it needs one, with the same setup as the check above.

a person opens the worker panel on a fixture run with and without the cap marker, presses s, and reads the recorded reason in the panel and in Watch
you judge: Open the worker panel on fixture runs with and without the marker. Press s, then read the reason in the panel and in Watch. The reason should match the log and the wording should be unchanged from before.

## Affected users and systems
The Tinkery operator who presses s on a worker. In src/menu/worker.rs, the key handler maps c to Action::StopCap and the key bar and help text name c. In src/worker.rs, the Action enum has StopOperator and StopCap, and act() passes --stopped-by cap or operator to tink-sdlc. The existing cap-marker parsing is in src/worker/orientation.rs, which reads the pi launch log. Tests: tests/worker_terminal.py (the 'cap' journey presses c), src/worker/tests.rs (Action::StopCap at line 1064), and cap_review, which writes fixture logs containing the marker. Docs that mention c: docs/first-task.md:107 and docs/design/approve.md:214.

## Constraints and boundaries
Must: record a cap stop only if the worker's log shows the cap marker. Must: remove the c key. Out of scope, per the operator's supplied rules: tink-sdlc's --stopped-by interface, the meaning of the word, how orientation reads it, and Watch's wording. Builder rules supplied by the operator: fewer, deeper modules and smaller interfaces; check data once where it comes in; write one-off logic where it is used; skip try/catch where possible; name each public interface added and why it is narrow; name the architecture diagnostics moved. The builder commits on the current branch and does not push. Nothing is implemented by this seed.

## Decisions (from the ledger)
- goal [delegated] When s stops a worker, a cap stop is recorded only if the worker's log shows the cap marker, and the c key is removed, for the Tinkery operator who presses s on a worker. Why: Fix #813. Done when: a worker whose log shows the cap marker is recorded as stopped by cap on s; a worker whose log does not show the cap marker is recorded as stopped by operator on s; c is gone from the key bar and help and does nothing; a regression test drives the panel input against fixture logs with and without the marker and fails when reverted; python3 scripts/full-check.py passes. Must: record a cap stop only if the worker's log shows the cap marker. Must: remove the c key. (source: Goal confirmed in Tinkery with typed confirm and Enter; caller-declared attribution: declared: agent; typed confirm at this terminal; declaration is not verified identity; goal only, not seed or implementation approval.)
  Delegation (caller-declared, not authenticated): {"agent": "agent"}
- probe-real-sample [delegated] Operator's relayed line, #813, and the supervisor's standing rules for this run: Scope is exactly #813: make it a subtraction; remove the c key; when s stops a worker record cap if the step's log shows the launcher's cap marker 'pi-launcher: response cap N reached;' with responses >= cap, and operator otherwise; update the help text and key bar; it does not change tink-sdlc's --stopped-by interface, the meaning of the word, how orientation reads it, or Watch's wording; zero model calls in tests, fixture logs only. Done when: (1) A worker whose log shows the cap marker is recorded as stopped by its cap when s is pressed; a worker without the marker is recorded as stopped by the operator. (2) c no longer appears in the worker key bar or help, and pressing it does nothing. (3) A regression test drives the panel through its input against a fixture log with the marker and one without, and checks the reason that was recorded; with the check reverted, the test fails. (4) Full zero-call verification passes (python3 scripts/full-check.py). Builder rules: Fewer, deeper modules, smaller interfaces. Check data once where it comes in, then just use it. Write one-off logic right where it's used instead of splitting it into a function ahead of time. Skip try/catch wherever you can. Name each public interface added and why it is narrow. Name the architecture diagnostics moved. The builder commits its work on the current branch before ending its turn; it does not push. Human Taste judgment: a person opens the worker panel on a fixture run with and without the cap marker, presses s, and reads the recorded reason in the panel and in Watch. Sample: no sample, fixture logs only per #813. (source: The agent answered with these words in Tinkery Shape.; caller-declared declared: agent; typed confirm at this terminal; declaration is not verified identity)
- probe-pre-mortem [delegated] skip (source: The agent answered with these words in Tinkery Shape.; caller-declared declared: agent; typed confirm at this terminal; declaration is not verified identity)
- obs-menu-keys-813 [evidence] s maps to Action::StopOperator and c to Action::StopCap with no log check. The key bar line includes 'c cap stop'. The help text says c records that its cap stopped it. (source: read-only investigation in Tinkery Shape)
- obs-act-813 [evidence] StopOperator and StopCap share one branch that passes --stopped-by cap or operator to the step complete command. (source: read-only investigation in Tinkery Shape)
- obs-marker-813 [evidence] Logs are found as <step>.launch-<ts>.pi.log. The marker is 'pi-launcher: response cap {n} reached;' taken from the step contract's max_responses. It counts only when responses >= cap. Orientation reads the recorded stopped_by, not the log, to decide whether to look for the marker. (source: read-only investigation in Tinkery Shape)
- obs-tests-813 [evidence] cap_review writes fixture logs with the marker. The 'cap' journey presses c and asserts stopped_by == 'cap'. src/worker/tests.rs iterates StopOperator and StopCap. python-tests.py runs every tests/*.py file. (source: read-only investigation in Tinkery Shape)
- obs-docs-813 [evidence] Both say s stops a worker and c records a cap stop. (source: read-only investigation in Tinkery Shape)
- obs-fullcheck-813 [evidence] full-check.py runs the single bash block in AGENTS.md between the full-check markers under a temporary TEST HOME. The block builds, runs Rust tests, node tests, python-tests.py, fmt and clippy. (source: read-only investigation in Tinkery Shape)
- inf-which-log-813 [evidence] Launch and resume logs for one step can coexist. The builder has to pick one, and the fixtures can exercise this. (source: read-only investigation in Tinkery Shape)
- unk-real-logs-813 [evidence] This investigation has not established that real logs carry the marker in exactly the fixture form. (source: read-only investigation in Tinkery Shape)

## Assumed (not confirmed by you)
- none

## Knowledge map
What we know, with proof (observed this session; re-check if these files change before implementation): src/menu/worker.rs:232-233 maps s to StopOperator and c to StopCap. The key bar at line 280 and the help text at line 343 name c. src/worker.rs:546-556 turns the two actions into --stopped-by operator or cap. src/worker/orientation.rs:251 builds the marker as 'pi-launcher: response cap {n} reached;' and lines 253-263 return a result only when responses >= cap. The log files are named <step>.launch-<ts>.pi.log (collect_logs, orientation.rs). tests/worker_terminal.py:743-749 presses c and asserts stopped_by == 'cap'. scripts/python-tests.py:37 runs every tests/*.py file, so a new test file is picked up automatically.
What we know we don't know: (1) which log s reads when a step has several launch and resume logs; the builder decides, and it can read cap_review's resume cases and orientation's launch selection; (2) what s records when the log is missing or unreadable. The supplied rule says 'operator otherwise', so the builder should follow that. (3) Whether to reuse the orientation parsing or write a short check in act(); the builder decides.
What we have not read: scripts/ci-scope.py, the docs/design text around the c mentions beyond the cited lines, src/watch tests beyond line 512, and the tests/worker_terminal.py fixture code for the sdlc stub beyond lines 140-190.
What could surprise us: a resumed step whose latest log has no marker but an earlier one has; a marker line appearing before the response count reaches the cap, which the supplied rule counts as not a cap stop; the 'cap' journey in tests/worker_terminal.py breaking when c is removed. A concrete signal that we were wrong: the new test passing with the log check reverted.

## Evidence and uncertainty
Grounded in:
- AGENTS.md
- docs/design/approve.md
- docs/first-task.md
- scripts/full-check.py
- scripts/python-tests.py
- src/menu/worker.rs
- src/worker.rs
- src/worker/orientation.rs
- src/worker/tests.rs
- tests/worker_terminal.py

Provenance unavailable: reported reads do not verify filesystem origins or unrecorded reads. Some recorded findings have no attributable read path.

Checked:
- Caller-declared authorization, not authenticated identity: declared: agent; typed confirm at this terminal; declaration is not verified identity (observed 2026-11-11)
Checked by read-only file tools in /Users/jondev/dev/worktrees/tinkery/worker-cap-stop:
- src/menu/worker.rs:229-235, 280, 343: s maps to Action::StopOperator and c to Action::StopCap with no log check. The key bar line includes 'c cap stop'. The help text says c records that its cap stopped it. (observation; scope: src/menu/worker.rs key handling, key bar and help) (observed 2026-11-11)
- src/worker.rs:145-147, 546-556: StopOperator and StopCap share one branch that passes --stopped-by cap or operator to the step complete command. (observation; scope: src/worker.rs stop action) (observed 2026-11-11)
- src/worker/orientation.rs:154-170, 245-263: Logs are found as <step>.launch-<ts>.pi.log. The marker is 'pi-launcher: response cap {n} reached;' taken from the step contract's max_responses. It counts only when responses >= cap. Orientation reads the recorded stopped_by, not the log, to decide whether to look for the marker. (observation; scope: src/worker/orientation.rs marker parsing) (observed 2026-11-11)
- tests/worker_terminal.py:535-595, 743-751, 780-790; src/worker/tests.rs:1064; scripts/python-tests.py:37: cap_review writes fixture logs with the marker. The 'cap' journey presses c and asserts stopped_by == 'cap'. src/worker/tests.rs iterates StopOperator and StopCap. python-tests.py runs every tests/*.py file. (observation; scope: existing tests touching the c key and StopCap) (observed 2026-11-11)
- docs/first-task.md:107; docs/design/approve.md:214: Both say s stops a worker and c records a cap stop. (observation; scope: docs mentioning c) (observed 2026-11-11)
- scripts/full-check.py:17-24; AGENTS.md:99-124: full-check.py runs the single bash block in AGENTS.md between the full-check markers under a temporary TEST HOME. The block builds, runs Rust tests, node tests, python-tests.py, fmt and clippy. (observation; scope: the full-check command) (observed 2026-11-11)
- src/worker/orientation.rs:154-170 and tests/worker_terminal.py cap_review cases: Launch and resume logs for one step can coexist. The builder has to pick one, and the fixtures can exercise this. (inference; scope: how s could pick the log; limits: Does not establish which of several logs the operator expects; no multi-log resume fixture was read for this.; supported by: obs-marker-813, obs-tests-813) (observed 2026-11-11)
- No real pi launcher log was read; fixtures only, per the operator's rule.: This investigation has not established that real logs carry the marker in exactly the fixture form. (unknown; scope: real launcher output) (observed 2026-11-11)

## Suggested first slice
Proposal only, not binding. From the repository root, run: python3 tests/worker_terminal.py. This is the existing journey file whose 'cap' case presses c and must change. The smallest step is to rewrite that case to press s against a fixture log that contains the marker and against one that does not, and to assert the recorded stopped_by. After that comes the s-time log check in src/worker.rs or src/menu/worker.rs, then removing c.

## Risks and verification
Risks: a log check that is too loose records cap on a log that does not show it, and one that is too strict records operator for a real cap stop. Reading the wrong log of several would give the same errors. Removing c leaves stale mentions in docs/first-task.md and docs/design/approve.md. Removing Action::StopCap without updating src/worker/tests.rs would break the Rust build. Proposed checks are listed below and none has been run. Intended red: with the log check reverted so s always records operator, the marker case's stopped_by == 'cap' assertion should fail. If s always recorded cap, the no-marker case's assertion should fail. Where we did not look: the real tink-sdlc runtime and real pi launcher logs, CI workflow files, and the visual layout of the panel at narrow widths.

## Open questions and deferrals
No operator decision is blocking. Deferred to the builder as implementation choices: which log s reads when there are several, and reuse versus a short in-place check. Revisit if a resumed-step fixture shows a different log would change the recorded reason.

## Owners and next steps
The operator confirms the seed. The builder then implements it under the supplied rules and commits without pushing. The unread items above are for the builder to read before editing. The human taste judgment (opening the panel on fixture runs with and without the marker, pressing s, and reading the recorded reason in the panel and in Watch) is for the operator. Guardrail for the repeat risk of stale c mentions: grep the repository for 'cap stop' and 'c records' after the change.

## Downstream handoff
The session folder is the handoff: seed-contract.md together with ledger.json, seed-confirmation.json and seed-checks.json. A consumer checks tinkery seed status --json <session>; handoff.verified proves the content only, not who confirmed it. This is discovery input, not an implementation plan, approved specification or review receipt. Later plans should keep the constraints above and surface any conflict instead of replacing a decision.

