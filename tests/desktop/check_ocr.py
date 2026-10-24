"""Score recognised text against the expected text (character accuracy, whitespace ignored),
and make sure redacted strings were NOT recognised.

Usage: python check_ocr.py ocr.txt "expected text" MIN_ACCURACY [forbidden ...]
Prints OCR_PASS or a reason.
"""
import sys


def dist(a, b):
    prev = list(range(len(b) + 1))
    for i, ca in enumerate(a, 1):
        cur = [i]
        for j, cb in enumerate(b, 1):
            cur.append(min(prev[j] + 1, cur[j - 1] + 1, prev[j - 1] + (ca != cb)))
        prev = cur
    return prev[-1]


def squash(s):
    return ''.join(s.split())


def main(path, expected, min_acc, forbidden):
    got = open(path, encoding='utf-8').read()
    g, e = squash(got), squash(expected)
    acc = 1 - dist(g, e) / max(1, len(e))
    leaked = [f for f in forbidden if squash(f) in g]
    if leaked:
        return f'FAIL redacted text recognised: {leaked}'
    if acc < min_acc:
        return f'FAIL accuracy {acc:.3f} < {min_acc} got={g!r}'
    return f'OCR_PASS accuracy={acc:.3f} chars={len(e)}'


if __name__ == '__main__':
    a = sys.argv[1:]
    print(main(a[0], a[1], float(a[2]), a[3:]))
