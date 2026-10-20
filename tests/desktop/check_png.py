"""Checks used by e2e parts 10-14.

python tests/desktop/check_png.py chunks <file.png>            -> only IHDR/IDAT/IEND, no metadata chunks
python tests/desktop/check_png.py same <a.png> <b.png>         -> identical RGB pixels
python tests/desktop/check_png.py colour <file.png> rrggbb min  -> at least `min` pixels of that colour
python tests/desktop/check_png.py band <file.png> x0 y0 x1 y1 rrggbb min -> near-colour count inside a box
"""
import struct
import sys

from PIL import Image


def chunks(path):
    data = open(path, "rb").read()
    assert data[:8] == b"\x89PNG\r\n\x1a\n", "not a PNG"
    i, kinds = 8, []
    while i < len(data):
        n = struct.unpack(">I", data[i:i + 4])[0]
        kinds.append(data[i + 4:i + 8].decode("latin-1"))
        i += 12 + n
    bad = sorted(set(kinds) - {"IHDR", "IDAT", "IEND"})
    print(f"CHUNKS {'/'.join(dict.fromkeys(kinds))} extra={bad}")
    return 0 if not bad else 1


def rgb(path):
    return Image.open(path).convert("RGB")


def count(img, box, c, tol=0):
    px = img.load()
    x0, y0, x1, y1 = box
    near = lambda p: abs(p[0] - c[0]) + abs(p[1] - c[1]) + abs(p[2] - c[2]) <= tol
    return sum(1 for y in range(y0, y1) for x in range(x0, x1) if near(px[x, y]))


def main():
    mode, *a = sys.argv[1:]
    if mode == "chunks":
        return chunks(a[0])
    if mode == "same":
        x, y = rgb(a[0]), rgb(a[1])
        diff = -1 if x.size != y.size else sum(1 for p, q in zip(x.tobytes(), y.tobytes()) if p != q)  # differing RGB bytes
        print(f"SAME diff={diff}")
        return 0 if diff == 0 else 1
    if mode == "colour":
        img = rgb(a[0]); c = tuple(int(a[1][i:i + 2], 16) for i in (0, 2, 4))
        n = count(img, (0, 0, *img.size), c)
        print(f"COLOUR {a[1]} count={n}")
        return 0 if n >= int(a[2]) else 1
    if mode == "band":
        img = rgb(a[0]); box = tuple(map(int, a[1:5])); c = tuple(int(a[5][i:i + 2], 16) for i in (0, 2, 4))
        n = count(img, box, c, tol=90)  # anti-aliased glyph edges
        print(f"BAND {box} {a[5]} count={n}")
        return 0 if n >= int(a[6]) else 1
    raise SystemExit("unknown mode")


if __name__ == "__main__":
    sys.exit(main())
