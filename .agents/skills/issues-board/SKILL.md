---
name: issues-board
description: Use when filing, picking, starting, finishing or reporting on Tinkery work, or asked what's going on: manage issues across Tinkery's repos and its GitHub Projects board.
---

# Tinkery issues board

Board: https://github.com/users/jon-devlapaz/projects/1 (`jon-devlapaz`, `1`), holding
open issues from tinkery, tink-sdlc, seed-me, tink-route, tink, tink-substrate and tink-skills.

Fields:
- **Lane:** Now, Needs your call, Next, Blocked, Later, Close?
- **Release:** M1–M6 match tinkery milestones; M7 Shape is R11 (#149, #150).
  *Supports the trial* means another repo's work used by #116's trial;
  *Off the release line* means everything else.
- **Status:** Todo, In Progress, Done.

## Access

Requires `gh` with `project` scope (`gh auth status`). Cloud sessions cannot reach
Projects (GraphQL/user REST blocked): use the operator's Mac, e.g. Remote Control,
never workarounds.

```bash
gh project item-list 1 --owner jon-devlapaz --format json --limit 200            # the default limit (30) misses items
gh project item-list 1 --owner jon-devlapaz --format json --limit 200 --query 'lane:Blocked'
gh project field-list 1 --owner jon-devlapaz --format json                        # field and option ids
gh project view 1 --owner jon-devlapaz --format json --jq .id                     # project id for item-edit
gh project item-edit --project-id P --id ITEM --field-id F --single-select-option-id O
gh project item-add 1 --owner jon-devlapaz --url https://github.com/jon-devlapaz/REPO/issues/N --format json --jq .id
gh api repos/jon-devlapaz/REPO/issues/N/dependencies/blocked_by --jq '.[] | "\(.number) \(.state)"'
```

JSON keys are `.lane`, `.release`, `.status`; use `(.labels // [])` for nullable labels.
Issue state is absent.

## Automations already on

Closing sets Done; never set Done yourself, especially on open issues: auto-close is on.
Auto-added sub-issues lack Lane/Release; fill both.

## Reading the board

Skip Done. Report in order: Needs your call and Close? (operator decisions), Now,
Next, Blocked with its blockers, then Later as a count. Check native blocked-by links
**and** body gates (Context and dependencies; Open decisions and permissions), including
unaccepted designs.

## Approval

Lanes never authorize building. In tinkery, follow [work intake](../../../AGENTS.md#work-intake);
elsewhere there is no `approved` label: require the operator's explicit issue/thread
permission. Quote the decision when starting.

## Changing the board

Change fields only; editing, labelling or closing issues requires operator permission.

- **Start approved work:** Now / In Progress, one issue per thread/branch. Skip existing
  In Progress cards: another agent owns them.
- **Finish:** merged PR closes the issue; automation sets Done, leaving Lane unchanged.
- **Unblock:** move Blocked → Next only when all native blockers close; report remaining body gates.
- **Propose:** only the operator moves unapproved cards to Now or acts on Close?.
- **New issue:** add to board; set Release from its tinkery milestone or definitions above.
  Lane is Blocked with an open native blocker, otherwise Now if approved/started,
  Needs your call if awaiting approval, Later if outside the release.

Within Next, prefer most downstream unblockings, then earlier milestone; ask on ties.

## Filing issues

1. Name the Tinkery place/release work served; otherwise don't file. Search all seven
   repos for open duplicates: `gh search issues --owner jon-devlapaz --state open "<key words>"`.
   Extend an existing issue rather than duplicate it.
2. File where code changes; keep unready ideas in the [parked backlog](../../../docs/research/shared/backlog.md).
   Review proposals before publication; replace published local specs with issue links,
   preserving design contracts and historical findings.
3. Use [Bounded task headings](../../../.github/ISSUE_TEMPLATE/task.yml) in every repo;
   outside tinkery start with `Serves Tinkery: <place or release work>`.
4. Show the draft; publish only on the operator's word. Publication is not approval.
5. Tinkery triage: `needs-approval`, exactly one `area:` and one kind (`bug`, `enhancement`,
   `documentation`, `research`), release milestone where relevant, native blocking links.
   Form dropdowns don't apply labels; priority is separate. Add board Lane/Release.

## Parallel work

Only approved, mutually non-blocking items with separate writer areas may run in
parallel. In tinkery, separate `area:` labels; one integrator owns shared menu, CLI
and architecture checks. Elsewhere, no shared file/module. Give each a thread,
branch and In Progress status so others skip it.
