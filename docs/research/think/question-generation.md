# How think's questions compare with research

Checked 2026-10-13 against main f4dad76. Earlier notes (SAGE, CIGAsk, Ask or Assume, Ambig-SWE
and others) live outside the repo in the project's think-eval research; this page adds only what
they didn't cover.

## How think makes questions

- **No dedicated question prompt.** The shaping call (`prompts/reading.txt`) returns one question
  and a list of decisions, each with at least two answers that would change a goal line.
- **The decision review** (`prompts/decision_review.txt`) is a second call. It adds up to three
  choices the draft settled silently, each with its question.
- **Code fallback** (`src/think/brain_dump/board.rs`): if a required part is empty and the model
  asked nothing, think asks a fixed question: "Who is this for?", "What should change for {who}?",
  "What is going wrong for {who}?" or "What would show that {goal line}?".
- **Stop:** no decision left that would change a goal line, or a part asked twice.

## What the research says

| Finding | Source | For think |
| --- | --- | --- |
| Goal questions lose almost all value once work starts. Clarifying at 10% of an agent's run reached pass@3 0.78 (0.80 with the full spec, 0.40 with none); by 70% it matched never asking. Unprompted, Claude Sonnet 4.5 asked in 23% of runs, halfway through on average. | [Ask Early, Ask Late, Ask Right](https://arxiv.org/html/2605.07937v1) | Supports think's purpose: settle the goal before Seed Me and agents start. |
| Low-quality questions disturb people. 89 people, 897 searches: sessions with only low-quality questions rated 3.00/5 against 3.65 for high-quality ones; showing only good questions was about 63% more efficient. Specific questions beat generic ones. | [Zou et al. 2022](https://irlab.science.uva.nl/wp-content/papercite-data/pdf/zou2022asking.pdf), [Clarification usefulness](https://arxiv.org/abs/2402.01934) | No change proposed. |
| Choose a question by where its answers lead; ties favour not asking. About +5% answer F1. | [Modeling Future Conversation Turns (ICLR 2025)](https://arxiv.org/html/2410.13788v2) | Already think's rule. |
| Splitting "is the request unclear?" from "is the step hard?" raised clarification F1 by 36-73%. | [Uncertainty Decomposition](https://arxiv.org/html/2606.19559v1) | Already done: decisions are separate from notes. |
| Batched multiple-choice questions (about 36 per session) raised prompt completeness; only 4 participants. | [PAWNI](https://arxiv.org/html/2608.01366) | Too weak to change one-question-at-a-time. |
