"""B3 clipboard check: safe mosaic + arrow + text on the synthetic shot.

Usage: python tests/desktop/compare_b3.py <source.png> <clipboard.png>
Checks (source px): mosaic region (374,143)-(476,163) only uses the safe palette;
the arrow box and text box contain red ink pixels; every other pixel equals the source.
"""
import sys

from PIL import Image

PALETTE = {(0x3a, 0x39, 0x34), (0x4b, 0x4a, 0x44), (0x5d, 0x5b, 0x54), (0x6f, 0x6d, 0x65)}
RED = (0xe5, 0x34, 0x2b)
MOSAIC = (374, 143, 476, 163)
ARROW = (348, 288, 462, 372)
TEXT = (392, 372, 480, 416)


def inside(b, x, y):
    return b[0] <= x < b[2] and b[1] <= y < b[3]


def main() -> int:
    src = Image.open(sys.argv[1]).convert("RGB")
    clip = Image.open(sys.argv[2]).convert("RGB")
    if src.size != clip.size:
        print(f"SIZE_MISMATCH {src.size} {clip.size}")
        return 1
    sp, cp = src.load(), clip.load()
    w, h = src.size
    off_palette = red_arrow = red_text = outside_bad = 0
    for y in range(h):
        for x in range(w):
            p = cp[x, y]
            if inside(MOSAIC, x, y):
                off_palette += p not in PALETTE
            elif inside(ARROW, x, y) or inside(TEXT, x, y):
                red_arrow += inside(ARROW, x, y) and p == RED
                red_text += inside(TEXT, x, y) and abs(p[0] - RED[0]) + abs(p[1] - RED[1]) + abs(p[2] - RED[2]) < 60
            else:
                outside_bad += p != sp[x, y]
    print(f"MOSAIC off_palette={off_palette} ARROW red={red_arrow} TEXT red_like={red_text} OUTSIDE mismatch={outside_bad}")
    ok = off_palette == 0 and red_arrow > 150 and red_text > 15 and outside_bad == 0
    print("B3_PIXELS_PASS" if ok else "B3_PIXELS_FAIL")
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
