# Change brief

## Problem and outcome

## Acceptance criteria

## Constraints and exclusions

- Must (non-negotiables):
- Must not (exclusions and rejected alternatives):

## Approach

The implementation checklist lives in `checklist.json` (definitions with id/description/verify) and is marked only with `sdlc.py mark`. Give an item a `check` (argv + timeout) whenever an automated proof exists; `verify` then runs it and no mark is needed.

## Risks and verification

## Supervisor note for the resumed turn (2026-11-11)

Relayed under the operator's authorization (branch, push, draft PR) and the seed's constraint
"The builder commits on the current branch and does not push":

1. The five Python failures are from the environment, not your change: this shell exports
   `PI_CODING_AGENT_DIR` (a real Pi login), which full-check's TEST HOME does not clear. The
   supervisor ran them on clean `main` and on your candidate without it; all pass. Do not run a
   baseline. Run the full check as
   `env -u PI_CODING_AGENT_DIR -u PI_CODING_AGENT_SESSION_DIR -u TINK_HOME python3 scripts/full-check.py`
   and report its result, including Node.
2. Commit your work on the current branch before ending the turn: the code, tests and docs, plus
   `runs/<slug>/` files. Leave `AGENTS.md` uncommitted and unchanged (an intake tool edited it).
   Do not push.
