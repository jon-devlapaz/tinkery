"""Run after cargo build: python3 tests/terminal_smoke.py (POSIX)."""
from pathlib import Path
import subprocess

BINARY = Path(__file__).resolve().parents[1] / "target/debug/tinkery"

for arguments in (["--help"], ["--snapshot"], ["--no-color", "--snapshot"]):
    result = subprocess.run([str(BINARY), *arguments], capture_output=True, check=True)
    assert b"\x1b" not in result.stdout, "Noninteractive mode emitted terminal escapes"

result = subprocess.run([str(BINARY), "--help"], capture_output=True, check=True)
assert b"prototype" not in result.stdout, "Help still calls Tinkery a prototype"
assert b"--workbench" not in result.stdout, "Help still lists the removed demo"

result = subprocess.run([str(BINARY)], capture_output=True)
assert result.returncode != 0 and b"needs a terminal" in result.stderr
result = subprocess.run([str(BINARY), "--unknown"], capture_output=True)
assert result.returncode != 0 and b"unknown argument" in result.stderr
for removed in ("--scratchpad", "--workbench"):
    result = subprocess.run([str(BINARY), removed], capture_output=True)
    assert result.returncode != 0 and f"unknown argument: {removed}".encode() in result.stderr, f"The removed {removed} is still reachable"
for arguments, error in [
    (["--shape-pi"], b"requires an explicit --model"),
    (["--model", "test/model"], b"require --shape-pi"),
    (["--thinking", "low"], b"require --shape-pi"),
    (["--record-root", "/tmp/tinkery-TEST-records"], b"require --shape-pi"),
    (["--shape-pi", "--model", "test/model"], b"requires --seed-me"),
    (["--shape-pi", "--model", "bare", "--seed-me", "/nonexistent/SKILL.md"], b"explicit --model provider/model-id"),
]:
    result = subprocess.run([str(BINARY), *arguments], capture_output=True)
    assert result.returncode != 0 and error in result.stderr, (arguments, result.stderr)
    assert b"\x1b" not in result.stdout, "Invalid configuration entered terminal mode"
print("PASS: noninteractive modes, removed entry points and explicit CLI errors")
