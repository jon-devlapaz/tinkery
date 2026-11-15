# Voice terminal test correction

The failing CI run was 43746327511 on candidate 46f5776.
The test required a fake helper to write node-closed.txt from its EOF cleanup.
Production closes stdin, waits at most 200 milliseconds, then kills and reaps the
helper. A slow helper can be reaped without writing that file.

Delaying fake cleanup by 400 milliseconds reproduced the original assertion failure.
Observation before fixture cleanup confirmed the helper PID was already reaped.
The exact scheduling delay on the CI runner is unknown. The reproduced test assumption
is invalid regardless of the runner's cause.

Only tests/voice_terminal.py changes. It retains the existing process-death assertions,
request counts, draft behavior and terminal restoration checks. Both marker prerequisites
are removed, including the missing-Node retry. A stalled EOF cleanup case exercises
forced shutdown. Test launches now declare --operator agent.
The actual helper's EOF and recorder cleanup test in voice/helper.test.mjs is unchanged.
The PTY claim is shutdown/reap, not independent proof of stdin closure.

All ten focused journeys passed. The normal journey covers 80x24, 100x30, 160x40
and 320x40, each with and without colour. One stalled-helper case runs at 80x24
without colour, followed by the missing-Node retry.
A temporary mutation removed voice.close from request_leave. The revised public
journey failed because the helper PID was still alive. Restoring production source
and rebuilding made that same journey pass. No production change remains.

All checks use fake voice engines and shaping hosts in TEST roots. No microphone,
real model, operator goals or operator application storage was used.
Full-suite results and independent review follow separately. Do not merge.
