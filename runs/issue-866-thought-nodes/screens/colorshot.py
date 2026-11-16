"""Drive a Tinkery binary in a real PTY and render its coloured screen to PNG.

Usage: colorshot.py BINARY ROOT OUTDIR PREFIX
ROOT holds repo/, home/, sessions/, records/, links/ (TEST data). No provider is configured.
Supports the CSI subset ratatui/crossterm emit: cursor moves, erase, SGR with 24-bit colours, bold, dim.
"""
import os, pty, re, select, sys, time, codecs
from PIL import Image, ImageDraw, ImageFont

BIN, ROOT, OUT, PREFIX = sys.argv[1:5]
W, H = 100, 30
CSI = re.compile(r"\x1b\[([0-?]*)([ -/]*)([@-~])")


class Screen:
    def __init__(self):
        self.cells = [[(" ", None, None, False, False)] * W for _ in range(H)]
        self.x = self.y = 0
        self.fg = self.bg = None
        self.bold = self.dim = False
        self.pending = ""
        self.dec = codecs.getincrementaldecoder("utf-8")()

    def sgr(self, args):
        vals = [int(v or 0) for v in args.split(";")] if args else [0]
        i = 0
        while i < len(vals):
            v = vals[i]
            if v == 0:
                self.fg = self.bg = None; self.bold = self.dim = False
            elif v == 1: self.bold = True
            elif v == 2: self.dim = True
            elif v == 22: self.bold = self.dim = False
            elif v in (38, 48) and i + 4 < len(vals) + 0 and vals[i + 1] == 2:
                col = tuple(vals[i + 2:i + 5])
                if v == 38: self.fg = col
                else: self.bg = col
                i += 4
            elif v == 39: self.fg = None
            elif v == 49: self.bg = None
            i += 1

    def feed(self, data):
        self.pending += self.dec.decode(data)
        while self.pending:
            c = self.pending[0]
            if c == "\x1b":
                if self.pending.startswith("\x1b]"):
                    end = self.pending.find("\x07")
                    end2 = self.pending.find("\x1b\\")
                    ends = [e for e in (end, end2) if e >= 0]
                    if not ends: return
                    e = min(ends)
                    self.pending = self.pending[e + (1 if e == end else 2):]
                    continue
                m = CSI.match(self.pending)
                if not m:
                    # An escape split across reads: wait for the rest unless it cannot be a CSI.
                    if self.pending.startswith("\x1b[") and re.fullmatch(r"\x1b\[[0-?]*[ -/]*", self.pending):
                        return
                    if len(self.pending) < 2: return
                    self.pending = self.pending[2:]
                    continue
                args, _, cmd = m.groups()
                self.pending = self.pending[m.end():]
                if cmd == "m" and not args.startswith(("<", ">", "?")):
                    self.sgr(args); continue
                if args[:1] in ("?", ">", "<", "="): continue
                vals = [int(v or 0) for v in args.split(";")] if args else [0]
                n = vals[0] or 1
                if cmd in "Hf":
                    self.y = (vals[0] or 1) - 1
                    self.x = ((vals[1] if len(vals) > 1 else 1) or 1) - 1
                elif cmd == "G": self.x = n - 1
                elif cmd == "A": self.y -= n
                elif cmd == "B": self.y += n
                elif cmd == "C": self.x += n
                elif cmd == "D": self.x -= n
                elif cmd == "J" and vals[0] in (2, 3):
                    self.cells = [[(" ", None, self.bg, False, False)] * W for _ in range(H)]
                elif cmd == "K" and 0 <= self.y < H:
                    start = 0 if vals[0] in (1, 2) else self.x
                    end = self.x + 1 if vals[0] == 1 else W
                    for xx in range(max(start, 0), min(end, W)):
                        self.cells[self.y][xx] = (" ", None, self.bg, False, False)
            else:
                self.pending = self.pending[1:]
                if c == "\r": self.x = 0
                elif c == "\n": self.y += 1
                elif c >= " ":
                    if 0 <= self.x < W and 0 <= self.y < H:
                        self.cells[self.y][self.x] = (c, self.fg, self.bg, self.bold, self.dim)
                    self.x += 1

    def text(self):
        return "\n".join("".join(cell[0] for cell in row) for row in self.cells)


