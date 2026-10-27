"""Spotlight + magnifier on the synthetic chat (e2e part 34).

The copy must be darkened outside the spotlight (to 45 %), unchanged inside it, and the
magnifier callout must show the redacted (mosaic) pixels enlarged, never the source digits.

Usage: python check_spotlight.py source.png copy.png OUT_X OUT_Y IN_X IN_Y
Prints SPOTLIGHT_PASS or a reason; also prints how many callout pixels are mosaic greys.
"""
import sys
from PIL import Image

MOSAIC = {(0x3a, 0x39, 0x34), (0x4b, 0x4a, 0x44), (0x5d, 0x5b, 0x54), (0x6f, 0x6d, 0x65)}
KEEP = 0.45


def main(src, got, ox, oy, ix, iy):
    a = Image.open(src).convert('RGB')
    b = Image.open(got).convert('RGB')
    o, d = a.getpixel((ox, oy)), b.getpixel((ox, oy))
    want = tuple(round(c * KEEP) for c in o)
    if any(abs(x - y) > 1 for x, y in zip(d, want)):
        return f'FAIL outside {d} != {want}'
    if b.getpixel((ix, iy)) != a.getpixel((ix, iy)):
        return f'FAIL inside changed {b.getpixel((ix, iy))} != {a.getpixel((ix, iy))}'
    # Pixels that are mosaic grey in the copy but not anywhere near in the source: the
    # mosaic itself plus its enlarged copy in the callout (callout >= 4x the mosaic area).
    grey = sum(1 for p in b.getdata() if p in MOSAIC)
    src_grey = sum(1 for p in a.getdata() if p in MOSAIC)
    # The mosaic itself sits at (506..595, 141..162); everything else that is mosaic grey is
    # the callout's enlarged copy, which must be about 4x the mosaic (at least 3x).
    own = sum(1 for y in range(141, 162) for x in range(506, 595) if b.getpixel((x, y)) in MOSAIC)
    if src_grey or own == 0 or grey - own < 3 * own:
        return f'FAIL callout: mosaic_px={grey} own={own} source_mosaic_px={src_grey}'
    return f'SPOTLIGHT_PASS mosaic_px={grey} own={own} callout={grey - own}'


if __name__ == '__main__':
    x = sys.argv[1:]
    print(main(x[0], x[1], int(x[2]), int(x[3]), int(x[4]), int(x[5])))
