---
name: writing-for-agents
description: Writing documents for agents. Use when creating or editing skills, or modifying AGENTS.md or CLAUDE.md.
---

Write for a repeatable process, not identical output. Apply this to skills, steering
files and referenced docs. For skill frontmatter, invocation and routing, read
[SKILL-MECHANICS.md](SKILL-MECHANICS.md).

## Pruning

Subtract before rewriting or splitting:

- **One home per meaning.** Remove duplication; link the authoritative rule.
  Repeating a leading word is useful; repeating its definition is not.
- **Use the environment.** Scripts, config, layout and `--help` already own discoverable
  facts. Cache only expensive lookups, unwritten conventions, reasons and gotchas.
- **Relevance:** remove stale material, exposition and branches outside the task.
  Shorter docs resist sediment; keeping every line because it once mattered does not.
- **No-ops:** delete instructions that don't change model behavior versus its default.
  This is model-relative; resolve disagreement by running the document. Delete the
  sentence, not just words. A weak leading word needs a stronger hook, not repetition.

## Context pointers

A pointer names out-of-context material **and when to read it**: a skill description
or steering-file link. Its wording controls discovery. Sharpen a weak trigger before
inlining its must-have target.

Front-load the leading word; name each distinct trigger branch once; cut identity
already carried by the body. Synonyms aren't separate branches. Always-loaded pointers
pay context cost on every turn, so prune them harder than their targets.

## The two loads

- **Context load:** always-loaded words spend model attention whether invoked or not.
- **Cognitive load:** humans must remember what exists and when to reach it. Spend
  this where human judgment matters; remove unnecessary indexing work.

Disclosure exchanges body context for a pointer. Unlinked material depends entirely
on human discovery; hiding everything is not the goal.

## Information hierarchy

Separate **steps** (ordered actions) from **reference** (rules/facts consulted on demand).
A document may contain either or both. Place material at the needed level:

1. In-file steps: what to do, in order.
2. In-file reference: needed during this path; a flat peer-set of rules is fine.
3. Disclosed reference: separate material behind a conditional pointer.

**Progressive disclosure:** inline what every branch needs; disclose what only some
need. Buried steps invite inconsistent execution. **Co-location:** keep a concept's
definition, rules and caveats together; scattering fragments meaning, duplication
repeats it. **Sprawl:** even unique, live content can be too long; split by branch or
sequence so each path carries only what it needs.

## Steps and completion criteria

End each step with a checkable, exhaustive completion criterion:

- **Clarity** distinguishes done from not-done. Vague bounds invite premature
  completion, pulled by visible later steps. Sharpen the bound first. Only if it
  remains fuzzy **and rushing is observed**, hide later steps across a real context
  boundary (handoff/subagent dispatch); an inline call doesn't clear context.
- **Demand** drives legwork: “every modified model accounted for” requires more than
  “produce a change list.” Exhaustiveness also binds flat reference (“every rule applied”).

## When to split

Split only when the added context/human indexing load earns its cost:

- **Sequence:** separate later steps when they demonstrably rush the current one;
  merging sequences can reintroduce that pull.
- **Invocation:** follow [skill mechanics](SKILL-MECHANICS.md#splitting-by-invocation).

## Leading words

Use compact, familiar concepts as behavioral hooks: _fog of war_, _tracer bullets_,
_red_. Define coined terms clearly, but prefer pretrained concepts over paying for
new definitions. Repeat the token, not the explanation: it anchors execution in the
body and discovery in pointers/prompts.

Replace repeated phrases with sharper hooks: “fast, deterministic, low-overhead” →
_tight_; “a loop you believe in” → _red_, an observable gate. A hook must beat the
model's default behavior to earn its place.

Prefer positive targets (“write one-line comments”) to negations that activate the
forbidden behavior. Keep prohibitions for hard guardrails that cannot be phrased
positively, paired with what to do instead.
