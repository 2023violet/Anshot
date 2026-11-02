"""Read an exported image back after the privacy check.

Usage: python tests/desktop/check_review.py <source.png> <export.png> <truth.json> <kinds-covered,comma>
PASS needs: no glyph pixel of a covered kind survives, every other value is untouched, and no
pixel changed more than MARGIN from a covered value (so no suggestion box reached the export).
"""
import json
import sys

from PIL import Image, ImageChops

MARGIN = 12


def main() -> int:
    src_path, out_path, truth_path, kinds = sys.argv[1:5]
    covered = set(kinds.split(','))
    src = Image.open(src_path).convert('RGB')
    out = Image.open(out_path).convert('RGB')
    if src.size != out.size:
        print(f'SIZE_MISMATCH {src.size} {out.size}')
        return 1
    diff = ImageChops.difference(src, out).convert('L').point(lambda v: 255 if v else 0)
    px = diff.load()
    allowed = Image.new('L', src.size, 0)
    bad = []
    for row in json.load(open(truth_path, encoding='utf-8')):
        x0, y0, x1, y1 = [int(v) for v in row['box']]
        box = diff.crop((x0, y0, x1, y1))
        changed = sum(1 for v in box.tobytes() if v)
        total = box.width * box.height
        if row['kind'] in covered:
            # The filled rectangle replaces every pixel except any that already had its colour.
            solid = out.crop((x0, y0, x1, y1)).getcolors(4)
            if not solid or len(solid) != 1:
                bad.append(f"{row['kind']} not covered ({changed}/{total} changed)")
            allowed.paste(255, (x0 - MARGIN, y0 - MARGIN, x1 + MARGIN, y1 + MARGIN))
        elif changed:
            bad.append(f"{row['kind']} touched ({changed}/{total} changed)")
    a = allowed.load()
    stray = sum(1 for y in range(src.height) for x in range(src.width) if px[x, y] and not a[x, y])
    print(f"REVIEW covered={sorted(covered)} problems={bad} stray_pixels={stray} {'PASS' if not bad and not stray else 'FAIL'}")
    return 0 if not bad and not stray else 1


sys.exit(main())
