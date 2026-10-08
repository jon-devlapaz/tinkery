"""Small UTF-8 screen reader for the cursor/style sequences emitted by this demo."""
import codecs
import re

CSI = re.compile(r"\x1b\[([0-?]*)([ -/]*)([@-~])")


class Screen:
    def __init__(self, width, height):
        self.width, self.height = width, height
        self.cells = [[" "] * width for _ in range(height)]
        self.x = self.y = 0
        self.pending = ""
        self.decoder = codecs.getincrementaldecoder("utf-8")()

    def feed(self, data):
        self.pending += self.decoder.decode(data)
        while self.pending:
            if self.pending[0] == "\x1b":
                match = CSI.match(self.pending)
                if not match:
                    if len(self.pending) == 1 or self.pending.startswith("\x1b["):
                        return
                    raise AssertionError(f"Unsupported terminal escape: {self.pending!r}")
                arguments, _, command = match.groups()
                self.pending = self.pending[match.end():]
                if arguments.startswith("?") or command == "m":
                    continue
                values = [int(value or "0") for value in arguments.split(";")]
                n = values[0] or 1
                if command in ("H", "f"):
                    self.y = (values[0] or 1) - 1
                    self.x = (values[1] or 1) - 1 if len(values) > 1 else 0
                elif command == "G":
                    self.x = n - 1
                elif command == "A":
                    self.y -= n
                elif command == "B":
                    self.y += n
                elif command == "C":
                    self.x += n
                elif command == "D":
                    self.x -= n
                elif command == "J" and values[0] in (2, 3):
                    self.cells = [[" "] * self.width for _ in range(self.height)]
                elif command == "K" and 0 <= self.y < self.height:
                    start = 0 if values[0] in (1, 2) else self.x
                    end = self.x + 1 if values[0] == 1 else self.width
                    self.cells[self.y][start:end] = [" "] * (end - start)
                else:
                    raise AssertionError(f"Unsupported terminal command: {command} {arguments}")
            else:
                char, self.pending = self.pending[0], self.pending[1:]
                if char == "\r":
                    self.x = 0
                elif char == "\n":
                    self.y += 1
                elif char == "\x08":
                    self.x -= 1
                elif char >= " ":
                    if 0 <= self.x < self.width and 0 <= self.y < self.height:
                        self.cells[self.y][self.x] = char
                    self.x += 1

    def text(self):
        return "\n".join("".join(row) for row in self.cells)
