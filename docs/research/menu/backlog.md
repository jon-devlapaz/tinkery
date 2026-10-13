# Menu backlog

Drafts, not permission. [Work intake](../../../CLAUDE.md#work-intake) says how a draft becomes work.

- **An unset `HOME` puts Seed Me sessions under the current folder.** `Home::default_paths`
  (`src/menu.rs`) still falls back to a relative `.local/share/seed-me/sessions` when
  `--seed-session-root` is absent. Think records refuse this since #88; sessions could do the same.
