"""Follow Shape's setup help back to the same TEST goal; navigation makes no calls or writes."""
from pathlib import Path
import os
import tempfile

from shape_answer_terminal import F1, Run, files
from shape_terminal import make, seed


def journey(width, height, drafted):
    with tempfile.TemporaryDirectory(prefix='tinkery-TEST-shape-recovery-') as directory:
        root = Path(directory).resolve()
        session = make(root / 'TEST-sessions', 'draft' if drafted else 'question',
                       goal='TEST reopen this exact saved goal', question=not drafted)
        if drafted:
            seed(session)
        # This synthetic human launch exercises the no-model reading hint. Agent-only
        # launches without --shape-pi have their own held-action message and tests.
        run = Run(root, width, height, budget=None, shape_setup=False)
        before = files(root)
        try:
            run.open_shape()
            run.read_until('Next: reopen this goal with Shape set up; F1 shows how.')
            run.send(F1, 'Right now')
            run.read_until('press Esc until the menu, then m: choose a model and calls, and s saves them.')
            run.read_until('Then q quits; start Tinkery again, keeping the same --seed-session-root if you supplied one.')
            run.read_until('Open Tend (2), select this saved goal, press Enter, then s to reopen Shape.')
            assert "tend's c" not in run.flat(), run.screen.text()
            run.menu()
            os.write(run.master, b'q')
            assert run.process.wait(timeout=5) == 0
            assert files(root) == before, 'Reading or setup help wrote TEST files'
        finally:
            run.close()

        # Follow the displayed setup options and Tend keys, with a zero-call limit.
        run = Run(root, width, height, budget=0, agent=True)
        before = files(root)
        try:
            run.open_shape()
            run.wait('Seed draft r1' if drafted else 'your answer')
            run.send(F1, 'Right now')
            run.read_until('Model calls left this run: 0 of 0.')
            run.read_until(str(session), wrapped_path=True)
            run.menu()
            assert run.calls() == [] and not (root / 'think-calls.jsonl').exists()
            assert files(root) == before, 'Reopening the configured goal wrote TEST files'
            print(f'Shape recovery passed at {width}x{height}: '
                  f'{"draft" if drafted else "question"}, same session, zero calls and writes')
        finally:
            run.close()


if __name__ == '__main__':
    for size in [(80, 24), (100, 30)]:
        for drafted in [False, True]:
            journey(*size, drafted)
