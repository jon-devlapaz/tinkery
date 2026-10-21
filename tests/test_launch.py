"""The sole process boundary for Python tests: Tinkery never inherits operator HOME."""
import atexit
import os
from pathlib import Path
import pwd
import subprocess
import sys
import tempfile

# Keep subprocess result/error types and pipe constants available to test callers.
CalledProcessError = subprocess.CalledProcessError
TimeoutExpired = subprocess.TimeoutExpired
PIPE = subprocess.PIPE
DEVNULL = subprocess.DEVNULL
STDOUT = subprocess.STDOUT

_home = None


class UnsafeTestHome(ValueError):
    """The requested process environment is not an isolated TEST home."""


def environment(base=None, *, home=None):
    """Explicitly select a TEST home; by default allocate one for this test process.

    Caller-selected homes must already exist. Preserve tool caches, not application
    data, when changing HOME so fixture builds need no toolchain installation.
    """
    global _home
    env = dict(os.environ if base is None else base)
    real_home = pwd.getpwuid(os.getuid()).pw_dir
    env.setdefault('CARGO_HOME', str(Path(real_home) / '.cargo'))
    env.setdefault('RUSTUP_HOME', str(Path(real_home) / '.rustup'))
    if home is None:
        if _home is None:
            _home = tempfile.TemporaryDirectory(prefix='tinkery-TEST-launch-home-')
            atexit.register(_home.cleanup)
        home = _home.name
    env['HOME'] = str(home)
    env.setdefault('TINKERY_NODE', '/tinkery-TEST-no-node')
    return _checked_environment(env, create=True)


def _checked_environment(env, *, create=False):
    """Refuse unsafe HOME before any subprocess API can start an executable."""
    if env is None or not env.get('HOME'):
        raise UnsafeTestHome('Test launches require an explicit temporary TEST HOME.')
    home = Path(env['HOME'])
    if not home.is_absolute():
        raise UnsafeTestHome('Test HOME must be absolute.')
    home = home.resolve()
    real_home = Path(pwd.getpwuid(os.getuid()).pw_dir).resolve()
    if home == real_home:
        raise UnsafeTestHome('Refusing to launch with the password-database real home.')
    temporary_roots = (Path(tempfile.gettempdir()).resolve(), Path('/tmp').resolve(),
                       Path('/var/tmp').resolve(),
                       (Path(__file__).resolve().parents[1] / 'target/tmp').resolve())
    if (not any(home.is_relative_to(root) for root in temporary_roots)
            or not any('TEST' in part for part in home.parts)):
        raise UnsafeTestHome('Test HOME must be a temporary TEST folder.')
    if create:
        home.mkdir(parents=True, exist_ok=True)
    if not home.is_dir():
        raise UnsafeTestHome('Test HOME must be an existing temporary TEST folder.')
    return dict(env)


def Popen(*args, env=None, **kwargs):
    """Start a process only with a guarded TEST HOME, including PTY launches."""
    return subprocess.Popen(*args, env=_checked_environment(env), **kwargs)


def run(*args, env=None, **kwargs):
    """Run a bounded test command with the same launch guard."""
    return subprocess.run(*args, env=_checked_environment(env), **kwargs)


def check_output(*args, env=None, **kwargs):
    """Capture test output with the same launch guard."""
    return subprocess.check_output(*args, env=_checked_environment(env), **kwargs)


if __name__ == '__main__' and len(sys.argv) > 1:
    raise SystemExit(run(sys.argv[1:], env=dict(os.environ)).returncode)
