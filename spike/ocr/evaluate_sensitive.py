"""M0 gate. Tolerant rules over OCR words, optionally the union of several OCR runs.
Recall: a finding overlaps the value's glyph box (the category may differ; the user still sees
it). Cover: the finding's boxes, padded, contain the glyph box. False positives: findings on
ordinary lines. Usage: evaluate_sensitive.py <pad> <words.tsv>..."""
import re
import sys
import unicodedata
from collections import defaultdict
from pathlib import Path

ROOT = Path(__file__).parent
PAD = float(sys.argv[1])
LOOK = 'OolI|eScgBZ'   # read in place of a digit
SEP = '.。·,，:：'


def norm(c):
    return unicodedata.normalize('NFKC', c)


def is_ascii_token(c):
    return c.isascii() and (c.isalnum() or c in '._%+-')


def digit_like(text, i):
    return text[i].isdigit() or text[i] in LOOK


def number_runs(text):
    """Digits with up to 2 misread characters inside; ends are real digits."""
    i, n = 0, len(text)
    while i < n:
        if not text[i].isdigit():
            i += 1
            continue
        j, bad, end, real = i, 0, i, 0
        while j < n:
            if text[j].isdigit():
                real += 1
                end = j
            elif digit_like(text, j) or not text[j].isascii():
                bad += 1
                if bad > 2:
                    break
            else:
                break
            j += 1
        yield i, end + 1, real
        i = max(end + 1, i + 1)


def findings(text):
    spans = []
    for s, e, real in number_runs(text):
        length = e - s
        if real >= 9 and length in (11, 12, 13) and text[s] == '1':
            spans.append((s, e))
        elif real >= 13 and 15 <= length <= 20:
            spans.append((s, e))
    # IPv4 with any single separator the engine put in place of the dots.
    for m in re.finditer(r'(?<![\d.])\d{1,3}(?:[^\dA-Za-z\s]{1,2}\d{1,3}){3}(?![\d])', text):
        spans.append(m.span())
    for m in re.finditer(r'(?:^|[^A-Za-z0-9._%+-])([A-Za-z0-9._%+-]{2,}@[A-Za-z0-9.-]{3,})', text):
        spans.append(m.span(1))
    for m in re.finditer(r'(?:sk-|gh[pos]_|xox[bps]-|AK[Il1]A|g\s*h\s*p_)[A-Za-z0-9_-]{8,}', text):
        spans.append(m.span())
    return spans


truth = {}
for row in (ROOT / 'fixtures/sensitive.tsv').read_text(encoding='utf-8').splitlines():
    name, font, size, theme, kind, value, *box = row.split('\t')
    truth[name] = dict(font=font, size=int(size), theme=theme, kind=kind, value=value, box=tuple(map(float, box)))

boxes = defaultdict(list)
for path in sys.argv[2:]:
    lines = defaultdict(lambda: defaultdict(list))
    for row in Path(path).read_text(encoding='utf-8').splitlines():
        p, line, x, y, w, h, text = row.split('\t')
        lines[Path(p).name][int(line)].append((text, float(x), float(y), float(w), float(h)))
    for name, ls in lines.items():
        for ws in ls.values():
            text, owner = '', []
            for i, w in enumerate(ws):
                for c in norm(w[0]):
                    text += c
                    owner.append(i)
            for s, e in findings(text):
                hit = [ws[i] for i in sorted(set(owner[s:e]))]
                boxes[name].append((min(w[1] for w in hit), min(w[2] for w in hit), max(w[1] + w[3] for w in hit), max(w[2] + w[4] for w in hit)))

groups = defaultdict(lambda: [0, 0, 0])
fp = total = 0
for name, t in truth.items():
    if t['kind'] == 'none':
        total += 1
        fp += bool(boxes[name])
        continue
    b = t['box']
    near = [x for x in boxes[name] if x[0] < b[2] and x[2] > b[0] and x[1] < b[3] and x[3] > b[1]]
    found = bool(near)
    if near:
        u = (min(x[0] for x in near) - PAD, min(x[1] for x in near) - PAD, max(x[2] for x in near) + PAD, max(x[3] for x in near) + PAD)
    cover = found and u[0] <= b[0] and u[1] <= b[1] and u[2] >= b[2] and u[3] >= b[3]
    for key in ('all', t['kind'], t['font'], f"{t['size']}px", t['theme']):
        g = groups[key]
        g[0] += 1
        g[1] += found
        g[2] += bool(cover)

print('group\tn\trecall\tcover')
for key, (n, f, c) in sorted(groups.items()):
    print(f'{key}\t{n}\t{f / n:.1%}\t{c / max(f, 1):.1%}')
print(f'false positives on ordinary lines: {fp}/{total}')
