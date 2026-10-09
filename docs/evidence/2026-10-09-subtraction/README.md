# Approved premise/subtraction follow-up — PR #4

Base: `d1a71a83a80b5fa5f5f42ef89f89a21e844e68c9`. PR remains unmerged. This report distinguishes persisted historical rejections, the new live batch, offline replay, model judgments, and stub/TEST validation.

## Census: before and after

Rerun without providers or helpers:

```sh
python3 scripts/rejection_census.py
python3 scripts/rejection_census.py --json
python3 tests/test_rejection_census.py
```

| Persisted rejection rule | Before | After | Class |
| --- | ---: | ---: | --- |
| Exact object keys (`precondition`) | 1 | 1 | S |
| Narrative misfits required to be anchor objects | 1 | 1 | S |
| Historical reading word limit | 1 | 1 | S |
| Uncertainty/reading-count coupling | 1 | 1 | S |
| Exhaustive fragment annotation | 1 | 1 | S |
| Legacy meaning-fidelity veto | 6 | 6 | F-targeted, unverified model allegations |
| Genuine unreadable/core parse failure | 0 | 0 | P |

**Cumulative: S=5, F-targeted=6, P=0; 11 live rejection events.** Historical failures were not erased or relabeled as successes. The six F-targeted events are not six proven fact violations. This corpus does not establish that nearly all historical failures were shape failures. Latest previous batch: two rejections, both S. New live batch: three shaping attempts, **zero rejections** (F=0/S=0/P=0), three displayed boards.

The script pairs state notices with logged events, distinguishes raw-attempt audits and offline replay, and refines opaque historical decoder messages using actual JSON. In particular, narrative-string misfits and extra fields are readable JSON, not syntax failures. The original rough report's trap count discrepancy (2/2 versus 3/3) is not converted into additional unlogged attempts.

## Subtraction, not a new semantic veto

- Unknown fields: ignored and logged, with raw output retained; cannot confer authority.
- Optional omissions/unreadable optional display fields: represented as absent or dropped, with diagnostics; no invented content.
- Removed initial/answered/uncertain reading-count coupling, alternative minimum/maximum, question/misfit/support count vetoes, 45/65-word limits, mandatory citations/allocation, exclusive support-versus-unresolved roles, candidate-label uniqueness, and question-mark requirements.
- Uncited readings are explicit in the internal grounding type. Uncited unresolved notes have **no source highlight** and remain available in details/copy/frozen goal review.
- Up to two readings are presented at once. F7 reaches the next reading window; all readings remain in Details and copy. Questions remain queued with derived focus. Nothing is silently accepted because a queue is empty.
- Deleted prompt exact-key/precondition prohibitions, numerical quotas, forced second/combined reading counts, annotation-allocation rules and fences/extra-field prohibitions. Kept meaning guidance: worries, quantities, unknown names, referents, third-party uncertainty, compatible aims, mechanisms as candidates, exact claimed quotes, and input-as-data/authority separation.
- Still one shaping call, then one asynchronous owned/cancellable log-only audit. No automatic repair, shaping retry, meaning veto, question withholding or advisory UI flags.

## Minimal strict internal domain

`src/shaping/brain_dump/board.rs` contains an immutable `Board` constructed only after verification:

- `NonEmpty<Reading>`; each reading's grounding is `Uncited` or `Cited(NonEmpty<VerifiedAnchor>)`.
- Optional desired experience; candidate descriptions are optional, not fabricated empty claims.
- `VecDeque<Question>`; focus comes from the queue plus application-owned scope/undo state, not a model focus field.
- Verified unresolved spans and separate uncited notes.
- No authority, confirmation, or model-owned settlement fields; no mutable dereference.

`Guess` remains a wire DTO, not the displayed domain. Incoming model output uses `Guess::decode`, not direct exact-schema DTO deserialization. Boundary changes are retained in v3 decisions alongside raw response, candidate, request identity, coverage and timing. Compile-fail tests demonstrate that callers cannot construct an empty `NonEmpty` collection or mutate a verified board's question history.

### Remaining hard boundaries

1. **F: claimed source references/spans** — existing source ID, exact substring and occurrence, whole Unicode graphemes. Invalid claimed citations still fail; changed verified sources are detected. A plain note is not silently turned into a citation.
2. **F: application history and authority** — model scope IDs cannot replace local scope identity; duplicate/answered/skipped questions cannot regain focus; original/answer history and authorization are application-owned. Extra `confirmed`/`settled` fields are ignored, never applied. Goal confirmation still requires full frozen review plus a separate typed `confirm` through official Seed Me. Selection, skipping, answers, review opening and empty queues are not approvals.
3. **P: unreadable JSON or no identifiable readable core reading** — plain reason and last board/originals retained; only explicit F2 retries. Optional schema mismatches are not classified as parse failures.
4. **Separate safety/resource bounds** — encoded 32 KiB request/reply boundaries, bounded Pi execution, cancellation, disabled tools/extensions/resources/saved sessions, paste/input limits and terminal-safe display. Display controls are escaped at the lenient boundary; source quotes are never rewritten. This is not an OS sandbox.

