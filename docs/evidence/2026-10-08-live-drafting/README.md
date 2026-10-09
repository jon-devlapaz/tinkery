# Live working-draft evidence — 2026-10-08

Synthetic-safe inputs and actual model-generated working papers. These are test evidence, not confirmed seeds, operator answers, or approved snapshots.

## Fixed expectations

- The investigation goal and proposed outcome describe what becomes better for the person, not a solution to build.
- An existing plan remains identifiable under **Possible approaches / not accepted**. It is not an expected deliverable or accepted decision.
- Stated intentions and supplied context are not assumptions. Only meaningful missing information belongs there.
- The interface labels assumptions once. Original sticky text remains unchanged.

These expectations were set before the reruns. They were not changed to match the responses.

## Runs and provenance

- Host: actual Pi CLI 1.1.0, selected model `openai-codex/gpt-5.6-luna`, thinking `off` (subject to Pi's capability clamping).
- Instructions: Seed Me 2.2.0, `tink-skills` revision `86066b175b53e13b4180ce4cc9929a626b57a2f6`, `skills/seed-me/SKILL.md`.
- Skill SHA-256: `8e8f4f07143b0bfa08c69134d7ff9e9dd009eafe28b32cb0c25f6d67e4792102`.
- Only the actual skill's writing guidance and pre-confirmation working-draft step are supplied. Tinkery adds the bounded response contract and presentation rules; it does not host the complete interview.
- Requests use `--print --no-session --no-tools --no-extensions --no-mcp --no-skills --no-prompt-templates --no-themes --no-context-files --no-approve --offline --thinking off`. Model input goes through stdin.
- `before/`: problem-first, plan-first, and one fixture-feedback revision before tightening. The plan-first outcome incorrectly promoted a graph to the deliverable; assumptions had duplicated labels and included supplied intentions.
- `after-initial/`: first tightened rerun. Outcomes improved, but a mostly supplied context statement remained an assumption and the graph candidate was silently generalised. Retained rather than discarded.
- `after/`: final tightened rerun, one request per entrance. Both express the person's improvement, retain the graph as a named candidate in the plan case, and record no assumptions. That does not establish that no assumptions exist.

Markdown files contain complete working papers, including exact synthetic source thoughts. JSON views preserve the rendered cell strings, including padding, and their terminal dimensions: problem-first at 100×30, plan-first and feedback at 80×24. They are observations, not automated semantic test oracles.

The fixture feedback used in the earlier plan revision was:

> Keep finding reasons near the work. Cut the graph as the leading option. Reshape the goal around being able to revisit a choice without losing its context.

It was test input, not an actual operator confirmation.

## Before and after outcomes

| Entrance | Before | Final after |
| --- | --- | --- |
| Problem | A practical way to record and find each decision and its reason alongside the work. | You can quickly see why choices were made while working and when revisiting the work. |
| Plan | A proposed decision-and-work graph that helps people trace each work item to the decisions behind it. | You can quickly trace each piece of work back to the decisions and reasons behind it. |

**Assistant's intent-fidelity judgment:** the final outputs match the stated purpose of recovering decision reasons near the work, without prescribing a graph. The plan's graph survives as a candidate. “Quickly” is the model's qualitative interpretation, not a measured performance claim or an accepted acceptance criterion.

This is a small, nondeterministic sample. It does **not** establish that the operator recognises or confirms the drafts, that every future input follows the instructions, or that a seed is ready for intake.

## What the checks establish

- Real Pi-backed requests completed through the actual Rust scratchpad and adapter, at both supported dimensions. Original note text and submitted source text were checked unchanged.
- Application/component checks cover selected live text, exclusion of unselected notes, blank skipping/counts, all-blank preservation, feedback history, retained paper on failure, and focus/reading-position preservation.
- Running-binary PTY journeys cover shape/correct/revise with a **stub provider**, readable feedback at both sizes, exit while a request is pending, terminal cleanup, and controlled filesystem checks.
- Parser/process checks cover disabled resources, no saved Pi sessions, malformed responses, authority fields, terminal controls, input/output bounds, timeout, cancellation, and rate-limit reporting.
- Mutation spot-checks reject removed blank filtering, removed `--no-session`, accepted-approach labelling, repeated provisional labels, and removed outcome/assumption instructions.

Limits: the actual-model checks are real-component checks, not actual-model PTY or Herdr end-to-end tests. Herdr routing still needs a manual host check. Disabled tools/resources are not an OS sandbox; Pi's existing credential/configuration handling remains its responsibility. No real interview, goal/seed confirmation, canonical artifact handoff, or factory execution was added.

## Reproduce explicitly

From the repository, choose a fresh empty evidence directory and the actual installed Seed Me skill path:

```bash
TINKERY_LIVE_MODEL=openai-codex/gpt-5.6-luna \
TINKERY_SEED_ME="$HOME/dev/active/factory/seed-me/skills/seed-me/SKILL.md" \
TINKERY_LIVE_SKIP_FEEDBACK=1 \
cargo run --locked --example live_drafting_check -- /path/to/new-empty-evidence-directory
```

Omit `TINKERY_LIVE_SKIP_FEEDBACK` to include the fixture feedback revision. This command makes real provider requests and may incur charges; it is not run by the automated test suite. The model is required and selectable. The example refuses to overwrite a prior run.

Launch the interactive experiment:

```bash
cargo run --locked -- --shape-pi \
  --model openai-codex/gpt-5.6-luna \
  --seed-me "$HOME/dev/active/factory/seed-me/skills/seed-me/SKILL.md"
```

F2 shapes selected written stickies; `r` opens feedback; F2 revises; PgUp/PgDn reads the paper while writing feedback. Default launch remains simulated and unsaved.
