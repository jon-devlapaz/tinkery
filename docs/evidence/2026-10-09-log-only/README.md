# Approved log-only dogfood eval

Operator authorization: exact books/onboarding inputs relayed in chat, trap copied verbatim from item 1 of `Tinkery dogfood report 2026-10-09.md`; `openai-codex/gpt-5.6-luna`, native Pi thinking default (no `--thinking` override), at most three shaping plus three advisory calls, no goal confirmation. Actor: assistant-driven TEST, **not operator recognition**.

## Decisions and measured timings

| Input | Shaping result | Shape time | Advisory decision | Audit time |
|---|---|---:|---|---:|
| Books | v1 schema/spans/history pass; board available at 8.392s | 8.373s | log-only pass; no whole-board loss identified by the model | 6.578s |
| Onboarding emails | reject: unsupported `precondition` keys in candidate objects | 21.131s | log-only flag: possible false choice between compatible aims | 11.705s |
| Trap | v1 reject: `uncertain:true` incorrectly coupled to requiring two readings | 16.772s | log-only flag: possible loss of red-X context and tentative quiz-engine rebuild | 8.886s |

These audit answers are **model judgments, not established losses or authorization**. In particular, a tentative rebuild may be peripheral, and volume/timing routes may be a genuine unresolved choice. Operator calibration remains necessary. AI is covered in the trap reading as a possible route; its absence from outcome did not trigger an acronym veto.

Books' measured initial step was 14.978s including the audit, with usable-board availability before it. The other initial steps ended in structural rejection (21.168s and 16.812s), **not usable boards**. Their audits were separately completed later against the unchanged raw attempts. Do not add those audit times to the initial UI-step durations or claim that all three live inputs succeeded.

The original rejected-case JSON fields named `first_validated_board_ms` recorded time to **failure result**, not a validated board; retained verbatim as failed-attempt evidence. The harness now emits null for rejected boards and a separate `shaping_result_ms`.

## Corrections and remaining boundary

The eval exposed an old semantic coupling in validation: a model's uncertainty flag was forcing an invented second reading. Check bundle v2 permits one or two initial readings independently of that flag, while retaining exactly one combined reading after answers, word/count bounds, exact spans, unsafe-text and authority-field rejection. The **same original 44-word trap candidate passes v2 offline structural revalidation** (`raw-attempt-audits/trap/decisions.json`). This is **not another live shaping run** or evidence of operator recognition.

The original onboarding candidate **still fails strict schema validation** because of extra `precondition` fields. These were not stripped, normalized or accepted. The shaping prompt now explicitly requires exactly the four permitted candidate fields and putting preconditions into their existing strings. Fresh successful shaping with that prompt remains unverified: the authorized budget is exhausted. The unchanged failed raw attempt is human-readable in Why, non-confirmable, and retained below.

**No advisory flags are enabled.** Semantic audits are log-only and cannot reject, repair, mutate readings, withhold questions, or block confirmation. Structural errors remain explicit. No automatic shaping retry occurred.

## Evidence and scope

- `initial-eval/*/`: exact originals, original shaping/audit decisions, raw responses, declarative Ratatui captures at 80×24–320×40, timings and confirmation=false.
- `raw-attempt-audits/`: two separately authorized advisory evaluations of rejected raw attempts, with no shaping rerun or acceptance; v2 offline schema replay recorded separately.
- `call-trace/`: **six Pi invocations, exactly three shaping plus three advisory**, all without a thinking override; full request JSON and CLI arguments. The wrapper reserves a slot before exec and refuses invocation seven. Underlying provider billing/retry internals are not measured.
- Canonical skill: merged `jon-devlapaz/seed-me` `1a2f31a136d53bceff51ae8c2e0e787c1a8a90bb`, read-only guidance. **Zero helper invocations, official sessions, goal confirmations, seed writes or implementation approvals in this eval.**
- Initial failed unit/PTY logs are retained. The first PTY failure was a stale test assumption: answers now select their own original, so extraction explicitly switches back first. The new PTY's first failure expected the nonexistent heading “Originals” instead of “Original 1 / dump”; it was not an operator host diagnosis.
- No claims about native typography, clipboard permissions or live-model reliability follow from the stub tests or buffer captures.

Original byte hashes (no added newline):

- Books: 212 bytes, SHA256 `66998954bc491a17b79178d276a19bc8441cf1e82de8a7e8642e2fbf51032864`.
- Onboarding emails: 236 bytes, SHA256 `ad8de86dac1e152532889e4a6f1268af01c6179efd541005a15a47ab8cc96cfc`.
- Trap: 836 bytes, SHA256 `09080faf77a128910d84acffb4e04d31efeb6c52d3355f84a414b21acc4e21bd`.
