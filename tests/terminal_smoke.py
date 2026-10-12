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


def run(quit_key, width=100, height=30):
    master, slave = os.openpty()
    resize(slave, width, height)
    original = termios.tcgetattr(slave)
    output = bytearray()
    screen = Screen(width, height)
    with tempfile.TemporaryDirectory(prefix="tinkery-smoke-") as directory:
        arguments = ["--workbench"]
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

        try:
            label = "demo / read only"
            await_text(label)
            await_text("q quit")
            expected = subprocess.run(
                [str(BINARY), *arguments, "--snapshot"],
                capture_output=True, check=True,
            ).stdout.decode().rstrip("\n")
            if (width, height) == (100, 30):
                assert screen.text() == expected, "Real 100 x 30 screen differs from snapshot"
            send(b"j", "Make the handoff legible")
            send(b"\t", "Tab work")
            send(b"\x1b[C", "Example evidence")
            send(b"a", "Attention")
            send(b"a", "1 question waiting [a]")
            send(b"?", "Keys")
            send(b"\r", "Example evidence")
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
            restored = termios.tcgetattr(slave)
            # macOS can change the transient input-retype flag when flushing input.
            original[3] &= ~getattr(termios, "PENDIN", 0)
            restored[3] &= ~getattr(termios, "PENDIN", 0)
            assert restored == original, "Terminal attributes not restored"
            final_files = {path.name: (path.stat().st_size, path.stat().st_mtime_ns) for path in Path(directory).iterdir()}
            assert final_files == initial_files, "Application wrote files"
        finally:
            if process.poll() is None:
                process.kill()
                process.wait()
            os.close(master)
            os.close(slave)


for quit_key in (b"q", b"\x03"):
    run(quit_key)
    print(f"PASS: workbench, 100x30, input, help, resize, no writes, restore ({quit_key!r})")

for arguments in (["--help"], ["--snapshot"], ["--no-color", "--snapshot"]):
    result = subprocess.run([str(BINARY), *arguments], capture_output=True, check=True)
    assert b"\x1b" not in result.stdout, "Noninteractive mode emitted terminal escapes"

result = subprocess.run([str(BINARY)], capture_output=True)
assert result.returncode != 0 and b"needs a terminal" in result.stderr
result = subprocess.run([str(BINARY), "--unknown"], capture_output=True)
assert result.returncode != 0 and b"unknown argument" in result.stderr
result = subprocess.run([str(BINARY), "--scratchpad"], capture_output=True)
assert result.returncode != 0 and b"unknown argument: --scratchpad" in result.stderr, "The removed prototype is still reachable"
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
