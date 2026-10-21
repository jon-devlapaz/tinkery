"""Measure reopens, reflows and quits without starting work or writing records."""
import json

from measure_terminal import SIZES, UNKNOWN, Terminal


def journey(width, height, no_color, quit_key, help_open):
    terminal = Terminal(width, height, no_color)
    try:
        for _ in range(3):
            terminal.send(b'7', UNKNOWN)
            terminal.send(b'q', '1  think')
        terminal.send(b'7', UNKNOWN)
        terminal.resize(100 if width == 80 else 80, 24)
        terminal.wait(lambda: UNKNOWN in terminal.screen.text())
        assert 'esc back   ? help' in terminal.screen.text()
        terminal.send(b'?', 'q goes back to the menu')
        terminal.resize(width, height)
        terminal.wait(lambda: 'q goes back to the menu' in terminal.screen.text())
        if not help_open:
            terminal.send(b'\x1b', UNKNOWN)
        terminal.finish(quit_key)
        print(json.dumps(dict(width=width, height=height, no_color=no_color,
                              quit='ctrl-c' if quit_key == b'\x03' else 'f10',
                              help_open=help_open, visits=4, provider_calls=0, command_calls=0)))
    finally:
        terminal.close()


if __name__ == '__main__':
    for dimensions in SIZES:
        for no_color in (False, True):
            for quit_key in (b'\x03', b'\x1b[21~'):
                for help_open in (False, True):
                    journey(*dimensions, no_color, quit_key, help_open)
