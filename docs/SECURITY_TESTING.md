# Security testing — image intake (M4, self-contained part)

Status: **agent work, 2026-10-21**. This is not an external audit or a human review. The
independent security review of all code happens later, after the v0.2.0 features land.
Labels follow AGENTS.md: `MEASURED` = a command ran here with the stated result;
`DOCUMENTED` = stated by a source; `NOT_VERIFIED` = not tested.

## Scope

Untrusted bytes enter Anshot only through `crates/anshot-core`:

| Intake function | Input | Existing limits |
|---|---|---|
| `decode_source` | user PNG/JPEG files, clipboard PNG | `MAX_INPUT_BYTES` 25 MiB, `MAX_PIXELS` 24 MP |
| `dib_to_png` | clipboard `CF_DIB` / `CF_DIBV5` | `MAX_DIB_BYTES` (24 MP × 4 + 124 B), 24/32-bit only |
| `decode_generated_png` | readback of Rust-generated PNG artifacts | `MAX_GENERATED_PNG_BYTES` 128 MiB, expected size, 512 MiB decoder budget |

`anshot_editor::SourceCandidate::decode` is a direct wrapper around `decode_source`
(`crates/anshot-editor/src/lib.rs`, `SourceCandidate::decode`), so it is covered
transitively; it is not called separately because the core crate cannot depend on the
editor. A repository search found no other image decoding in `crates/` or `apps/`.

## What was added

- `tests/security/gen_corpus.py` — writes a hostile corpus (156 files, ~80 MiB) to the
  gitignored `tests/security/corpus/`. Generated locally from Pillow, zlib and struct;
  nothing is downloaded and every image is synthetic.
- `crates/anshot-core/tests/intake_hardening.rs` — no new dependencies:
  - a counting `#[global_allocator]` measures, per intake call, the peak growth of live
    heap bytes and the largest single allocation;
  - a panic hook plus `catch_unwind` around every call records any panic and where it
    happened; a watchdog thread aborts and saves the input if one call runs longer than
    `ANSHOT_FUZZ_TIMEOUT_SECS` (default 120 s), since a hang cannot be caught in-thread;
  - an inline xorshift64 mutator (bit flips, random/boundary bytes, 32-bit boundary
    values in both byte orders, truncation, deletion, insertion, splicing).
- `crates/anshot-core/tests/fixtures/zune-jpeg-*.jpg` — three mutation findings kept as
  regression inputs (≤1 KiB each, mutated from synthetic Pillow gradients).
- `deny.toml` — cargo-deny 0.20.2 policy shared by both Rust workspaces.

Every intake call must satisfy all of:

1. no panic escapes the public function;
2. largest single allocation ≤ `MAX_PIXELS × 8` B (192 MB, a 24 MP RGBA16 decode);
3. peak live-heap growth ≤ 512 MiB (`image::Limits::default().max_alloc`, the budget
   `decode_generated_png` already sets);
4. the result is `Err`, or a valid image: non-zero size, ≤ 24 MP, RGBA buffer of the right
   length. For `dib_to_png` the PNG is ≤ 25 MiB, re-decodes through `decode_source` and
   keeps the header's width × |height|. For `decode_generated_png` the size equals the
   expected size.

The allocator meter is process-wide; the four tests in this binary take a mutex so they
run one at a time.

## Corpus coverage

