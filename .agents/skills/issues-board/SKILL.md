---
name: issues-board
description: Use when picking, starting, finishing or reporting on Tinkery work, or asked what's going on: read and update the cross-repo GitHub Projects board.
---

# Tinkery issues board

Board: https://github.com/users/jon-devlapaz/projects/1 (owner `jon-devlapaz`, number `1`).
It holds issues from tinkery, tink-sdlc, seed-me, tink-route, tink, tink-substrate and tink-skills.

Fields:
- **Lane**: Now, Needs your call, Next, Blocked, Later, Close?
- **Release**: M1 Trust the goal … M7 Shape, Supports the trial, Off the release line
- **Status** (built in): Todo, In Progress, Done

## Access

Needs `gh` logged in with the `project` scope (`gh auth status` lists it). Cloud sessions can't reach
Projects (GraphQL and user-level REST are blocked there): run these commands on the operator's Mac,
for example through a Remote Control session, and never try workarounds.

```bash
gh project item-list 1 --owner jon-devlapaz --format json --limit 200   # read the board
gh project field-list 1 --owner jon-devlapaz --format json             # field and option ids
gh project view 1 --owner jon-devlapaz --format json | jq -r .id       # project id for item-edit
gh project item-edit --project-id P --id ITEM --field-id F --single-select-option-id O
gh project item-add 1 --owner jon-devlapaz --url https://github.com/jon-devlapaz/REPO/issues/N
```

## Reading the board

When asked what's going on, answer by lane: what's Now, what needs the operator's call, what's next and
what each Blocked item waits on (the issue's native blocked-by links). Lead with Needs your call.

## Changing the board

- The board shows decisions; it doesn't make them. A card in Now or Next is not approval to build: the
  issue still needs `approved` and its own permissions ([AGENTS.md](../../../AGENTS.md#work-intake)).
- Starting approved work: set Status to In Progress. One issue per thread or branch.
- Finishing: Status Done once the PR is merged or the issue is closed with evidence.
- When blockers close, move the card from Blocked to Next.
- Only the operator moves cards into Now or acts on Close?. Agents may propose either.
- A new issue in scope gets added with a Lane and a Release.
- Change board fields only. Editing, labelling or closing an issue needs the operator's word.

## Parallel work

Run items in parallel only when all of these hold:
- each is approved
- none blocks another
- they don't share a writer area: one integrator owns the shared menu, CLI and architecture checks

Give each its own thread and branch, and mark each In Progress so other agents skip it.
