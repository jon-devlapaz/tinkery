# Findings

## Think dogfood, October 12, 2026

[Read the report](2026-10-12-think-dogfood-report.pdf).

The report covers commit `4f520b22c28cd109de3c7d3fb7263cfad8bc3ccb`:
10 synthetic scenarios and four repeat runs, operated by agents with live model
replies. It does not establish whether later revisions still have these defects.
The PDF is preserved unchanged; the larger local evidence bundle it references
is not included here.

| Finding | Priority | PDF page | GitHub issue |
| --- | --- | --- | --- |
| F-01: Long reviews silently block confirmation | P2 | 4 | [#70](https://github.com/jon-devlapaz/tinkery/issues/70) |
| F-02: Unsent words silently block review | P2 | 5 | [#71](https://github.com/jon-devlapaz/tinkery/issues/71) |
| F-03: Back overlaps the save-error message | P2 | 6 | [#72](https://github.com/jon-devlapaz/tinkery/issues/72) |
| F-04: The record-save warning is clipped | P3 | 7 | [#73](https://github.com/jon-devlapaz/tinkery/issues/73) |
| F-05: Settled separate-goal context appears unfinished | P3 | 8 | [#74](https://github.com/jon-devlapaz/tinkery/issues/74) |
| F-06: One decision appears three times | P3 | 9 | [#64](https://github.com/jon-devlapaz/tinkery/issues/64#issuecomment-6138386385) |
| F-07: Exclusions use double-negative wording | P3 | 10 | [#75](https://github.com/jon-devlapaz/tinkery/issues/75) |

P2 means a reproduced usability failure obstructs an action or recovery. P3
means a lower-impact clarity defect; the intended goal and controls held in
these trials. F-06's priority describes that finding, not all of issue #64.

The six new issues were filed with `bug`, `priority: P2` or `priority: P3`, and
`needs-approval`. Filing them does not approve implementation. F-06 is linked to
the already-approved work in #64 instead of creating a duplicate. Follow the
issues for current status, and recheck findings after overlapping fixes settle.
