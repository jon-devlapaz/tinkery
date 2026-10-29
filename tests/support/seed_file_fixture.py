"""Non-regular TEST session files with a timed FIFO release, so reverted readers cannot hang QA."""
from contextlib import contextmanager
import os
import threading


@contextmanager
def replaced_seed(seed, kind):
    """Replace a session file; release blocking readers after three seconds until cleanup."""
    original = seed.read_bytes()
    seed.unlink()
    stopped = threading.Event()
    release = None
    if kind == 'directory':
        seed.mkdir()
    else:
        os.mkfifo(seed)
        def release_readers():
            if not stopped.wait(3):
                while not stopped.is_set():
                    descriptor = os.open(seed, os.O_RDWR | os.O_NONBLOCK)
                    os.close(descriptor)
                    stopped.wait(.02)
        release = threading.Thread(target=release_readers)
        release.start()
    try:
        yield
    finally:
        stopped.set()
        if release:
            release.join()
        if seed.is_dir():
            seed.rmdir()
        else:
            seed.unlink()
        seed.write_bytes(original)
