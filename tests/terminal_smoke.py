"""Run after cargo build: python3 tests/terminal_smoke.py (POSIX)."""
import fcntl
import os
from pathlib import Path
import select
import signal
import struct
import subprocess
import tempfile
import termios
import time

from vt_screen import Screen

BINARY = Path(__file__).resolve().parents[1] / "target/debug/tinkery"


def resize(fd, width, height):
    fcntl.ioctl(fd, termios.TIOCSWINSZ, struct.pack("HHHH", height, width, 0, 0))


def run(quit_key, workbench=True, width=100, height=30, example=None, real_draft=False, full_redraw=False):
    master, slave = os.openpty()
    resize(slave, width, height)
    original = termios.tcgetattr(slave)
    output = bytearray()
    screen = Screen(width, height)
    with tempfile.TemporaryDirectory(prefix="tinkery-smoke-") as directory:
        arguments = ["--workbench"] if workbench else []
        if real_draft:
            skill = Path(directory) / "SKILL.md"
            skill.write_text("# Seed Me\n### Shape the working draft\nPreserve intention; proposals are not decisions.\n### Size gate\n")
            host = Path(directory) / "pi"
            host.write_text('''#!/usr/bin/env python3
import json, os, sys, time
assert all(flag in sys.argv for flag in ['--no-session', '--no-tools', '--no-extensions', '--no-context-files', '--no-mcp'])
assert 'PI_SESSION_FILE' not in os.environ
request = json.load(sys.stdin)
assert request['thoughts'][0]['text'] == 'I want a graph, so I can find decision reasons.'
feedback = request['feedback']
if feedback == 'wait':
    time.sleep(10)
if feedback:
    assert request['previous_draft'] is not None
    goal = 'Revised goal: find reasons, not a graph.'
else:
    goal = 'Initial goal: find decision reasons.'
print(json.dumps({'goal': goal, 'outcome': 'Find reasons beside the work.', 'context': ['You proposed a graph.'], 'questions': ['Which reasons matter first?'], 'assumptions': [], 'options': []}))
''')
            host.chmod(0o755)
            arguments = ["--shape-pi", "--model", "test/model", "--seed-me", str(skill), "--pi-command", str(host)]
        if full_redraw:
            arguments.append("--full-redraw")
        initial_files = {path.name: (path.stat().st_size, path.stat().st_mtime_ns) for path in Path(directory).iterdir()}
        process = subprocess.Popen(
            [str(BINARY), *arguments], stdin=slave, stdout=slave, stderr=slave,
            cwd=directory, env={**os.environ, "TERM": "xterm-256color"},
        )

        def await_text(text, row=None, column=None):
            deadline = time.monotonic() + 5
            while time.monotonic() < deadline:
                if select.select([master], [], [], 0.05)[0]:
                    data = os.read(master, 65536)
                    output.extend(data)
                    screen.feed(data)
                visible = screen.text() if row is None else screen.text().splitlines()[row]
                if column is not None:
                    visible = visible[column:column + len(text)]
                if text in visible:
                    return
                if process.poll() is not None:
                    break
            raise AssertionError(f"Missing {text!r}:\n{screen.text()}")

        def send(keys, expected, row=None, column=None):
            output.clear()
            os.write(master, keys)
            await_text(expected, row, column)

        def mouse(button, x, y, expected, row=None):
            send(f"\x1b[<{button};{x + 1};{y + 1}M".encode(), expected, row)
            if button < 3:
                os.write(master, f"\x1b[<{button};{x + 1};{y + 1}m".encode())

        def click_label(label, expected):
            await_text(label)
            for y, line in enumerate(screen.text().splitlines()):
                if label in line:
                    mouse(0, line.index(label), y, expected)
                    return
            raise AssertionError(f"No visible clickable control: {label}")

        try:
            label = "demo / read only" if workbench else ("real model / unsaved" if real_draft else "simulated / unsaved")
            await_text(label)
            await_text("q quit" if workbench else "? help")
            expected = subprocess.run(
                [str(BINARY), *arguments, "--snapshot"],
                capture_output=True, check=True,
            ).stdout.decode().rstrip("\n")
            if (width, height) == (100, 30):
                assert screen.text() == expected, "Real 100 x 30 screen differs from snapshot"
            if workbench:
                send(b"j", "Make the handoff legible")
                send(b"\t", "Tab work")
                send(b"\x1b[C", "Example evidence")
                send(b"a", "Attention")
                send(b"a", "1 question waiting [a]")
                send(b"?", "Keys")
                send(b"\r", "Example evidence")
            elif real_draft:
                send(b"\r", "Scratchpad / editing")
                send(b"\x1b[200~I want a graph, so I can find decision reasons.\x1b[201~", "I want a graph")
                send(b"\x1bOQ", "Working paper / model")
                await_text("Shaped 1 note; skipped 0 blanks.")
                await_text("Initial goal: find decision reasons.")
                await_text("1 selected")
                assert not screen.clipboards, "Clipboard written without a copy action"
                send(b"y", "Clipboard escape sent")
                copied = screen.clipboards[-1]
                assert copied.startswith("# Investigation draft")
                assert "## Open questions / not answered" in copied and "Which reasons matter first?" in copied
                assert "### Note 1\n\nI want a graph, so I can find decision reasons." in copied
                assert not any(glyph in copied for glyph in "│┄┆┌┐└┘"), "Clipboard contains terminal boxes"
                assert "Clipboard escape sent" not in copied
                screen.cells[1][2] = " "  # Simulate a host-lost header cell, not an app edit.
                assert "tinkery / scratchpad" not in screen.text()
                send(b"\x0c", "tinkery / scratchpad")
                await_text("Initial goal: find decision reasons.")
                await_text("1 selected")
                send(b"r", "Keep / cut / reshape")
                await_text("Initial goal: find decision reasons.")
                send(b"\x1b[200~Cut the graph; keep the intention.\x1b[201~", "Cut the graph")
                send(b"q", "intention.q")
                send(b"\x1bOQ", "Revised goal: find reasons, not a graph.")
                await_text("Working paper / model")
                assert "confirmed for intake" not in screen.text()
                send(b"p", "p paper")
                await_text("I want a graph")
                if quit_key == b"\x03":
                    send(b"r", "Keep / cut / reshape")
                    send(b"\x1b[200~wait\x1b[201~", "wait")
                    send(b"\x1bOQ", "Shaping draft...")
            elif example is not None:
                name, thoughts = example
                await_text("What brought you here? A problem, hunch, or plan.")
                for index, thought in enumerate(thoughts):
                    send(b"\r" if index == 0 else b"n", "Scratchpad / editing")
                    send(b"\x1b[200~" + thought.encode() + b"\x1b[201~", thought[:12])
                    send(b"\x1b", "Scratchpad / Pinstar")
                if len(thoughts) > 1:
                    for y, line in enumerate(screen.text().splitlines()):
                        if "Note 1" in line:
                            x = line.index("Note 1") + 1
                            send(f"\x1b[<4;{x + 1};{y + 1}M\x1b[<4;{x + 1};{y + 1}m".encode(), "2 selected")
                            break
                    else:
                        raise AssertionError("No Note 1 target for Shift-click")
                click_label("F2 shape", "Drafting...")
                await_text("Draft / unconfirmed", row=height - 3)
                await_text("Working paper / sample")
                await_text("What you want to change")
                await_text("From Note 1" if len(thoughts) == 1 else "From 2 notes")
                await_text(f"{len(thoughts)} selected")
                send(b"\t", "j/k scroll")
                send(b"\x1b[F", "What should we keep, cut, or reshape?")
                # Correct a source thought without overwriting the submitted paper.
                send(b"\x1b", "p paper")
                body_y = (height - 20) // 2 + 5
                mouse(0, 3, 5, "0 selected")
                mouse(0, 7, body_y, "1 selected")
                click_label("e edit", "Scratchpad / editing")
                send(b" A correction.", "Unsent changes", row=height - 3)
                send(b"\x1b", "e edit")
                click_label("p paper", "Working paper / sample")
                await_text("Unsent changes", row=height - 3)
                send(f"\x1b[<2;31;{body_y + 1}M\x1b[<2;31;{body_y + 1}m".encode(), "Delete note")
                for y, line in enumerate(screen.text().splitlines()):
                    if "Delete note" in line:
                        # Click the menu padding over the inspector, not the canvas beneath it.
                        mouse(0, line.index("Delete note") + len("Delete note") + 1, y,
                              "Source removed / draft retained", row=height - 3)
                        break
                else:
                    raise AssertionError("No visible Delete note menu item")
                await_text(f"{len(thoughts) - 1} note")
                await_text("Working paper / sample")
                send(b"\x1a", "Unsent changes", row=height - 3)
                send(b"\x19", "Source removed / draft retained", row=height - 3)
                assert "confirmed for intake" not in screen.text(), name
            else:
                assert b"\x1b[?1006h" in output, "Mouse capture was not enabled"
                assert "Working paper" not in screen.text(), "Inspector should start closed"
                await_text("What brought you here? A problem, hunch, or plan.")
                send(b"\r", "Scratchpad / editing")
                send(b"\x1b[200~I keep losing track of why we made certain choices.\n\nI want those reasons near the work.\x1b[201~", "I keep losing")
                send(b"\x1b", "Scratchpad / Pinstar")
                body_y = (height - 20) // 2 + 5
                # Exercise real SGR drag, undo, and wheel events before editing.
                drag_y = body_y + 2
                send(f"\x1b[<0;11;{drag_y + 1}M\x1b[<32;14;{drag_y + 3}M\x1b[<0;14;{drag_y + 3}m".encode(), "I keep losing", row=body_y + 2)
                send(b"\x1a", "I keep losing", row=body_y)
                mouse(64, 10, body_y, "I keep losing", row=body_y - 1)
                mouse(65, 10, body_y, "I keep losing", row=body_y)
                mouse(0, 7, body_y, "Scratchpad / Pinstar")
                mouse(0, 7, body_y, "Scratchpad / editing")
                os.write(master, b"\x15")
                send(b"\x1b[200~Keep reasons nearby.\x1b[201~", "Keep reasons nearby.")
                send(b"q", "nearby.q")
                click_label("F2 shape", "Drafting...")
                await_text("What you want to change")
                await_text("Draft / unconfirmed", row=height - 3)
                await_text("From Note 1")
                mouse(0, width - 15, 10, "Scratchpad / Pinstar")
                send(b"\x1b[F", "What should we keep, cut, or reshape?")
                click_label("? help", "Keys")
                click_label("? close help", "Working paper")
                click_label("e edit", "Scratchpad / editing")
                send(b" New thought.", "Unsent changes")
                send(b"\x1b", "e edit")
                click_label("n new", "Scratchpad / editing / 2 notes")
                send(b"\x1b[200~" + b"One more line.\n" * 30 + b"\x1b[201~", "One more line.")
                click_label("F2 shape", "Drafting...")
                await_text("From Note 2")
                await_text("Draft / unconfirmed", row=height - 3)
                # Hide the inspector, drag the second note across the full workspace,
                # and verify real undo/redo restore its screen position.
                click_label("p close paper", "p paper")
                second_y = min((height - 20) // 2 + 3, height - 20) + 5
                end_x = width - 30
                y = second_y + 2
                send(f"\x1b[<0;9;{y + 1}M\x1b[<32;{end_x + 1};{y + 1}M\x1b[<0;{end_x + 1};{y + 1}m".encode(), "One more line.", row=second_y, column=end_x - 2)
                send(b"\x1a", "One more line.", row=second_y, column=6)
                send(b"\x19", "One more line.", row=second_y, column=end_x - 2)
                click_label("p paper", "From Note 2")
                await_text("more below / Tab to read")
                mouse(65, width - 15, 10, "more above / below / Tab to read")
                mouse(0, width - 15, 10, "more above / below / j/k scroll")
                send(b"\x1b[6~", "more above / below / j/k scroll")
                send(b"\x1b[F", "more above / j/k scroll")
                await_text("What should we keep, cut, or reshape?")
            output.clear()
            resize(slave, 60, 20)
            screen = Screen(60, 20)
            process.send_signal(signal.SIGWINCH)
            await_text("Resize to at least 80 x 24.")
            output.clear()
            resize(slave, width, height)
            screen = Screen(width, height)
            process.send_signal(signal.SIGWINCH)
            await_text(label)
            os.write(master, quit_key)
            deadline = time.monotonic() + 5
            while process.poll() is None and time.monotonic() < deadline:
                if select.select([master], [], [], 0.05)[0]:
                    output.extend(os.read(master, 65536))
            process.wait(timeout=1)
            assert process.returncode == 0, process.returncode
            while select.select([master], [], [], 0.05)[0]:
                output.extend(os.read(master, 65536))
            assert b"\x1b[?1049l" in output, "Alternate screen not restored"
            if not workbench:
                assert b"\x1b[?2004l" in output, "Bracketed paste not restored"
                assert b"\x1b[?1006l" in output, "Mouse capture not restored"
                assert b"\x1b[>0s" in output, "Shift override not restored"
                assert b"\x1b[?2026l" in output, "Synchronized update not ended"
            restored = termios.tcgetattr(slave)
            # macOS can change the transient input-retype flag when flushing input.
            original[3] &= ~getattr(termios, "PENDIN", 0)
            restored[3] &= ~getattr(termios, "PENDIN", 0)
            assert restored == original, "Terminal attributes not restored"
            final_files = {path.name: (path.stat().st_size, path.stat().st_mtime_ns) for path in Path(directory).iterdir()}
            assert final_files == initial_files, "Application or draft host wrote files"
        finally:
            if process.poll() is None:
                process.kill()
                process.wait()
            os.close(master)
            os.close(slave)


for workbench in (True, False):
    for quit_key in (b"q", b"\x03"):
        run(quit_key, workbench)
        print(f"PASS: {'workbench' if workbench else 'scratchpad'}, 100x30, input, help, resize, no writes, restore ({quit_key!r})")

run(b"q", False, 80, 24)
print("PASS: scratchpad, 80x24, multiple notes, full-width drag, inspector, no writes, restore")

examples = [
    ("problem-first", ["I keep losing track of why we made certain choices.\n\nI want those reasons near the work."]),
    ("plan-first", ["I want a graph connecting decisions to work.", "So I can find why we made those choices."]),
]
for width, height in [(100, 30), (80, 24)]:
    for example in examples:
        run(b"q", False, width, height, example)
        print(f"PASS: {example[0]}, {width}x{height}, draft, source correction, right-click delete, undo/redo, no writes")

for width, height, quit_key in [(100, 30, b"q"), (80, 24, b"\x03")]:
    run(quit_key, False, width, height, real_draft=True)
    print(f"PASS: model adapter stub, {width}x{height}, shape/correct/revise, original retained, no writes, safe exit")

for width, height, full_redraw in [(160, 40, False), (240, 40, False), (320, 40, True)]:
    run(b"q", False, width, height, real_draft=True, full_redraw=full_redraw)
    print(f"PASS: wide stub journey, {width}x{height}, paper/revision/repaint/clean OSC52, full_redraw={full_redraw}")

for arguments in (["--help"], ["--snapshot"], ["--no-color", "--snapshot"]):
    result = subprocess.run([str(BINARY), *arguments], capture_output=True, check=True)
    assert b"\x1b" not in result.stdout, "Noninteractive mode emitted terminal escapes"

result = subprocess.run([str(BINARY)], capture_output=True)
assert result.returncode != 0 and b"needs a terminal" in result.stderr
result = subprocess.run([str(BINARY), "--unknown"], capture_output=True)
assert result.returncode != 0 and b"unknown argument" in result.stderr
for arguments, error in [
    (["--shape-pi"], b"requires an explicit --model"),
    (["--model", "test/model"], b"require --shape-pi"),
    (["--thinking", "low"], b"require --shape-pi"),
    (["--shape-pi", "--workbench"], b"not --workbench"),
    (["--shape-pi", "--model", "test/model"], b"requires --seed-me"),
    (["--shape-pi", "--model", "bare", "--seed-me", "/nonexistent/SKILL.md"], b"explicit --model provider/model-id"),
]:
    result = subprocess.run([str(BINARY), *arguments], capture_output=True)
    assert result.returncode != 0 and error in result.stderr, (arguments, result.stderr)
    assert b"\x1b" not in result.stdout, "Invalid configuration entered terminal mode"
print("PASS: noninteractive modes and explicit CLI errors")
