# Tinkery / provisional board

Nothing confirmed. Unsaved.

## I think this is about… / guess

I get consistent clippy results locally and on GitHub.
Why: The clippy version on CI differs from the laptop, so checks pass locally and fail on GitHub; this happened twice this week.

Supporting words:



also in your dump: Rename the measure tab; Automate release builds after a PR merges

## Desired experience / proposed



## Doesn't fit yet

The tab rename and automatic release build look like separate goals rather than parts of the clippy-consistency goal.

Clippy consistency is the main goal because it is the stated source of repeated failures.

## One question in focus

What observable check would show that local and GitHub clippy results are consistent?

## Possible approaches / not accepted

## Your deliberate extractions / exact excerpts

## Original 1 / dump

ok so the ci on tinkery takes like 2 minutes on mac and i dont really care about that but what bugs me is that the clippy version on ci is different from my laptop so things pass locally and fail on github. happened twice this week. also unrelated but i want to rename the measure tab to something nicer, ledger maybe? and honestly i think the release build should happen automatically when a pr merges so i stop copying binaries around by hand. rm then cp every time is annoying and i forgot once and ran an old build for an hour