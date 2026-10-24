# OCR feasibility spike — Windows.Media.Ocr from an unpackaged Rust exe

Date 2026-10-24 · repo HEAD `0dee02582691870818d879f318fdea71e935ccce` (spike files uncommitted) · agent-run, not a human test.

## Verdict against the 0.3.0 gate

| Gate criterion | Measured | Result |
|---|---|---|
| Chinese synthetic char accuracy ≥ 95 % | **96.44 %** (35 errors / 984 chars, 12 images, no preprocessing) | **PASS** (MEASURED) |
| 600×400 region in < 1 s | cold **78.2 ms** median (process wall 92.7 ms), warm **40.7 ms** median | **PASS** (MEASURED) |

**Gate: PASS — with limits that the 0.3.0 design must absorb** (see "Limits"):

- At **100 % scale** (13–16 px native) Chinese is only **92.1 %**; the aggregate passes because the 125 %/150 % slices are 98.5–98.8 %. A **1.5× bilinear upscale in Rust** before OCR fixes this: Chinese **99.70 %** overall and **99.1 %** at 100 % scale, for a cold time of 128 ms. Recommend making it the default for small regions.
- The realistic synthetic chat (`spike/assets/shot.png`) is only **85.8 %** (86.7 % with 1.5×): coloured bubbles, a phone number on green, arrows `→` and an en-dash `–` are misread. OCR output must be presented as editable/copyable text with a "may contain errors" note, never as authoritative.
- Only **zh-Hans-CN** is installed on this machine; `en-US` cannot be created. English was read by the zh-Hans engine (98.5 %). Language availability depends on the user's installed Windows language packs.

## 1. Can an unpackaged exe create the engine? — yes (MEASURED)

`ocr-spike.exe langs` (plain `cargo build` exe, no MSIX/package identity, `RoInitialize(RO_INIT_MULTITHREADED)`), exit 0:

```
MaxImageDimension	10000
available	zh-Hans-CN	中文(简体，中国)
create	user	OK	zh-Hans-CN                  # TryCreateFromUserProfileLanguages
IsLanguageSupported	zh-Hans	true
create	zh-Hans	OK	zh-Hans-CN                 # TryCreateFromLanguage("zh-Hans")
IsLanguageSupported	zh-Hans-CN	true
create	zh-Hans-CN	OK	zh-Hans-CN
IsLanguageSupported	en-US	false
create	en-US	FAILED	Error { code: HRESULT(0x00000000), ... }
```

`OcrEngine::AvailableRecognizerLanguages` = **[zh-Hans-CN]** only. Note for the product: when the language is unavailable `TryCreateFromLanguage` returns null, which windows-rs 0.62 surfaces as `Err` with HRESULT `S_OK` — treat any `Err` as "language not installed", and check `IsLanguageSupported` first.

## 2. Accuracy (MEASURED, synthetic only)

Fixtures (`make_fixtures.py`): 8 invented UI strings per set, rendered with the repo font `assets/fonts/NotoSansCJKsc-Regular.otf` (Pillow 12.3.0, grayscale antialiasing, `#222` on white) at 13/14/15/16 px × 100/125/150 % scale (13–24 px actual) = 12 images per set; plus `spike/assets/shot.png` (620×430 synthetic chat, ground truth copied from `spike/make_shot.py`, including the header `客服 · 小林`). Pixels are passed to OCR as raw BGRA via `SoftwareBitmap::CreateCopyFromBuffer` — no image decoding outside Rust. Engine: `zh-Hans` for every set.

Metric: `1 − Levenshtein(truth, ocr) / len(truth)`, whitespace removed on both sides, aggregated per set (Σdist / Σlen). `nfkc` also NFKC-normalises both sides (full-width ↔ half-width punctuation). `bag` is order-free (max of missing/extra character counts), shown because the engine reads the right-aligned green bubble out of line order at higher upscales, which the order-sensitive metric punishes heavily.

| Set (chars) | raw ×1 | nfkc ×1 | raw ×1.5 | raw ×2 | raw ×3 | bag ×2 | bag ×3 |
|---|---|---|---|---|---|---|---|
| zh (984) | **96.44 %** | 96.44 % | **99.70 %** | 99.80 % | 99.49 % | 99.80 % | 99.49 % |
| en (2316) | 98.49 % | 98.49 % | 98.06 % | 97.75 % | 97.84 % | 97.75 % | 97.84 % |
| mixed (1320) | 94.32 % | 96.21 % | 96.44 % | 96.44 % | 96.21 % | 96.52 % | 96.29 % |
| shot (120) | 85.83 % | 86.67 % | 86.67 % | 46.67 %* | 42.50 %* | 91.67 % | 92.50 % |

\* line-order artifact, not a recognition collapse (see `bag`).

By scale (raw, per set):

| | zh 100 % | zh 125 % | zh 150 % | mixed 100 % | mixed 125 % | mixed 150 % | en 100 % | en 125 % | en 150 % |
|---|---|---|---|---|---|---|---|---|---|
| ×1 | 92.1 % | 98.5 % | 98.8 % | 92.5 % | 95.0 % | 95.5 % | 97.8 % | 98.6 % | 99.1 % |
| ×1.5 | 99.1 % | 100 % | 100 % | 95.0 % | 97.0 % | 97.3 % | 98.3 % | 97.7 % | 98.2 % |

