"""Print truth vs OCR text for a sample of each kind, to see why values are missed."""
import sys
from collections import defaultdict
from pathlib import Path

ROOT = Path(__file__).parent
truth = [r.split('\t') for r in (ROOT / 'fixtures/sensitive.tsv').read_text(encoding='utf-8').splitlines()]
lines = defaultdict(lambda: defaultdict(list))
for row in Path(sys.argv[1]).read_text(encoding='utf-8').splitlines():
    path, line, *_, text = row.split('\t')
    lines[Path(path).name][int(line)].append(text)
step = int(sys.argv[2]) if len(sys.argv) > 2 else 37
for t in truth[::step]:
    name, font, size, theme, kind, value = t[:6]
    got = ' // '.join('|'.join(w) for w in lines[name].values())
    print(f'{font} {size} {theme} {kind}\n  T {value}\n  O {got}')
