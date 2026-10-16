---
name: issues-board
description: Use when picking, starting, finishing or reporting on Tinkery work, or asked what's going on: read and update the cross-repo GitHub Projects board.
---

# Tinkery issues board

Board: https://github.com/users/jon-devlapaz/projects/1 (owner `jon-devlapaz`, number `1`).
It holds the open issues of tinkery, tink-sdlc, seed-me, tink-route, tink, tink-substrate and tink-skills.

Fields:
- **Lane**: Now, Needs your call, Next, Blocked, Later, Close?
- **Release**: M1–M6 match tinkery's milestones of the same name. M7 Shape is R11 (#149, #150).
  *Supports the trial* is work in another repo that #116's real-work trial runs through.
  *Off the release line* is everything else.
- **Status** (built in): Todo, In Progress, Done

## Access

Needs `gh` logged in with the `project` scope (`gh auth status` lists it). Cloud sessions can't reach
Projects (GraphQL and user-level REST are blocked there): run these commands on the operator's Mac,
for example through a Remote Control session, and never try workarounds.

```bash
gh project item-list 1 --owner jon-devlapaz --format json --limit 200            # the default limit (30) misses items
gh project item-list 1 --owner jon-devlapaz --format json --limit 200 --query 'lane:Blocked'
gh project field-list 1 --owner jon-devlapaz --format json                        # field and option ids
gh project view 1 --owner jon-devlapaz --format json --jq .id                     # project id for item-edit
gh project item-edit --project-id P --id ITEM --field-id F --single-select-option-id O
gh project item-add 1 --owner jon-devlapaz --url https://github.com/jon-devlapaz/REPO/issues/N --format json --jq .id
gh api repos/jon-devlapaz/REPO/issues/N/dependencies/blocked_by --jq '.[] | "\(.number) \(.state)"'
```

In item-list JSON, fields are lowercase keys (`.lane`, `.release`, `.status`), `.labels` can be null
(use `(.labels // [])`), and issue state isn't included.

## Automations already on

- Closing an issue sets its Status to Done. Don't set Done yourself.
- **Auto-close issue** is on: setting Done on an open issue may close it. Never set Done on an open issue.
- Auto-added sub-issues arrive with no Lane or Release; fill both in.

## Reading the board

When asked what's going on, answer in this order and skip Done items:
1. Needs your call, and Close? (the operator acts on both)
2. Now
3. Next
4. Blocked, each with what it waits on
5. Later, as a count

What an item waits on comes from its native blocked-by links *and* the gates named in its body
("Context and dependencies", "Open decisions and permissions"). Name any gate still open, such as an
unaccepted design.

## Approval

The board shows decisions; it doesn't make them. A card's lane is never approval to build.
- In tinkery, approval is the `approved` label ([AGENTS.md](../../../AGENTS.md#work-intake)).
- The other repos have no such label. There, approval is the operator's explicit word for that issue,
  in the issue or the thread. Quote it when you start.

## Changing the board

Change board fields only. Editing, labelling or closing an issue needs the operator's word.
- **Start** approved work: Lane Now, Status In Progress. One issue per thread or branch. Skip a card
  that is already In Progress: another agent owns it.
- **Finish**: the merged PR closes the issue, and the automation sets Done. The lane stays as it was.
- **Unblock**: when every native blocker is closed, move the card from Blocked to Next. If gates in its
  body are still open, say so when you report it.
- **Propose, don't act**: only the operator moves an unapproved card into Now or acts on Close?.
- **New issue**: add it, then set Lane and Release.
  - Lane: Blocked if it has an open native blocker. Otherwise Now if it is approved and started, Needs
    your call if it waits on approval, and Later if it doesn't belong to the release.
  - Release: the tinkery milestone if it has one, else the definitions above.

**Picking within Next**: prefer the item that unblocks the most others, then the earlier milestone. If
it's still a tie, ask.

## Parallel work

Run items in parallel only when all of these hold:
- each is approved
- none blocks another
- they don't share a writer area. In tinkery, that means the same `area:` label. One integrator owns
  the shared menu, CLI and architecture checks. In other repos, it means the same file or module.

Give each its own thread and branch, and mark each In Progress so other agents skip it.
