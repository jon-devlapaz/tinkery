"""A late PageDown must not look like the end of the reading."""
import os
import threading
import unittest
from unittest import mock

import reading_scroll_terminal as reading


class ReadingScrollTests(unittest.TestCase):
    def test_a_late_page_down_still_shows_the_last_exclusion(self):
        write = os.write
        delayed = None

        def delay_one_page(fd, data):
            nonlocal delayed
            if delayed is None and data == b'\x1b[6~':
                delayed = threading.Timer(.5, write, args=(fd, data))
                delayed.start()
                return len(data)
            return write(fd, data)

        try:
            with mock.patch('os.write', side_effect=delay_one_page):
                reading.journey(100, 30, False)
            self.assertIsNotNone(delayed)
        finally:
            if delayed is not None:
                delayed.cancel()
                delayed.join()


if __name__ == '__main__':
    unittest.main()
