"""The pointer was drawn into a capture: pixels differ from the source only in a small box at
the pointer position, and that box is not empty.

Usage: python check_cursor.py source.png captured.png X Y
Prints CURSOR_PASS or a reason.
"""
import sys
from PIL import Image


def main(src, got, px, py):
    a = Image.open(src).convert('RGB')
    b = Image.open(got).convert('RGB')
    if a.size != b.size:
        return f'FAIL size {b.size} != {a.size}'
    diff = [(x, y) for y in range(a.height) for x in range(a.width) if a.getpixel((x, y)) != b.getpixel((x, y))]
    if len(diff) < 20:
        return f'FAIL only {len(diff)} px differ: no pointer drawn'
    x0, y0 = min(p[0] for p in diff), min(p[1] for p in diff)
    x1, y1 = max(p[0] for p in diff), max(p[1] for p in diff)
    # A standard pointer is at most 32 x 32 px at 100 %, with its hotspot at the top-left.
    if not (px - 4 <= x0 <= px + 4 and py - 4 <= y0 <= py + 4 and x1 - x0 < 48 and y1 - y0 < 48):
        return f'FAIL differences at {(x0, y0, x1, y1)}, pointer was at {(px, py)}'
    return f'CURSOR_PASS diff={len(diff)} box={(x0, y0, x1, y1)}'


if __name__ == '__main__':
    a = sys.argv[1:]
    print(main(a[0], a[1], int(a[2]), int(a[3])))
