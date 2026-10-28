"""Own one guarded terminal process, its screen and descriptors; feature waits stay local."""
import fcntl
import os
import signal
import struct
import termios
import test_launch as subprocess
from vt_screen import Screen


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
