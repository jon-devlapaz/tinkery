# Change brief

Source: https://github.com/jon-devlapaz/tinkery/issues/874
Issue body sha256: a83c9ac156e37f9192d8e56396ffaff2267367d058bccf5da43b4bffba784882
Title: Tend: Confirm a thought save after F3 in Think

The approved issue below is this run's brief, unchanged. Each Done-when line is one item in `checklist.json`.

---

### Primary area

tend

### Kind

bug

### Goal

After you press F3 in Think, you must see in Tend that the thought was saved, and whether it was new or a duplicate. Without this, you cannot tell that the words are safe.

### Evidence

Observed on `main` at `c6783e7`.

- Expected: Tend shows the save result and the saved thought. The removed Tend editor showed "Thought saved. No goal confirmed." or "Thought already saved. One entry. No goal confirmed."
- Actual: F3 opens Tend with no message. A new save and a duplicate look the same.
- The garden summary shows the first thought in alphabetical order, not the saved thought. With two or more thoughts, the saved words are often not on screen.
- In #868, the scope says "Each screen supplies its own user message." The Think screen supplies messages for refused and uncertain saves only.
- The dogfood screen `runs/issue-868-think-thought/04-test/output/dogfood-869/100x30-normal-garden.txt` shows the saved thought only because the store holds one thought.

### Cause (bugs)

- `Exit::Garden` carries no save outcome (`src/think/brain_dump.rs`, `save_thought`). `Saved::New` and `Saved::Duplicate` both become the same exit.
- `Home` opens `Place::Tend` with a new `Garden`, so `selected` is 0 (`src/menu.rs`, `tick`).
- `Garden.notice` is now cleared but never set (`src/tend/thought.rs`).

### Scope and boundaries

- Changes: Think passes the save outcome and the saved words to Tend. Tend selects the saved thought and shows one message for a new save or a duplicate.
- Cleanup in the same file: remove the unnecessary block and the fixed-string `format!` arguments in `Garden::render`.
- Unchanged: storage format, file names, duplicate rule, refusal and uncertain handling, F2 Send, goal confirmation, and the `g` copy-back route.
- Excluded: #866. Thoughts do not go into the garden graph.

### Done when

- [ ] After F3 with new words, Tend shows a saved message and the saved thought, at 80×24 and 100×30 with colour off.
- [ ] After F3 with words that are already saved, Tend shows a duplicate message and the total thought-record count remains unchanged, including when other thoughts already exist.
- [ ] With two or more thoughts already saved, the thought shown is the one just saved, not the first in alphabetical order.
- [ ] The save message and saved-thought summary preserve existing unreadable-record warnings and fit at 80×24 and 100×30 with colour off. Check both new and duplicate saves with unreadable records present.
- [ ] The message clears when you leave Tend or open the reader. It never claims a goal was confirmed.
- [ ] No provider request, goal record or confirmation record is created. The existing zero-request assertions still pass.
- [ ] Reversion probe: with the outcome dropped, the new terminal assertion fails. Restore the code, then run `python3 scripts/full-check.py` with temporary TEST roots.

### Context and dependencies

- Follows #868 (closed), PRs #869 and #871.
- Tests to extend: `tests/tend_thought_terminal.py` and `src/think/brain_dump/thought_tests.rs`.
- Design: `docs/design/shape-and-handoff.md`.

### Open decisions and permissions

- Decision accepted: Keep the return to the garden after F3. Show the save result and saved thought in the garden summary. Do not open the reader automatically. #866 remains a separate display proposal.
- The operator authorized a supervised worker implementation, checks, independent review, and a PR on 2026-11-15. Do not merge. Keep #866 separate. Model and provider budget is zero. No operator data may be used.