| Area | Cases |
|---|---|
| PNG bombs / dimensions | 100000², u32::MAX², 2³¹ × 1, 24 MP + 1, zero width; 4898² (inside 24 MP) with a 64-byte IDAT; 1×1 image whose IDAT inflates to 64 MiB; legitimate 4000² all-zero image (16 MP, ~16 KiB) |
| PNG structure | truncated (half, magic only, inside IHDR, no IEND), bad IHDR CRC, bad IDAT CRC, corrupt zlib with a correct CRC, chunk before IHDR, IDAT before IHDR, two IHDRs (second claims 50000²), chunk length 2³¹−1, bad depth/colour/interlace method, unknown critical chunk, trailing garbage, 16 MiB zTXt, 32 MiB iCCP |
| PNG formats | APNG (2 frames), 16-bit RGBA, 16-bit grey, 1-bit grey, palette, palette index out of range, palette without PLTE, oversize tRNS, Adam7 interlaced RGBA / 16-bit / truncated |
| JPEG structure | truncated (half, SOI only, no EOI), corrupt scan, corrupt Huffman counts, corrupt DQT, SOF dimensions 0², 65535², 5000² (over 24 MP), 4000×6000 (exactly 24 MP), zero components, SOF length past EOF, two SOFs, arithmetic SOF9, lossless SOF3, progressive cut after its first scan, with and without a dangling marker |
| JPEG formats | CMYK, greyscale, progressive, progressive truncated, 4:4:4, restart markers, progressive + restart markers, trailing bytes after EOI, fill bytes before EOI |
| EXIF | orientation 0–9, 255, 65535; big-endian; wrong TIFF type; count 2; IFD offset 0xFFFFFFFF; entry count 65535 claimed; 512 and 513 entries; bad TIFF magic; truncated TIFF; one maximal 64 KiB APP1; 64 maximal APP1 segments (~4 MiB); APP1 length < 2; APP1 length past EOF |
| Size limits | 0 bytes; 25 MiB + 1 with PNG and JPEG magic; exactly 25 MiB (valid PNG + trailer); GIF, BMP, WebP magic |
| Clipboard DIB | valid 24-bit and V5 32-bit with alpha; empty, 39 bytes, header only; header size 0/12/39/41/64/107/125/0xFFFFFFFF; width/height i32::MAX, i32::MIN, negative width, zero, 65536², 24 MP + 1 top-down, both i32::MAX; claims 24 MP with 16 pixel bytes; truncated by one byte / one row; bit depth 0/1/4/8/15/16/48/64/65535; planes 0/2/65535; compression RLE8/RLE4/JPEG/PNG/ALPHABITFIELDS/0xFFFFFFFF; BITFIELDS with 24-bit, truncated masks, BGR masks; biClrUsed 0xFFFFFFFF; V5 linked profile, profile offsets; V4 odd alpha mask; V5 colour space 0 |
| Generated readback | valid; with tEXt; APNG; interlaced; RGB; 64 MiB deflate bomb; 4898² with tiny IDAT; truncated |

The Rust test also embeds the critical cases (bombs, chunk before IHDR, bad CRC,
truncations, 0 bytes, > 25 MiB, EXIF orientation abuse, absurd DIB headers, a generated-PNG
deflate bomb), so they run even when the corpus has not been generated.

Policy checks in the corpus test, by file name: bombs, oversize, over-24 MP, empty, wrong
magic, truncated, bad CRC, chunks before IHDR, cut progressive JPEGs and all bad DIB headers
must be **rejected**; well-formed inputs (`valid`, EXIF orientation variants, CMYK, grey,
progressive, 4:4:4, APNG, 16-bit, 1-bit, palette, interlaced, 4000² zeros) must be
**accepted**. Deliberately lenient and not asserted: malformed optional EXIF (ignored →
orientation 1), deflate bombs (decoder stops at the image size), and the 4000×6000 SOF
(exactly at the 24 MP gate; zune-jpeg fills missing scan data).

## Findings and fixes

All three come from the seeded mutation run and live in zune-jpeg 0.4.21, which Anshot
gets through the pinned `image =0.25.8` in both lock files. Upgrading `image`/zune-jpeg would
change the pinned decoder behind the frozen goldens, so it was **not** done; the fixes are
in `crates/anshot-core/src/lib.rs`.

| # | Input | Upstream behaviour | Release impact | Fix |
|---|---|---|---|---|
| F1 | progressive JPEG whose data ends before EOI (`zune-jpeg-progressive-eof-hang.jpg`) | `src/mcu_prog.rs` `'eoi` loop: in non-strict mode `get_marker` keeps returning `ExhaustedData` while the pending marker never changes, so the loop never ends | **Hang, one core at 100% forever** (MEASURED: the call exceeded the 30 s watchdog in a `--release` run) | `decode_source` now walks the JPEG markers strictly (`jpeg_reaches_eoi`) and rejects a stream that ends before EOI, as `invalid image header: …` |
| F2 | baseline JPEG with mutated sampling factors (`zune-jpeg-worker-slice-oob.jpg`) | `src/worker.rs:233` slice index 32 out of range for a 16-entry buffer; a second site, `src/upsampler/scalar.rs:59`, appeared in a 1M run | **Panic in release builds**, unwinding through every caller of `decode_source` | the decoder now runs inside `std::panic::catch_unwind`; a panic becomes `Err("image decode failed: decoder panicked on malformed data")`. The desktop crate does not set `panic = "abort"`, so this also holds in the shipped profile |
| F3 | progressive JPEG (`zune-jpeg-progressive-bits-left.jpg`) | `debug_assert!(self.bits_left >= n)` at `src/bitstream.rs:407` | debug builds only; release uses `saturating_sub` | the stream has no reachable EOI, so F1's marker walk now rejects it before decoding; F2's containment would also catch it |

