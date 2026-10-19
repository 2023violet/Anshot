"""Compare the clipboard image read back by another process with the expected redaction.

Usage: python tests/desktop/compare_clipboard.py <source.png> <clipboard.png> x0 y0 x1 y1
Expected: pixels inside the half-open region equal Core COVER (17,24,39);
every other pixel equals the source RGB.
"""
import sys

from PIL import Image

COVER = (17, 24, 39)


def main() -> int:
    src_path, clip_path, *box = sys.argv[1:]
    x0, y0, x1, y1 = map(int, box)
    src = Image.open(src_path).convert("RGB")
    clip = Image.open(clip_path).convert("RGB")
    if src.size != clip.size:
        print(f"SIZE_MISMATCH source={src.size} clipboard={clip.size}")
        return 1
    w, h = src.size
    sp, cp = src.load(), clip.load()
    inside_bad = outside_bad = 0
    for y in range(h):
        for x in range(w):
            if x0 <= x < x1 and y0 <= y < y1:
                inside_bad += cp[x, y] != COVER
            else:
                outside_bad += cp[x, y] != sp[x, y]
    total_inside = (x1 - x0) * (y1 - y0)
    print(f"REGION {x0},{y0}-{x1},{y1} inside_pixels={total_inside} inside_mismatch={inside_bad} "
          f"outside_pixels={w * h - total_inside} outside_mismatch={outside_bad}")
    ok = inside_bad == 0 and outside_bad == 0
    print("CLIPBOARD_PIXELS_PASS" if ok else "CLIPBOARD_PIXELS_FAIL")
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
