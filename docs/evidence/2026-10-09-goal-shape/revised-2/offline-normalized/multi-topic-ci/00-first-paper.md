# Tinkery / provisional board

Nothing confirmed. Unsaved.

## I think this is about… / guess

I get the same clippy results locally and on GitHub CI, during CI checks.
Why: The clippy version differs between CI and my laptop, so things pass locally and fail on GitHub; this happened twice this week.
Must: The clippy version on CI matches the clippy version on my laptop.

Supporting words:



also in your dump: rename the measure tab to something nicer, possibly ledger; automatically build a release when a PR merges

## Desired experience / proposed



## Doesn't fit yet

Pinning or otherwise synchronizing the clippy version is a possible approach.

The measure-tab rename and automatic release build appear to be separate goals from the clippy-version problem.

The desired release-build trigger and artifact handling are not defined.

## One question in focus

What observable check should show that the clippy mismatch problem is resolved?

## Possible approaches / not accepted

## Your deliberate extractions / exact excerpts

## Original 1 / dump

ok so the ci on tinkery takes like 2 minutes on mac and i dont really care about that but what bugs me is that the clippy version on ci is different from my laptop so things pass locally and fail on github. happened twice this week. also unrelated but i want to rename the measure tab to something nicer, ledger maybe? and honestly i think the release build should happen automatically when a pr merges so i stop copying binaries around by hand. rm then cp every time is annoying and i forgot once and ran an old build for an hour