Product behaviour changes, all fail-closed:

- a JPEG whose marker structure does not reach EOI is now rejected. Before this, a
  baseline JPEG missing only its EOI was accepted with filled pixels. Bytes after EOI are
  still ignored. Well-formed JPEGs (baseline, progressive, CMYK, grey, restart markers,
  EXIF, trailing data) still decode, and so do all five frozen goldens;
- a decoder panic now returns `Err` instead of unwinding.

Error texts keep the existing `invalid image header` / `image decode failed` prefixes that
the frozen `e1-spike/tests/codec_security.rs` asserts.

Regression test: `zune_jpeg_panic_inputs_fail_closed` asserts that all three fixtures return
`Err` without unwinding, that the worker.rs panic is contained (so containment stays tested),
and that the hang fixture returns in under 5 s.

Existing limits held: across all runs below, no allocation came near the bounds (largest
single allocation 96 MB = a 24 MP RGBA8 buffer, from the 4898² and exactly-24 MP cases), no
`Ok` result was outside the limits, and no DIB or generated-PNG input panicked.

Known limitation (not changed): zune-jpeg 0.4.21 rejects a well-formed progressive JPEG that
uses restart intervals (`src_jpg_progressive_restart.jpg`); the user sees a decode error.

## Commands and results (MEASURED, 2026-10-21)

Windows 10 x64, rustc/cargo 1.98.1, Python 3.13.15, Pillow 12.3.0, cargo-deny 0.20.2.
The base commit is in the git history of this file.

```bash
python tests/security/gen_corpus.py
```
→ `wrote 156 files (79.5 MiB)`; exit 0. Prefix tells the test which function to call:
`src_*` 95 → `decode_source`, `dib_*` 53 → `dib_to_png`, `gen_WxH_*` 8 → `decode_generated_png`.

```bash
cargo test --locked -p anshot-core --test intake_hardening -- --nocapture
```
Default run (debug, 20 000 mutations), exit 0, 4 tests pass in ~28 s:

| Test | Calls | Accepted | Peak growth | Largest allocation | Contained panics |
|---|---|---|---|---|---|
| embedded | 36 | 8 | 96 113 160 B | 95 961 616 B | 0 |
| corpus | 156 | 50 | 168 000 000 B | 96 000 000 B | 0 |
| mutations | 20 000 | 2 109 | 25 617 441 B | 9 409 536 B | worker.rs:233 × 1 |
| regressions | 3 | 0 | 5 526 B | 1 488 B | worker.rs:233 × 1 |

Full mutation runs (exit 0 each; watchdog 30 s; no escaped panic, hang or limit breach):

```bash
ANSHOT_FUZZ_ITERS=100000 cargo test --release --locked -p anshot-core --test intake_hardening -- --nocapture
ANSHOT_FUZZ_SEED=7 ANSHOT_FUZZ_ITERS=1000000 cargo test --release --locked -p anshot-core --test intake_hardening seeded -- --nocapture
```

| Build | Seed | Mutations | Accepted | Peak growth | Largest allocation | Contained panics | Time |
|---|---|---|---|---|---|---|---|
| release | default (0x5eed20261021) | 100 000 | 10 871 | 25.6 MB | 9.4 MB | worker.rs:233 × 1 | 0.8 s |
| release | 1 | 100 000 | 10 934 | 12.5 MB | 6.3 MB | 0 | 1.1 s |
| release | 2 | 100 000 | 10 907 | 25.1 MB | 12.5 MB | 0 | 0.9 s |
| release | 3 | 100 000 | 10 880 | 25.6 MB | 9.4 MB | 0 | 0.7 s |
| release | 42 | 100 000 | 11 111 | 40.5 MB | 20.2 MB | 0 | 1.1 s |
| release | 7 | 1 000 000 | 110 856 | 61.1 MB | 34.9 MB | worker.rs:233 × 1, upsampler/scalar.rs:59 × 1 | 9.9 s |
| release | 99 | 5 000 000 | 551 554 | 69.8 MB | 34.9 MB | worker.rs:233 × 6 | 42.9 s |
| debug | default | 100 000 | 10 871 | 25.6 MB | 9.4 MB | worker.rs:233 × 1 | 15.4 s |

