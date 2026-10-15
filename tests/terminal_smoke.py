"""Run after cargo build: python3 tests/terminal_smoke.py (POSIX)."""
from pathlib import Path
import os
import subprocess
import tempfile

BINARY = Path(__file__).resolve().parents[1] / "target/debug/tinkery"

for arguments in (["--help"], ["--snapshot"], ["--no-color", "--snapshot"]):
    result = subprocess.run([str(BINARY), *arguments], capture_output=True, check=True)
    assert b"\x1b" not in result.stdout, "Noninteractive mode emitted terminal escapes"

result = subprocess.run([str(BINARY), "--help"], capture_output=True, check=True)
assert b"prototype" not in result.stdout, "Help still calls Tinkery a prototype"
assert b"--workbench" not in result.stdout, "Help still lists the removed demo"
assert b"--no-color   Turn off colors" in result.stdout, "Help misdescribes --no-color"

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
# No HOME means no default record folder: refuse rather than write under the current folder.
pi = ["--shape-pi", "--model", "test/model", "--seed-me", "/nonexistent/TEST-SKILL.md"]
no_home = {k: v for k, v in os.environ.items() if k != "HOME"}
for env in (no_home, {**no_home, "HOME": ""}, {**no_home, "HOME": "TEST-relative"}):
    result = subprocess.run([str(BINARY), *pi], capture_output=True, env=env)
    assert result.returncode != 0 and b"pass --record-root PATH" in result.stderr, (env.get("HOME"), result.stderr)
result = subprocess.run([str(BINARY), *pi, "--record-root", "/tmp/tinkery-TEST-records"], capture_output=True, env=no_home)
assert result.returncode != 0 and b"pass --seed-session-root PATH" in result.stderr, result.stderr
result = subprocess.run([str(BINARY), *pi, "--record-root", "/tmp/tinkery-TEST-records", "--seed-session-root", "/tmp/tinkery-TEST-sessions"], capture_output=True, env=no_home)
assert result.returncode != 0 and b"HOME" not in result.stderr, result.stderr
for arguments in (["--snapshot"], ["--home", "--snapshot"]):
    subprocess.run([str(BINARY), *arguments], capture_output=True, check=True, env=no_home)
# Tend's destination is independent of live drafting, and never created by navigation/snapshots.
with tempfile.TemporaryDirectory(prefix="tinkery-TEST-link-cli-") as directory:
    root = Path(directory)
    for home in (no_home, {**no_home, "HOME": ""}, {**no_home, "HOME": "TEST-relative"}):
        for entry in ([], ["--home"]):
            result = subprocess.run([str(BINARY), *entry, "--snapshot",
                                     "--seed-session-root", str(root / "TEST-sessions"),
                                     "--link-root", str(root / "TEST-links")],
                                    capture_output=True, env=home)
            assert result.returncode == 0, result.stderr
    for arguments in (["--link-root"], ["--link-root", ""]):
        result = subprocess.run([str(BINARY), *arguments], capture_output=True, env=no_home)
        assert result.returncode != 0 and b"--link-root needs a folder" in result.stderr
    assert not (root / "TEST-links").exists()
assert b"--link-root PATH" in subprocess.check_output([str(BINARY), "--help"])
print("PASS: noninteractive modes, removed entry points, explicit CLI errors, HOME refusal and practice link roots")