Typical errors at ×1, 100 %: visually similar glyphs (撤销→撒销/撤誚, 剪贴→剪阽, 遮挡→遮扌当, 设置→设酋, 使用→使牛), dropped characters (截图→截), Latin `o`→`O`/`0`, `l`→`I`, `+`→`十`, `.`→`．` in mixed text. Shot: `小林`→`刁啉`, `手机 138 0013 8000`→`手 138 00B 80 開` (green bubble), `→` dropped, `–`→`一`. Full per-image output: `out/x*/recognized.tsv`, `out/x*/accuracy.json`.

## 3. Latency, 600×400 region (MEASURED)

Region: `fixtures/region_600x400.bgra`, crop (10,0)-(610,400) of `shot.png` (all five bubbles, 121 recognised chars). Cold = fresh process; timer covers engine creation + BGRA upload + first `RecognizeAsync().join()`. Warm = same process, engine reused; each run includes upload + recognition. 10 runs each.

| Upscale | cold median | process wall median | warm median |
|---|---|---|---|
| ×1 | **78.2 ms** | 92.7 ms | **40.7 ms** |
| ×1.5 | 127.7 ms | 143.6 ms | 90.7 ms |
| ×2 | 182.9 ms | 202.0 ms | 151.8 ms |
| ×3 | 327.8 ms | 343.3 ms | 339.5 ms |

×1 raw runs: cold `[79, 78, 74, 79, 76, 71, 78, 78, 83, 101]` ms, warm `[68, 70, 40, 39, 42, 40, 43, 41, 40, 39]` ms. The upscale cost includes the spike's naive single-threaded bilinear loop. "Cold" is process-cold only: OS file cache and OCR model files were already warm from earlier runs; a first-after-boot run was NOT_VERIFIED.

## Commands and exit codes

Run from `spike/ocr/` in Git Bash; full output in `out/run.log`.

| Command | Exit |
|---|---|
| `cargo generate-lockfile` (once, created `Cargo.lock`) | 0 |
| `cargo build --release --locked` (from clean `target/`) | 0 |
| `python make_fixtures.py` | 0 |
| `./target/release/ocr-spike.exe langs` | 0 |
| `python evaluate.py` (×1) | 0 |
| `python evaluate.py 1.5` / `2` / `3` | 0 / 0 / 0 |

`make_fixtures.py` is run without `python -I` because Pillow is in the user site-packages on this machine.

## Machine

Windows 10 Home China 22H2, 10.0.19045.6466 · Intel Core i5-10200H @ 2.40 GHz · 15.8 GB RAM · Rust 1.98.1 · Python 3.13.15 · display language zh-CN · installed OCR languages: zh-Hans-CN only.

## Dependency, licence and size footprint

- One direct dependency: `windows = "=0.62.2"` with features `Foundation`, `Foundation_Collections`, `Globalization`, `Graphics_Imaging`, `Media_Ocr`, `Storage_Streams`, `Win32_System_WinRT` (the last for `RoInitialize`; `Foundation_Collections` for the language and line vectors).
- `Cargo.lock` (sha256 `c64e9172…ed9546`): 15 external crates — runtime: `windows`, `windows-core`, `-collections`, `-future`, `-numerics`, `-link`, `-result`, `-strings`, `-threading`; build-time proc macros only: `windows-implement`, `windows-interface`, `proc-macro2`, `quote`, `syn`, `unicode-ident`. Licences: all `MIT OR Apache-2.0`, except `unicode-ident` 1.0.27 (build-time only): `(MIT OR Apache-2.0) AND Unicode-3.0`. The OCR engine itself is part of Windows; nothing is bundled or downloaded.
- The product already locks `windows-link 0.2.1` / `windows-sys 0.61.2`; adding `windows 0.62.2` would add the crates above to the product lock.
- Exe size: **217,600 bytes** (release, `opt-level = "s"`, thin LTO, `codegen-units = 1`).

## Limits (honest)

- **NOT_VERIFIED**: Windows 11; machines without the zh-Hans OCR language (e.g. English-only Windows — HYPOTHESIS: Chinese OCR unavailable there until the user installs the language pack); en-US engine accuracy; ClearType/subpixel-rendered real UI text (fixtures use grayscale AA); dark themes and coloured backgrounds beyond the one chat shot; first-after-boot latency; multi-monitor/DPI; real screenshots (deliberately not used).
- Only one machine; 12 synthetic images per set. The gate sample is small (984 Chinese chars); a ±1 % shift is ~10 characters.
- The engine reorders lines on mixed-alignment layouts; the product should not assume reading order matches visual order.
- `RoInitialize` was called explicitly; behaviour inside the Slint app's existing apartment state is NOT_VERIFIED.

## Files

`Cargo.toml`, `Cargo.lock`, `src/main.rs` (spike exe), `make_fixtures.py`, `evaluate.py`, `fixtures/*.png` + `truth.json` (`.bgra` regenerable, git-ignored), `out/` (measurement output, git-ignored — the numbers above are the record; rerun `evaluate.py` to regenerate), `.gitignore`. Standalone `[workspace]`; not in the root workspace.
