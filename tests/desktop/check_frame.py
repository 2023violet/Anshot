"""Beautify e2e checks (ADR 0.5-BEAUTIFY).

python tests/desktop/check_frame.py inner <plain.png> <framed.png> <ratio|free> <out.png>
    The plain export appears unchanged inside the framed one (rounded corners excepted),
    the framed size has the ratio, and the screenshot area is written to out.png.
python tests/desktop/check_frame.py big <out.png>
    A 6000x4000 synthetic image that compresses well (under the 25 MiB input limit).
"""
import math
import sys

import numpy as np
from PIL import Image


def rgb(path):
    return np.asarray(Image.open(path).convert("RGB"), dtype=np.int16)


def inner(plain_path, framed_path, ratio, out):
    plain, framed = rgb(plain_path), rgb(framed_path)
    h, w, _ = plain.shape
    fh, fw, _ = framed.shape
    ok_ratio = True
    if ratio != "free":
        rw, rh = (int(v) for v in ratio.split(":"))
        ok_ratio = abs(fw * rh - fh * rw) < max(rw, rh)
    # Corners may be rounded: radius is at most 2.4% of the long side, at least 12 px.
    r = math.ceil(max(12, max(w, h) * 0.024)) + 2
    mask = np.ones((h, w), bool)
    for ys in (slice(0, r), slice(h - r, h)):
        for xs in (slice(0, r), slice(w - r, w)):
            mask[ys, xs] = False
    # The screenshot is centred horizontally; chrome above it moves it down.
    found = None
    x0 = (fw - w) // 2
    for y0 in range(0, fh - h + 1):
        if not np.array_equal(framed[y0 + h // 2, x0:x0 + w], plain[h // 2]):
            continue
        part = framed[y0:y0 + h, x0:x0 + w]
        if np.array_equal(part[mask], plain[mask]):
            found = (x0, y0)
            break
    if found:
        # The rounded corners are the frame's (checked above to be the only difference); the
        # read-back of covers then sees the rest exactly as delivered.
        part = framed[found[1]:found[1] + h, found[0]:found[0] + w].copy()
        part[~mask] = plain[~mask]
        Image.fromarray(part.astype(np.uint8)).save(out)
    ok = found is not None and ok_ratio
    print(f"FRAME size={fw}x{fh} plain={w}x{h} at={found} ratio_ok={ok_ratio} {'PASS' if ok else 'FAIL'}")
    return 0 if ok else 1


def big(out):
    y, x = np.mgrid[0:4000, 0:6000]
    img = np.stack([(x // 40) % 256, (y // 40) % 256, ((x + y) // 80) % 256], axis=-1).astype(np.uint8)
    Image.fromarray(img).save(out, optimize=True)
    print(f"BIG {out}")
    return 0


if __name__ == "__main__":
    mode, *a = sys.argv[1:]
    sys.exit(inner(*a) if mode == "inner" else big(*a))
