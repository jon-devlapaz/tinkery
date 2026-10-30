"""Own one guarded terminal process, its screen and descriptors; feature waits stay local."""
import errno
import fcntl
import os
import select
import signal
import struct
import termios
import time
import test_launch as subprocess
from vt_screen import Screen


def wait_output(master, process, consume, deadline, *, interval=.02):
    """Yield the observed return code after reading, draining output on exit.

    The absolute monotonic deadline bounds reads and exit draining. Callers own
    success, frame readiness and diagnostics. EOF and an exited PTY's EIO finish
    a read pass; other read errors remain failures. No frame is required here.
    """
    def read_pending(timeout):
        if not select.select([master], [], [], timeout)[0]:
            return False
        try:
            chunk = os.read(master, 65536)
        except OSError as error:
            if error.errno != errno.EIO or process.poll() is None:
                raise
            return False
        if not chunk:
            return False
        consume(chunk)
        return True

    while time.monotonic() < deadline:
        started = time.monotonic()
        read_output = read_pending(min(interval, max(0, deadline - started)))
        returncode = process.poll()
        if not read_output and returncode is None:
            remaining = min(started + interval, deadline) - time.monotonic()
            if remaining > 0:
                time.sleep(remaining)
        if returncode is not None:
            # Exit is not EOF: unread final output can span many small reads.
            while time.monotonic() < deadline and read_pending(0):
                pass
        yield returncode
        if returncode is not None:
            return


class TerminalProcess:
    def __init__(self, argv, *, cwd, env, width, height):
        self.master, self.slave = os.openpty()
        self.raw = bytearray()
        try:
            self.resize(width, height)
            self.process = subprocess.Popen(argv, cwd=cwd, env=env,
                                            stdin=self.slave, stdout=self.slave, stderr=self.slave)
        except BaseException:
            os.close(self.master)
            os.close(self.slave)
            raise

    def resize(self, width, height):
        fcntl.ioctl(self.slave, termios.TIOCSWINSZ, struct.pack('HHHH', height, width, 0, 0))
        self.screen = Screen(width, height)
        if hasattr(self, 'process'):
            os.kill(self.process.pid, signal.SIGWINCH)

    def feed(self):
        chunk = os.read(self.master, 65536)
        self.raw.extend(chunk)
        self.screen.feed(chunk)

    def close(self):
        if self.process.poll() is None:
            self.process.kill()
        self.process.wait()
        os.close(self.master)
        os.close(self.slave)