Before the fixes, the same harness measured: F2 escaping as a panic at iteration 17 254
(debug, default seed); F1 as a hang at iteration 169 842 of a 1M release run; F3 at
iteration 11 652 (debug). The seed mixing was changed to splitmix64 after those runs (seeds
2 and 3 used to share a stream), so those iteration numbers do not replay today; the fixtures do.

```bash
python tests/golden/verify_v1_baseline.py
```
→ `SR_V2_00_FROZEN_GOLDEN_VERIFIED`, exit 0 (five frozen goldens unchanged).

```bash
cargo test --workspace --locked
```
→ exit 0, all test binaries `ok`.

```bash
cargo test --manifest-path apps/desktop/Cargo.toml --locked
```
→ exit 0, 37 passed (desktop consumes the changed `decode_source`; run against the shared
working tree, which also held another session's uncommitted desktop/render edits).

### Supply chain: cargo-deny

```bash
cargo deny --locked check
cargo deny --locked --manifest-path apps/desktop/Cargo.toml check
```
→ both `advisories ok, bans ok, licenses ok, sources ok`, exit 0 (RustSec DB fetched
2026-10-21).

- **Advisory found: RUSTSEC-2026-0192**, `ttf-parser` 0.25.1 is unmaintained (via
  `ab_glyph` 0.2.32 → `anshot-render`). It is not a vulnerability notice and cargo-deny
  reports no safe upgrade. It is ignored in `deny.toml` with a reason: the parser reads only
  the bundled `assets/fonts/NotoSansCJKsc-Regular.otf` (`include_bytes!`), never a
  user-supplied font. Revisit if a vulnerability advisory names ttf-parser/ab_glyph, or if
  user fonts are ever accepted. Without that ignore, both checks fail on this one item.
- No vulnerability advisories in either lock file.
- Licenses: the allow-list is exactly the set used by both lock files for the Windows x64
  and wasm32 targets, plus `LicenseRef-Slint-Royalty-free-2.0` for Slint (see
  `apps/desktop/THIRD_PARTY_LICENSES.md`). GPL-3.0 is not allowed. The root workspace shows
  `license-not-encountered` warnings because the list is shared with the desktop workspace.
- Bans: wildcards are denied (path dependencies between the private workspace crates are
  allowed). Duplicate versions are warnings only: root `miniz_oxide`; desktop also `hashbrown`,
  `smol_str`, `syn`, `tiny-skia(-path)`, `windows-sys`, `windows-targets`,
  `windows_x86_64_msvc`, `zune-core`, `zune-jpeg` (0.4.21 for `image`, 0.5.15 from the Slint
  graph). Removing them would mean upgrades, so none were made.
- Sources: crates.io only; unknown registries and git sources are denied.

## Not covered (NOT_VERIFIED)

- **No coverage-guided fuzzing.** The mutator is blind (no cargo-fuzz/libFuzzer/AFL; those
  would be new tooling). The corpus and ~6.6M mutations are evidence, not proof of absence.
- **No external or human security audit.** All of this is agent work; the independent review
  is still to come.
- The meter counts Rust heap allocations in this test process. It is not an OS memory
  ceiling, and it does not see stack use or allocations made outside the global allocator.
- The marker walk guards against hangs at EOF. Other decoder loops that never end are only
  covered as far as the mutations reached (watchdog 30 s per call).
- A panic is contained only where unwinding works. A `panic = "abort"` profile, or an abort
  inside a dependency (for example a failed allocation), would still end the process.
- Windows clipboard capture (`crates/anshot-platform-windows`), drag-and-drop paths and
  the desktop UI were not exercised. Only the core functions they hand bytes to were.
- Upstream reporting of F1–F3 to zune-jpeg, and moving to a fixed zune-jpeg/image, are left
  as decisions for the release owner (an upgrade must be re-checked against the frozen goldens).
- Decoder output correctness on unusual formats (CMYK colour, 16-bit down-conversion) was not
  compared against a reference decoder; only limits and absence of crashes were checked.

## How to rerun

```bash
python tests/security/gen_corpus.py
cargo test --locked -p anshot-core --test intake_hardening -- --nocapture
ANSHOT_FUZZ_ITERS=100000 cargo test --release --locked -p anshot-core --test intake_hardening -- --nocapture
```

Environment variables: `ANSHOT_FUZZ_ITERS` (default 20 000), `ANSHOT_FUZZ_SEED`
(u64, default 0x5eed20261021), `ANSHOT_FUZZ_TIMEOUT_SECS` (default 120). A failing or
hanging input is written to `target/tmp/intake-failure-*.bin` and its path is printed.