def png(screen, path, title):
    font = ImageFont.truetype("/System/Library/Fonts/Menlo.ttc", 22)
    bold = ImageFont.truetype("/System/Library/Fonts/Menlo.ttc", 22, index=1)
    sym = ImageFont.truetype("/System/Library/Fonts/Apple Symbols.ttf", 22)
    cw, lh = font.getbbox("M")[2], 30
    paper = next((c[2] for row in screen.cells for c in row if c[2]), (255, 252, 240))
    ink = (16, 15, 15)
    img = Image.new("RGB", (W * cw + 32, (H + 2) * lh + 16), paper)
    d = ImageDraw.Draw(img)
    d.text((16, 8), title, font=font, fill=(31, 122, 114))
    for y, row in enumerate(screen.cells):
        for x, (ch, fg, bg, b, dim) in enumerate(row):
            px, py = 16 + x * cw, 8 + (y + 2) * lh
            if bg and bg != paper:
                d.rectangle([px, py, px + cw, py + lh], fill=bg)
            if 0x2800 <= ord(ch) <= 0x28FF:
                # Braille cell: draw its dots, as a terminal does.
                bits = ord(ch) - 0x2800
                dots = [(0,0,0x01),(0,1,0x02),(0,2,0x04),(1,0,0x08),(1,1,0x10),(1,2,0x20),(0,3,0x40),(1,3,0x80)]
                for cx, cy, bit in dots:
                    if bits & bit:
                        ox, oy = px + cw * (0.3 + 0.4 * cx), py + lh * (0.15 + 0.23 * cy)
                        d.ellipse([ox - 2, oy - 2, ox + 2, oy + 2], fill=fg or ink)
            elif ch != " ":
                f = bold if b else font
                if f.getmask(ch).getbbox() is None:
                    f = sym
                d.text((px, py), ch, font=f, fill=fg or ink)
    img.save(path)


pid, fd = pty.fork()
if pid == 0:
    os.chdir(os.path.join(ROOT, "repo"))
    env = {"PATH": "/usr/bin:/bin", "HOME": os.path.join(ROOT, "home"), "TERM": "xterm-256color",
           "COLORTERM": "truecolor", "TINKERY_NODE": "/tinkery-TEST-no-node", "LANG": "en_US.UTF-8"}
    os.execve(BIN, [BIN, "--home", "--operator", "agent", "--seed-session-root", os.path.join(ROOT, "sessions"),
                    "--record-root", os.path.join(ROOT, "records"), "--link-root", os.path.join(ROOT, "links")], env)
import fcntl, struct, termios
fcntl.ioctl(fd, termios.TIOCSWINSZ, struct.pack("HHHH", H, W, 0, 0))
screen = Screen()


def pump(seconds):
    end = time.time() + seconds
    while time.time() < end:
        r, _, _ = select.select([fd], [], [], 0.05)
        if r:
            try:
                data = os.read(fd, 65536)
            except OSError:
                return
            # Answer device-attribute and kitty keyboard queries so crossterm does not stall.
            if b"\x1b[c" in data or b"\x1b[?u" in data:
                os.write(fd, b"\x1b[?0u\x1b[?62c")
            screen.feed(data)


def wait_for(text, seconds=10):
    end = time.time() + seconds
    while time.time() < end:
        pump(0.2)
        if text in screen.text():
            pump(0.4)
            return
    raise SystemExit(f"missing {text!r}:\n{screen.text()}")


steps = [s.split("|", 2) for s in os.environ["SHOT_STEPS"].split("\n") if s.strip()]
wait_for("1  think", 15)
for keys, expect, name in steps:
    os.write(fd, keys.encode().decode("unicode_escape").encode("latin-1"))
    wait_for(expect)
    if name:
        png(screen, os.path.join(OUT, f"{PREFIX}-{name}.png"), f"{PREFIX}-{name} · 100x30 · TEST roots, no provider")
        open(os.path.join(OUT, f"{PREFIX}-{name}.txt"), "w").write(screen.text())
os.write(fd, b"\x1bq")
pump(0.5)
os.kill(pid, 15)
print("ok", PREFIX)
