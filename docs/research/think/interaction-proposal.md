# Think interaction proposal — historical, not accepted

**Historical design proposal** from the
[October 22 dogfood review](../../../findings/2026-10-22-think-dogfood.md), retained
with its evidence and audit. It is not current guidance, accepted design or
implementation permission. The #427 audit, merged in
[#429](https://github.com/jon-devlapaz/tinkery/pull/429), recommends keeping shipped
behavior unless a concrete user-visible gap is observed.

[Yohaku](../../yohaku.md) governs this proposal. The shipped
[Think interaction](../../design/think.md#interaction-contracts),
[goal-block contract](../../design/goal-block.md),
[confirmation attribution](../../design/think.md#confirmation-attribution-185)
and [visual principles](../../design/principles.md) remain authoritative until an
approved slice deliberately updates its affected decision. This proposal changes
attention order/presentation, not model intent or permission policy.

## Audit — 2026-10-25

This crosswalk compares the proposal with current contracts, code and regression
tests. “Shipped” describes mechanics, not an accepted redesign or evidence of better
usability. The October 22 dogfood is one scenario; it does not validate the
presentation hypotheses below.

| Proposal behavior | Current evidence | Status |
| --- | --- | --- |
| One focused question, with F2 as the only send action | [Think question and send contracts](../../design/think.md#questions); [question layout tests](../../../src/think/brain_dump/layout_tests.rs), including `active_question_is_readable_without_review_and_survives_resize` | Shipped. The proposed question → answer → draft hierarchy is not established by those tests. |
| Separate writing, reading, in-flight, ready and review behavior | [Think shipped experience and model-call contracts](../../design/think.md#shipped-experience); [readiness tests](../../../src/think/brain_dump/intent_tests.rs), including `ready_invites_review_without_claiming_complete_intent` | Existing behavior is specified. Proposal-specific captions, pane arrangement and a distinct revision mode remain unverified. |
| Suggested answer and reason, copied without sending or overwriting a draft | [#330](https://github.com/jon-devlapaz/tinkery/issues/330), shipped in [#332](https://github.com/jon-devlapaz/tinkery/pull/332); [suggestion implementation](../../../src/think/brain_dump/suggestion.rs) and [tests](../../../src/think/brain_dump/suggestion_tests.rs), including `suggestion_copies_without_submitting_or_overwriting` and `unaccepted_suggestions_are_recorded_but_never_become_model_context` | Shipped as separate text plus Ctrl-R. Inline ghost text and contextual Tab are not shipped. |
| Read the full question and suggestion at small sizes without losing originals or unsent text | [question and wheel tests](../../../src/think/brain_dump/layout_tests.rs); [suggestion reachability test](../../../src/think/brain_dump/suggestion_tests.rs), which covers 80×24, 100×30, 160×40 and 320×40 in colour and no-colour | Those regions are covered. The proposal’s combined question/answer/draft layout, independently clamped viewports, actual pane size and long-answer interaction are not verified. |
| Friendlier draft headings while preserving the frozen goal and originals | [Goal block](../../design/goal-block.md) owns the canonical parts and exact displayed block; [#322](https://github.com/jon-devlapaz/tinkery/issues/322) / [#328](https://github.com/jon-devlapaz/tinkery/pull/328) ship “Success criteria” in Why and copied output; [criteria tests](../../../src/think/brain_dump/readiness_tests.rs) | Accurate criteria labeling is shipped. The proposed headings are not; the goal-block schema and canonical labels remain unchanged. |
| Review framing and access to originals, answers and open intent | [Think review contract](../../design/think.md#readiness-and-review); [editing and focus contract](../../design/think.md#editing-and-focus); [goal review tests](../../../src/think/brain_dump/goal_tests.rs) | F3 review, original-pane access and unresolved-intent review are shipped. The proposal’s combined review presentation is not tested. |

### D1–D6 recommendations

These are audit recommendations, not operator decisions or design acceptance. Do not
open a feature issue from a preference alone. A later issue needs a concrete
user-visible gap and separate scoped approval.

| ID | Audit finding and recommendation |
| --- | --- |
| D1 | **Revise; defer a feature issue.** A focused question is shown and can be read and answered without opening review. The tests do not establish that placing the draft below it reduces effort or improves comprehension. Keep the shipped layout unless a specific gap is observed; if one is, scope a prototype to that gap. |
| D2 | **Revise; no implementation issue yet.** Ctrl-R already copies a separate suggestion into an empty draft; Tab/Shift-Tab already navigate writing and originals. Keep this behavior and the shipped safeguards. Retain ghost text/Tab only as an unaccepted alternative until the operator explicitly chooses whether it should replace or supplement Ctrl-R. |
| D3 | **Revise; defer a feature issue.** Reading/question overflow and suggestion/reason reachability have regression coverage, but the proposed shared scroll across question, ghost and draft, long answers, and combined layout is not proven. Keep the current scroll contract; require a concrete clipping or navigation failure before proposing a new route. |
| D4 | **Drop the proposed heading change for now.** The goal block already has canonical `Why`, `Done when`, `Must` and `Must not` parts, while “Success criteria” is correctly used in supporting display and copied output. No evidence shows the proposed heading set solves a user-visible problem; do not rename or reshape stored goal content. |
| D5 | **Revise; defer a feature issue.** The ready state already offers “Ready for your review” and F3, without claiming intent is complete. The persistent writing surface remains part of the shipped interaction; a dedicated `YOUR REVISION` mode or changed draft hierarchy is not established as necessary. |
| D6 | **Revise; keep existing access and review contracts.** Originals remain accessible through Tab/Shift-Tab, and review carries open intent. The proposal’s particular source/answer framing has no regression evidence of an access gap. Do not add a presentation slice unless a concrete review-access failure is identified. |

No usability improvement, reduced effort, or recommendation quality is established by
this audit. No application, prompt, schema, saved-data or provider behavior changed.

## Proposed direction — not accepted

**Candidate while answering: question → answer → draft.** Exact originals remain on the left;
the current question and answer sit together at the top right. When no question
is active, the draft expands and review becomes the main next action. Thinking
occupies the interaction region, not a detached footer. There is no permanent
“anything to add or change?” form when the next step is review.

## Compact answering screen

This 80×24 sketch uses excerpts from the trial. The suggestion/reason are
illustrative guesses, not operator-authored words or a new model response. The
goal deliberately retains “I find…”; layout must not silently fix goal language.
Scroll cues are chrome, not inserted ellipses or rewritten source/goal text.

```text
tinkery › think                         Real · GPT-6.1 Sol                 ?
Operator: agent — declared, not verified

YOUR WORDS                          NEXT QUESTION
I want to make a city healthcare     What would make this loop satisfying
system management game with the      to you: finding a workable balance for
approachable simulation and          different patient populations,
management feel of RollerCoaster     understanding consequences across the
Tycoon. Think of managing an         network, making difficult funding
organization like Central Health:    tradeoffs, or some combination?
↓ More original
                                    YOUR ANSWER · suggestion
                                    Understanding how funding decisions
                                    affect care across city populations.
                                    Tab to use · or type your own
                                    Why: links budgets to population care.
                                    DRAFT GOAL · Tinkery’s guess
                                    I find the smallest satisfying
                                    playable city healthcare system
                                    management game with the approachable
                                    ↓ More draft · PgUp/PgDn


F2 send answer   F8 skip   F3 review   F10 menu
```

Headings are bold ink with deliberate spacing, not new boxes. The question keeps
jade plus a non-colour focus cue; source words stay ink, interpretation remains
visibly provisional/italic, and existing changed-line emphasis is preserved.
Ghost text is muted **and labeled**, not distinguished by colour alone. The
attribution line remains truthful; its wording/placement is not authentication or
new authority. Do not grow this slice into a voice/setup or diagnostics redesign.

## State behavior

| State | Main region and next action |
| --- | --- |
| Before first send | Full writing area, **WHAT’S ON YOUR MIND?**, visible caret. Compact real/practice disclosure before effort is spent. F2 sends; Enter remains newline. Practice explicitly says simulated; live disclosure names the selected provider/model accurately. |
| Call in flight | **READING YOUR WORDS / ANSWER** at the interaction region, with the actual shaping/review phase. Submitted text and previous draft remain available. New unsent typing stays visibly separate and survives the result. Refusal/error recovery takes precedence. |
| Question active | Full accessible question, optional suggestion/reason, answer editor, then draft viewport as above. Keep question and answer adjacent; never require F3 to discover hidden question text. |
| Ready or intent carried forward | Draft takes the main space. **F3 REVIEW THIS DRAFT** is primary; typing opens **YOUR REVISION**, retaining the first character. Existing unsent text never disappears. Missing/open intent stays honest; no “all decisions settled” claim. |
| Review | **REVIEW GOAL**, not “You are confirming”. Show the same complete goal, relevant original/answer access, and carried-forward questions/choices. Explain saving consequences and any authorization blocker in plain language. Confirmation remains a separate existing action, not F3. |

### Draft sections

Proposed display headings: **DRAFT GOAL · Tinkery’s guess**, **WHY IT MATTERS**,
**SUCCESS CRITERIA**, **CONSTRAINTS**, **OUT OF SCOPE**. The last two describe
existing Must/Must not only; omit absent optional sections. No inferred exclusions,
new metrics, semantic consolidation or rewritten first-person outcome.

Heading approval does not approve changing the stored goal schema. Keep chrome
separate from canonical content and prove a lossless mapping through reading,
review, copying, saving and handoff. The frozen whole block must remain unchanged,
not reconstructed from a different interpretation. If friendly headings cannot
meet that contract, retain and strengthen the existing labels rather than weaken
the invariant. Correcting “I find…” or separating instructions from requirements
belongs to a later, independently evaluated semantic slice.

## Shipped suggestions

[#330](https://github.com/jon-devlapaz/tinkery/issues/330) shipped through
[#332](https://github.com/jon-devlapaz/tinkery/pull/332). The current contract is a
separate suggestion and reason, with **Ctrl-R** copying into an empty draft. F2
alone sends; existing drafts are preserved; unaccepted suggestions stay out of
subsequent request and handoff context. This proposal does not authorize a new
interaction or live evaluation; those need separate approval and an explicit budget.
It must not add a second generator, wire format, suggestion store or acceptance path.

The operator subsequently requested inline muted text accepted with Tab. That is
a requested presentation direction, **not a resolved keybinding/integration decision**.
Current shipped Tab/Shift-Tab toggle writing and originals; a change needs explicit
agreement before code. The operator identified **Codex chat Research drafted-goal
answer hints** as #330/#332's owner and **Codex chat Evaluate Think design and
quality** as master supervisor. Those ownership decisions were resolved; they do
not approve the remaining presentation proposal.

### Proposed acceptance behavior — unaccepted

| Situation | Proposed behavior |
| --- | --- |
| Empty answer, writing focused, matching suggestion visible | Show labeled ghost text and its short reason. **Tab** copies it into editable unsent text, keeps focus/caret in writing, and does nothing else. |
| Operator types instead | Dismiss ghost presentation; retain the first typed character and ordinary editing. Never mix the suggestion into existing text. |
| Existing text or selection | Do not overwrite or append a suggestion. No acceptance hint when acceptance is unavailable. |
| Original pane focused | Tab returns to writing; it does not accept a suggestion in the same keypress. |
| No acceptable visible ghost | Tab keeps the original/writing toggle. **Shift-Tab** retains original access in all suggestion states. |
| Accepted suggestion | Ordinary editable draft; F2 remains a separate send. #330 owns its provenance rules; unaccepted suggestions never become answers or leak into model/seed context. |
| Question changes, request starts, or suggestion is malformed/unavailable | Discard stale ghost and its acceptance hint. Keep answering/skipping usable; do not invent a preference. |

The shipped Ctrl-R slice is complete. Contextual Tab remains the operator-requested
**unaccepted proposal**. Whether Tab replaces, supplements or is rejected in favor
of Ctrl-R remains unresolved under D2. Do not implement ghost text or silently ship
both mappings.

### Overflow and focus — unvalidated proposal

Recommendation: retain one reading-side PgUp/PgDn route, with each overflowing
question, unaccepted ghost and draft viewport independently clamped; the wheel
over those regions follows the same route. Content that fits stays put. The answer
editor follows its caret separately; originals keep their existing access/scroll
behavior. This avoids adding a focus-key maze solely to read the question.

The compact sketch is not proof for arbitrary content. Question, reason, suggestion,
long answer and draft must each remain reachable at 80×24, 100×30 and the actual
pane size. Give question/answer priority and reduce draft preview first; never
silently clip or summarize either original or goal. Unaccepted ghost text needs
a discoverable full-text route before use, not an invisible tail. If shared scrolling
is confusing in a prototype, return to approval rather than invent extra bindings.

<a id="approval-decisions"></a>

## Decisions and implementation boundary

The recommendations above do not accept any proposed interaction. Only D7 records
an already-resolved ownership decision.

| ID | Operator decision | Recommendation / current state |
| --- | --- | --- |
| D1 | Question/answer first, draft below while answering? | **Unaccepted.** Audit recommendation: revise; defer a feature issue until a specific usability gap is observed. |
| D2 | Future inline ghost with contextual Tab, or keep the shipped Ctrl-R presentation? | **Unaccepted.** Audit recommendation: revise; keep Ctrl-R and obtain an explicit choice before changing key behavior. |
| D3 | Accept the compact preview/shared-scroll model for a bounded prototype? | **Unaccepted.** Audit recommendation: revise; retain current scrolling unless a concrete overflow or navigation failure is found. |
| D4 | Approve the proposed display headings? | **Unaccepted.** Audit recommendation: drop the heading change; keep the canonical goal-block labels. |
| D5 | Ready state uses review as primary; typing opens revision instead of a permanent empty form? | **Unaccepted.** Audit recommendation: revise; keep the existing ready/F3 behavior unless a distinct revision-mode problem is demonstrated. |
| D6 | Include review wording/source access in this presentation slice or defer? | **Unaccepted.** Audit recommendation: revise; retain current original and unresolved-intent access unless a specific gap is identified. |
| D7 | Current ownership and integration | **Resolved by operator:** Research drafted-goal answer hints owns #330/#332; Evaluate Think design and quality is master supervisor. This proposal neither blocks the merged slice nor approves a new interaction. |

## Validation before an implementation claim

After design approval, publish one bounded implementation scope with a named owner
and coordinate #330 first. Name architecture diagnostics 3/6 for screen-owned layout
and 5/7 for explicit decisions/discoverability; do not invent a higher design score.

Required evidence for that later scope:

- Empty/typing/thinking/refining/question/ready/review states, including cancellation,
  failure, unsent edits, resize and long question/suggestion/reason/goal combinations.
- Correct Tab/Shift-Tab/Ctrl-R behavior for both focuses, text/selection, stale or
  unavailable suggestions; acceptance/display/navigation make no call or send.
- Exact original/answer and frozen goal equality through review/copy/save/handoff;
  provenance stays at #330's existing boundary, not inferred from appearance.
- 80×24, 100×30 and actual-pane terminal journeys with colour/no-colour, full
  applicable verification and deliberate reversion probes. No real audio.
- Fresh operator dogfood after mechanics pass. Agree a separate live budget and
  success threshold before #330/model-quality evaluation; previous trial accounting
  does not authorize it. Fakes do not prove effort reduction or intent fidelity.

The #427 audit is complete. D1–D6 remain unaccepted; this record does not authorize
an interaction feature or new provider calls.