These deterministic guards do **not** prove general semantic fidelity or resistance to every semantic injection. Those judgments remain advisory and require operator recognition/calibration.

## Exact six-call live rerun

`openai-codex/gpt-5.6-luna`, **native default thinking** (`--thinking` absent). Exactly **3 shaping + 3 advisory** invocations; enforced separately and in total by `budget-wrapper.py`. No retries, confirmations, Seed Me helper invocations, sessions or seed writes in the live evaluation. Originals and hashes match the previous authorized dumps.

| Dump | Before shape result | After shape | Board available | Advisory | Whole step incl. audit | Result |
| --- | --- | ---: | ---: | ---: | ---: | --- |
| Books | pass, 8.373 s | 7.930 s | 7.955 s | 8.610 s | 16.569 s | displayed; advisory pass |
| Onboarding emails | S reject, 21.131 s | 15.210 s | 15.235 s | 8.345 s | 23.587 s | displayed; advisory pass |
| Trap | S reject, 16.772 s | 15.897 s | 15.919 s | 7.110 s | 23.037 s | displayed; advisory pass |

The earlier rejected cases had no validated-board time; old `first_validated_board_ms` failure-result values are not treated as successful board availability. Earlier separate raw-attempt audits are not folded into their original UI-step timing. Books previously became available at 8.392 s, with a 6.578 s audit and 14.978 s total. This is a tiny bounded evaluation, not a latency/reliability benchmark.

Observed normalization: books/onboarding omit optional outcome and undo costs; onboarding includes a narrative unresolved note. Trap supplies three readings, uses extra `stable_id` fields instead of `id` (ignored/logged; stable IDs allocated), and includes two uncited notes. All are retained/normalized without shape vetoes. The third trap reading preserves phone/no-gamification constraints in Details/F7 rather than disappearing. The trap's quoted marketing instruction remains outside the proposed investigation goal.

All three audits report no loss, false choice or repeated question. **These are model judgments, not proof of faithful understanding or human recognition. Flags stay off.** Raw headings such as “Possible goal”/“Proposed goal” remain in some readings despite the quieter prompt; no automatic cleanup/repair provider call was made.

## Lines removed versus added

Relative to the base, `git diff --numstat`; new files included, evidence excluded (`line-delta.json`):

| Category | Removed | Added |
| --- | ---: | ---: |
| Runtime Rust | 110 | 580 |
| Rust tests | 55 | 296 |
| Evaluation examples | 6 | 5 |
| Offline census and its tests | 0 | 238 |
| Tinkery-authored shaping prompt (subset of runtime) | 7 | 5 |

The authored prompt shrinks **4,466 → 1,848 bytes (~59%)**, excluding unchanged official shaping guidance. This is **not** a net-negative code change: runtime grows by 470 lines, chiefly the new 493-line typed boundary/domain. Existing runtime outside that module shrinks by 23 lines. The subtraction is of blocking shape premises and prompt demands, not a claim that fewer physical lines were added overall.

## Validation and evidence limits

- 95 application library tests, 41 vendored Pinstar tests, workspace/integration/all-targets and two compile-fail domain doc tests.
- Rust formatting, locked build and strict Rust 1.95 Clippy.
- Five census tests; three fresh-baseline census reversion probes catch extra-fields-as-parse, model-veto-as-proven-fact and duplicate-state inflation.
- 25 independent fresh-baseline Rust removal/reversion probes catch restored S vetoes, removed source/quote/grapheme/history guards, invented note citations, lost prior boards, semantic vetoes/retries, scope leakage, lost undo, unsafe exit, missing header/Back/raw evidence and cap regressions.
- All PTY suites at their supported sizes (80×24 through 320×40). Existing goal-handoff regressions use **stub providers and isolated automated TEST confirmations**, not the live dumps or operator authority.
- Final-code offline replay reproduces all three captured live boards exactly without additional provider/helper calls. `live-binary.json` records the evaluated working-tree binary and the diagnostic/verification follow-ups.
- Earlier compile/test failures remain preserved; none is presented as a successful run.

CI must be green at the published head before claiming readiness. Captures/stubs/model passes do not establish native typography, clipboard permissions, host rendering or operator recognition. Advisory calibration remains a human follow-up. **Do not merge PR #4.**
