"""Run the OCR spike over the synthetic fixtures and report accuracy and latency.

Usage: python spike/ocr/evaluate.py [upscale]   (after make_fixtures.py and cargo build --release --locked)
Writes spike/ocr/out/<tag>/recognized.tsv, accuracy.json, latency.json; tag = x1 or x<upscale>.
The optional upscale is passed to the exe as OCR_UPSCALE (bilinear enlargement in Rust).
Accuracy = 1 - Levenshtein(truth, ocr) / len(truth), whitespace removed from both sides.
"nfkc" additionally applies NFKC (full-width punctuation -> half-width) to both sides.
"bag" is order-free: errors = max(missing, extra) character counts (a lower bound of the
edit distance), so a right-aligned bubble read out of line order is not penalised.
"""
import json
import os
import sys
import statistics
import subprocess
import time
from collections import Counter
import unicodedata
from pathlib import Path

HERE = Path(__file__).resolve().parent
EXE = HERE / "target/release/ocr-spike.exe"
FIX = HERE / "fixtures"
UPSCALE = sys.argv[1] if len(sys.argv) > 1 else None
OUT = HERE / "out" / (f"x{UPSCALE}" if UPSCALE else "x1")
ENV = {**os.environ, **({"OCR_UPSCALE": UPSCALE} if UPSCALE else {})}
LANG = "zh-Hans"  # only recognizer installed on the test machine (see RESULT.md)
SEP = "␤"


def lev(a, b):
    prev = list(range(len(b) + 1))
    for i, ca in enumerate(a, 1):
        cur = [i]
        for j, cb in enumerate(b, 1):
            cur.append(min(prev[j] + 1, cur[j - 1] + 1, prev[j - 1] + (ca != cb)))
        prev = cur
    return prev[-1]


def bag_dist(a, b):
    ca, cb = Counter(a), Counter(b)
    return max(sum((ca - cb).values()), sum((cb - ca).values()))


def squash(s, nfkc=False):
    s = s.replace(SEP, "")
    if nfkc:
        s = unicodedata.normalize("NFKC", s)
    return "".join(s.split())


def run(args):
    p = subprocess.run([str(EXE), *args], capture_output=True, encoding="utf-8", env=ENV)
    if p.returncode != 0:
        raise SystemExit(f"{args[0]} exit {p.returncode}: {p.stderr}")
    return p.stdout


def accuracy(truth):
    stems = list(truth)
    out = run(["recog", LANG, *[str(FIX / f"{s}.bgra") for s in stems]])
    got = {Path(l.split("\t", 1)[0]).stem: l.split("\t", 1)[1] for l in out.splitlines() if "\t" in l}
    rows, agg = [], {}
    for s in stems:
        t, o = "".join(truth[s]["text"]), got.get(s, "")
        r = {"stem": s, **{k: truth[s][k] for k in ("set", "size", "scale", "px")}}
        for key, nf in (("raw", False), ("nfkc", True), ("bag", False)):
            ts, os_ = squash(t, nf), squash(o, nf)
            d = bag_dist(ts, os_) if key == "bag" else lev(ts, os_)
            r[key] = {"len": len(ts), "dist": d, "acc": max(0.0, 1 - d / len(ts))}
            a = agg.setdefault((truth[s]["set"], key), [0, 0])
            a[0] += len(ts)
            a[1] += d
        r["ocr"] = o
        rows.append(r)
    sets = {f"{k[0]}/{k[1]}": {"len": v[0], "dist": v[1], "acc": 1 - v[1] / v[0]} for k, v in agg.items()}
    (OUT / "recognized.tsv").write_text(
        "".join(f"{r['stem']}\t{r['raw']['acc']:.4f}\t{r['ocr']}\n" for r in rows), encoding="utf-8")
    return {"sets": sets, "rows": rows}


def latency(n=10):
    """Cold: n fresh processes, one recognition each. Warm: n runs in one process, engine reused."""
    region = str(FIX / "region_600x400.bgra")
    cold, wall = [], []
    for _ in range(n):
        t = time.perf_counter()
        out = run(["bench", LANG, region, "0"])
        wall.append((time.perf_counter() - t) * 1000)
        cold += [float(l.split("\t")[1]) for l in out.splitlines() if l.startswith("cold_ms")]
    out = run(["bench", LANG, region, str(n)])
    warm = [float(l.split("\t")[1]) for l in out.splitlines() if l.startswith("warm_ms")]
    med = statistics.median
    return {"runs": n, "cold_ms": cold, "cold_median": med(cold), "process_wall_ms": wall,
            "process_wall_median": med(wall), "warm_ms": warm, "warm_median": med(warm)}


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    truth = json.loads((FIX / "truth.json").read_text(encoding="utf-8"))
    acc = accuracy(truth)
    (OUT / "accuracy.json").write_text(json.dumps(acc, ensure_ascii=False, indent=1), encoding="utf-8")
    for k, v in sorted(acc["sets"].items()):
        print(f"{k:12s} acc={v['acc']:.4f}  dist={v['dist']}/{v['len']}")
    lat = latency()
    (OUT / "latency.json").write_text(json.dumps(lat, indent=1), encoding="utf-8")
    print(f"cold median {lat['cold_median']:.1f} ms (process wall {lat['process_wall_median']:.1f} ms); "
          f"warm median {lat['warm_median']:.1f} ms")


if __name__ == "__main__":
    main()
