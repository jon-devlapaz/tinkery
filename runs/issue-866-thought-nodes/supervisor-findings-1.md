# Supervisor review of b57b40e: two findings to fix

Screens (100x30, real PTY, colour): /private/tmp/claude-501/-Users-jondev-dev-active-tinkery/99e25e1a-d809-4263-8511-030195467d58/scratchpad/shots866/after-3-two-thoughts.png, after-4-arrow-to-a-goal.png, after-5-arrow-to-a-thought.png
(text versions: same names with .txt). Steps: save "TEST colour weather..." then "TEST museum heist..." via Think choice 2, then Left.

1. Save feedback follows the selection. After saving "museum heist", pressing Left selects "colour weather", and the footer
   reads "Thought: TEST colour weather: forecast the colours I keep saving · Thought saved." That claims the wrong thought
   was just saved. Expected: the save result shows only while the saved thought is selected (or clears on the first
   selection change). Add a PTY assertion for this.
2. Adjacent labels touch. With two goals and two thoughts, the goal label row reads
   "TEST find repeated supp…TEST make documentation…" with no gap, so it reads as one string. Expected: at least one blank
   column between labels on the same row (clip earlier or move one label to its other row). Check goals and thoughts at
   80x24 and 100x30; add an assertion.

Keep everything else. Fix, rerun the affected journeys and the full check (new process session as before), amend nothing:
add a new commit. Update the report file with the new SHA and evidence, then stop.
