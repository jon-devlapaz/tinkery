# Think interaction proposal — awaiting operator approval

**Design only.** This is the single proposed interaction for the
[October 22 dogfood review](../../../findings/2026-10-22-think-dogfood.md).
[#339](https://github.com/jon-devlapaz/tinkery/issues/339) authorizes consolidation
and documentation publication, not application implementation. The later operator
instruction permits verified current WIP, including documentation, to merge under
the master supervisor; that does not accept this proposed interaction. The
[backlog entry](../shared/backlog.md#think-interaction-review) points here instead
of carrying another specification.

[Yohaku](../../yohaku.md) governs this proposal. The shipped
[Think interaction](../../design/think.md#interaction-contracts),
[goal-block contract](../../design/goal-block.md),
[confirmation attribution](../../design/think.md#confirmation-attribution-185)
and [visual principles](../../design/principles.md) remain authoritative until an
approved slice deliberately updates its affected decision. This proposal changes
attention order/presentation, not model intent or permission policy.

## Recommendation

**While answering: question → answer → draft.** Exact originals remain on the left;
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

## Integrating #330 — one recommendation system

[#330](https://github.com/jon-devlapaz/tinkery/issues/330) owns published suggestion
scope, provenance and call limits. At audit, its
[open PR #332](https://github.com/jon-devlapaz/tinkery/pull/332) uses a separate
suggestion/reason and **Ctrl-R** to copy into an empty draft. It is not shipped in
the audited main revision. Its live evaluation is deferred with zero approved
budget. This proposal must reuse that work, not add another generator, wire format,
suggestion store or acceptance path.

The operator subsequently requested inline muted text accepted with Tab. That is
a requested presentation direction, **not a resolved keybinding/integration decision**.
Current shipped Tab/Shift-Tab toggle writing and originals; a change needs explicit
agreement before code. The operator has identified **Codex chat Research drafted-goal
answer hints** as #330/#332's owner, finishing the already-approved Ctrl-R slice.
**Codex chat Evaluate Think design and quality** coordinates integration as master
supervisor. This document owns no #330 code and does not hold its current merge.

### Proposed acceptance behavior

| Situation | Proposed behavior |
| --- | --- |
| Empty answer, writing focused, matching suggestion visible | Show labeled ghost text and its short reason. **Tab** copies it into editable unsent text, keeps focus/caret in writing, and does nothing else. |
| Operator types instead | Dismiss ghost presentation; retain the first typed character and ordinary editing. Never mix the suggestion into existing text. |
| Existing text or selection | Do not overwrite or append a suggestion. No acceptance hint when acceptance is unavailable. |
| Original pane focused | Tab returns to writing; it does not accept a suggestion in the same keypress. |
| No acceptable visible ghost | Tab keeps the original/writing toggle. **Shift-Tab** retains original access in all suggestion states. |
| Accepted suggestion | Ordinary editable draft; F2 remains a separate send. #330 owns its provenance rules; unaccepted suggestions never become answers or leak into model/seed context. |
| Question changes, request starts, or suggestion is malformed/unavailable | Discard stale ghost and its acceptance hint. Keep answering/skipping usable; do not invent a preference. |

For current WIP, keep the approved Ctrl-R slice unchanged. Contextual Tab is the
operator-requested **future proposal**, not a change to make in #330/#332. Whether
Tab replaces, supplements or is rejected in favor of Ctrl-R remains D2. Do not
implement ghost text, silently ship both mappings, or take over that worktree.

### Overflow and focus

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

## Approval decisions

None of these decisions is implemented or accepted by this document.

| ID | Operator decision | Recommendation / current state |
| --- | --- | --- |
| D1 | Question/answer first, draft below while answering? | Recommend yes; assistant proposal awaiting approval. |
| D2 | Future inline ghost with contextual Tab, or keep the separately approved Ctrl-R presentation? | Operator requested ghost/Tab as a mockup refinement, not implementation. Preserve the current Ctrl-R slice; any later replacement/alias needs a new scoped decision. |
| D3 | Accept the compact preview/shared-scroll model for a bounded prototype? | Recommend yes, subject to long-content/no-colour tests and operator inspection; no new focus keys by default. |
| D4 | Approve the proposed display headings? | Recommend stronger hierarchy with these names, conditional on exact goal preservation. Semantic rewriting is excluded. |
| D5 | Ready state uses review as primary; typing opens revision instead of a permanent empty form? | Recommend yes; unsent input must stay visible in every state. |
| D6 | Include review wording/source access in this presentation slice or defer? | Recommend include framing/access only; unresolved-intent/model policy and authorization stay unchanged. |
| D7 | Current ownership and integration | **Resolved by operator:** Research drafted-goal answer hints owns #330/#332; Evaluate Think design and quality is master supervisor. Finish approved Ctrl-R work; this proposal neither blocks its merge nor approves a new interaction. |

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

For #339: **review D1–D6; no application changes or new provider calls.** D7 records
resolved ownership. Verified current WIP may merge under the operator's latest
instruction; no new interaction feature follows from this document.
