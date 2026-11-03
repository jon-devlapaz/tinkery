# #657 live eval (temporary; removed before merge)

Does Shape propose check commands that run in the checkout as cloned?

- `make_repos.py OUT` builds five TEST repos. In four, the bare test command fails as cloned
  (tox-only src layout, generated module via Makefile, npm workspace, Makefile-only PYTHONPATH);
  `cargo-workspace` is a control where the bare command works.
- `drive.py` runs one live Shape draft per repo through the real binary in a PTY, with a
  confirmed goal and an answered pre-mortem. A `pi` wrapper counts calls and keeps prompts and replies.
- `score.py` runs each proposed `cmd` in a fresh clone with its own venv and HOME. Pass = past
  setup (no import error, missing module or missing command); a test that runs and fails still passes.
- `variants/v1.txt` is the candidate `src/shape/prompt.txt`. Baseline is the prompt on `main`.

Only `tox-src` matches the rule's one example (`PYTHONPATH=src`); the other three are out of sample.

## On the Mac (operator's Pi auth, live calls)

Every call is counted from the `calls/` folders; stop at the budget the thread gives.

```bash
W=~/dev/worktrees/tinkery/claude-project-thread-syoh26
E=/tmp/tinkery-TEST-657-eval
git -C ~/dev/active/tinkery fetch origin claude/project-thread-syoh26
git -C ~/dev/active/tinkery worktree add "$W" origin/claude/project-thread-syoh26
cd "$W"
mkdir -p "$E/bin"
python3 eval-657/make_repos.py "$E/repos"
cargo +stable build --locked --example seed_fixture
# Binaries: baseline = main's prompt, v1 = candidate prompt.
git show origin/main:src/shape/prompt.txt > src/shape/prompt.txt
cargo +stable build --locked && cp target/debug/tinkery "$E/bin/base"
cp eval-657/variants/v1.txt src/shape/prompt.txt
cargo +stable build --locked && cp target/debug/tinkery "$E/bin/v1"
git checkout src/shape/prompt.txt
export PI_CODING_AGENT_DIR=~/.pi/agent
# One run (repeat per repo and binary):
python3 eval-657/drive.py --binary "$E/bin/base" --repos "$E/repos" --repo tox-src \
  --label base-tox-src-1 --out "$E/runs" --model openai-codex/gpt-6.1-sol --thinking medium --budget 2
# Calls so far:
ls "$E"/runs/*/calls/*.start | wc -l
```

Report each `runs/<label>/result.json` (its `checks`, `endings`, `calls`). Scoring runs in the cloud.
