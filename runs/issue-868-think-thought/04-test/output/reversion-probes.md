# Required F3 protection probes

Checked commit: cb44a4a. Disposable checkout:
`/Users/jondev/dev/worktrees/tinkery/codex/probes-868`.
The build/review checkout and their target directories were not modified.

Command:
`python3 /Users/jondev/scratch/tinkery-868-evidence/run-probes.py`

The script builds each production mutation with `cargo +stable build --locked`.
It runs the unchanged `tests/tend_thought_terminal.py::capture_guards((80,24))`.
That fixture uses TEST roots, `--operator agent`, local fake providers, and allowances 0 and 8.
No external model calls occur.

- Baseline: build and capture guards passed.
- Provider mutation: after a successful store save, call `self.host.reshape` synchronously
  using the current input's request. The real configured provider boundary runs.
  Test failed at line 285: `assert not flow.requests(root)`.
  This is the explicit zero-call assertion, not a compile failure or screen timeout.
- Goal mutation: after a successful store save, call the real `Config::confirm`
  with the input words and a TEST affirmation. This deliberately bypasses the normal
  separate goal-review authority. Test failed at line 44:
  `AssertionError: A thought changed goal sessions`.
- Restoration: restored the original source bytes, rebuilt, and reran capture guards successfully.
  `git diff --exit-code` passed and `git status --short` was empty.

Exact mutation blocks are in `provider-mutation.txt` and `goal-mutation.txt`.
Build and test outputs have corresponding `*-build.log` and `*-test.log` filenames.
The original changed-word probe lives in `/tmp/issue868-words-probe.log` and is separate.

No uncertain-persistence fault injection was attempted. That remains unverified at runtime.
