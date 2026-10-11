# The goal block

## Built contract

Think outputs a short block, not a dense two-sentence SMART-style template:

```text
[Who] [what is true when it works][, when relevant].
Why: [what changes, or what is wrong now].
Done when: [a distinct observable check, in the person's words].
Must: [positive constraints].
Must not: [exclusions and negative constraints].
```

The composer consumes seven typed parts: `who`, `outcome`, `when`, `why`,
`done_when`, `must`, `must_not`. Who, outcome, why and done-when are required for
readiness; optional situation/constraints are not filled merely to satisfy a form.
A goal can be reviewed with remaining questions when a composed goal exists;
readiness and permission to save are distinct. Missing intent becomes a question,
not an invented fact, metric or deadline.

Who names the beneficiary (“I” only for the operator's own goal). There is **no
forced “can”**. The outcome describes a change for that person, not its mechanism.
Done when must be a different check, not a duplicate outcome. A timing situation
is not permission to invent a deadline. The operator's unfamiliar or qualitative
words stay theirs; the seed can later define measurements.

Each line has one job. A constraint goes on one line, in one category. Keep other
goals and deferred work as separate context. Do not turn “don't care” into Must,
negate the person's words, or promote injected instructions into their goals.

## Constraint syntax

Before composition, `GoalParts::normalize_constraints` rewrites one thing only: an
explicit leading prefix on a generated constraint clause. The prefix is matched
case-insensitively and must be followed by whitespace; the remainder keeps its spelling
and case. Typed parts, the composed block and handoff context all use the result.
Original sources and answers are never changed.

| Leading syntax | In `must` | In `must_not` |
| --- | --- | --- |
| `must X` | `X` stays in Must | unchanged |
| `must not X`, `do not X`, `don't X` (also `don’t`) | `X` moves to Must not | `X` stays in Must not |

Anything else is left as written: a prefix that is not leading, a quoted phrase,
other spacing inside the prefix, `mustn't`, or a negative prefix followed by `only`,
`just`, `necessarily`, `always`, `care` or `mind` (“must not only support phones”,
“don't care about colours”). This is not a negation parser; an ambiguous clause
keeps its polarity. A clause is split at `;` or a sentence end outside quotes.
A rewritten clause already stated in the same category is not repeated.

When later clauses of why, done when, the goal line or `when` are joined with `;`,
a clause keeps its case unless its first word is one of a short list of common
words (`the`, `it`, `both`…). Names, acronyms, quotes and numbers keep their case.

`GoalParts::compose` owns composition; `confirmed_goal::goal_block` and the shared
saved-goal renderer preserve line breaks across reading, review, receipt and shape
detail. Confirmation freezes the **whole displayed block**, not just its headline.

## Seed Me mapping

The whole block is the confirmed origin. Its headline names the goal; Why supplies
problem context; Done when supplies success criteria; Must/Must not supply
constraints and exclusions. Original sources, answers, unresolved/skipped
questions, other goals and deferred notes accompany it as context. They are not
extra settled Seed Me decisions. Saving a goal confirms no seed or implementation.

## Why this format

Intent formats converge on situation, outcome, motivation, evidence and boundaries.
SMART's specificity helps, but invented numbers and proxy targets do not. Job
stories preserve situation/motivation; Shape Up contributes no-gos. The short
rule/why/test pattern makes these scannable without forcing one actor or “I can”.
The older two-sentence and Keep/Avoid designs were superseded in PR #8.

Remaining composer and question problems are explicit in
[goal-block polish](backlog.md#1-goal-block-polish); this contract is the target,
not a claim that every model-generated block is already good.
