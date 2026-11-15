# Isolated trial of PR 869

Checked product candidate: f0bf85d640e386720121529a32449570d986845a.
Executable SHA-256: 44b8e92ff99b8c23759fbbc9fa3495997913a3ef17e6db0be7266b5ed115a245.
The product source and executable were not modified for fault injection.

## Observed

Four terminal trials passed: normal capture and directory-sync failure, each at
80x24 and 100x30 with colour off. Each started from the home menu and fresh Think.
F3 saved the exact 101 UTF-8 bytes, including spaces, newlines and café.
Tend opened, v reopened the thought, and a new app process read the same original.
The reader's g action copied the thought into unsent Think. F3 saved that unchanged
copy without creating a second thought.

The injected directory-sync error ran twice per fault trial: once after new-file
publication, then again when saving the exact duplicate. Think displayed
"Thought persistence is uncertain. Keep these words: Input/output error".
The input remained visible. The app did not navigate to Tend or report confirmed success.
The published file already existed. That is why the app reports uncertainty rather
than claiming nothing was saved. Removing the fault flag and pressing F3 recovered
in the same process. The app returned to Tend with one unchanged thought.

Every launch used a fresh temporary TEST HOME and explicit TEST session, record
and link roots. Two synthetic goals served as unchanged control records.
Their bytes did not change. A local provider tripwire with allowance zero was never
launched. No real goals, operator storage, installed app or model service was used.

## Judgment and limits

Agent-observed capture and reuse work for this small slice. The save action says
what happens, the app returns to Tend, and the original remains available after restart.
Reading still requires v and the separate reader. This does not resolve the garden
display concern assigned to 866. No expansion of this PR is needed for that concern.

Done-7 now has runtime evidence for uncertain persistence, in addition to the
previous refusal and invalid-input checks. Done-11 remains pending because this was
an agent-run trial with synthetic words, not the operator's real-thought judgment.
The fault test proves the response to a failed sync system call. It does not simulate
a power loss or prove physical disk durability after one.

## Repeat

On macOS with the PR checkout built, compile the test-only library beside this script:

```sh
clang -dynamiclib -Wall -Wextra -Werror fail-directory-sync.c -o fail-directory-sync.dylib
python3 run.py /absolute/path/to/pr-checkout
```

The library intercepts only sync calls on the exact TEST link directory while its
fault flag exists. All other calls pass through. The first attempt intercepted only
fsync, but this binary imports fcntl for macOS sync. The next attempt exposed a missing
newline in the test log. Both test-setup faults were corrected before the passing run.
Neither attempt changed the product. The retained script, results and screens record
the completed run. No product fix or new full-suite run was needed.

The principle-prove-it-works skill led to testing the executable through terminal input.
The principle-build-the-lever skill led to retaining a repeatable script and fault library source.
