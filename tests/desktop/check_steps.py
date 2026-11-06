"""Checks for steps_e2e.ps1 (step image, ADR 0.6-STEPS).

python tests/desktop/check_steps.py find <image.png> <piece.png> [x y w h]  -> FOUND x y | MISSING
    the piece (or its crop) appears in the image pixel for pixel (RGB)
python tests/desktop/check_steps.py opaque <image.png>                       -> OPAQUE yes|no
python tests/desktop/check_steps.py nobytes <dir> <text>                      -> ABSENT n_files | PRESENT <file>
    no file under dir holds the text (UTF-8 or UTF-16LE)
"""
import os
import sys

import numpy as np
from PIL import Image


def rgb(path):
    return np.asarray(Image.open(path).convert("RGB"))


def find(image, piece, box=None):
    big, small = rgb(image), rgb(piece)
    if box:
        x, y, w, h = box
        small = small[y:y + h, x:x + w]
    h, w = small.shape[:2]
    first = small[0].tobytes()
    for yy in range(big.shape[0] - h + 1):
        row = big[yy].tobytes()
        start = row.find(first)
        while start != -1:
            if start % 3 == 0:
                xx = start // 3
                if xx + w <= big.shape[1] and np.array_equal(big[yy:yy + h, xx:xx + w], small):
                    print(f"FOUND {xx} {yy}")
                    return 0
            start = row.find(first, start + 1)
    print("MISSING")
    return 1


def opaque(image):
    a = np.asarray(Image.open(image).convert("RGBA"))[..., 3]
    ok = bool((a == 255).all())
    print(f"OPAQUE {'yes' if ok else 'no'}")
    return 0 if ok else 1


def nobytes(folder, text):
    needles = [text.encode("utf-8"), text.encode("utf-16-le")]
    n = 0
    for root, _, files in os.walk(folder):
        for f in files:
            n += 1
            data = open(os.path.join(root, f), "rb").read()
            if any(x in data for x in needles):
                print(f"PRESENT {f}")
                return 1
    print(f"ABSENT {n}")
    return 0


if __name__ == "__main__":
    cmd = sys.argv[1]
    if cmd == "find":
        box = tuple(int(v) for v in sys.argv[4:8]) if len(sys.argv) >= 8 else None
        sys.exit(find(sys.argv[2], sys.argv[3], box))
    if cmd == "opaque":
        sys.exit(opaque(sys.argv[2]))
    if cmd == "nobytes":
        sys.exit(nobytes(sys.argv[2], sys.argv[3]))
    sys.exit(2)
