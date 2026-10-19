> 本文件是独立子代理审查记录的本地路径脱敏副本；原始文件哈希见 RESULTS.json。

# Independent native batch review — 2026-10-17

Reviewer: separate `oracle_review` sub-agent. Product source was read-only. The only authored review material is this file; test caches, synthetic traces and the standalone probe are generated files under ignored `target/`.

## Conclusion and scope

**NO BLOCKING FINDINGS in the reviewed bounded local implementation and evidence batch.** The previously reproduced IDAT hidden-payload bypass and quadratic source-chunk parser stall are resolved. The subsequent harness source-identity and build-directory findings are also resolved for this batch.

This is an independent code and machine-evidence review, **not** complete G01–G10 acceptance, a security audit, a release decision or human usability/accessibility signoff. Physical pointer/DPI and visual rendering acceptance remain NOT_VERIFIED. The main baseline commit is `94f7f419aee91faf6f846349d40a897d0d72d7d0`; this review binds the actual working-tree bytes listed below, not a claim that all reviewed changes are already in that commit.

Final complete native report reviewed: `target/p03-c-evidence/native-20261017T111145813275Z/report.json`.

- Build identity: `337688fa76b157752017a0dabedb3b56d969cda15f9fb74b89bff755db9b866c`.
- Native EXE SHA256: `c81cb0f1e5b9f6b12a40c497a319f58ff64bfa9b8753cd6c101019c8d4ce0553`.
- Compiler reported by the bound build: `rustc 1.98.1 (48a229cea 2026-09-01)`.

## Commands independently executed and results

The Cargo commands used explicit `RUSTC=$LOCAL_USER/.rustup/toolchains/stable-x86_64-pc-windows-msvc/bin/rustc.exe` and `RUSTDOC=$LOCAL_USER/.rustup/toolchains/stable-x86_64-pc-windows-msvc/bin/rustdoc.exe`.

1. `$LOCAL_USER/.rustup/toolchains/stable-x86_64-pc-windows-msvc/bin/cargo.exe test --manifest-path poc/native-egui/Cargo.toml --locked` — **14 passed, 0 failed**, final version, 5.79 seconds of test execution. Includes Windows Ctrl+command event modifiers, text focus, Escape cancellation, eight handle Pointer sequences, preserved press origin, physical-size mesh, source-pixel texture upload, review-to-edit retention and trace import blocking.
2. `$LOCAL_USER/.rustup/toolchains/stable-x86_64-pc-windows-msvc/bin/cargo.exe test -p shareready-editor --locked` — **18 unit tests and 2 integration tests passed** in the earlier review pass. Final `crates/shareready-editor/src/lib.rs` and its geometry/viewport source hashes remain identical. The new draw-preview test rejects stale Copy completion and clears reviewed output/delivery state without changing committed history.
3. `$LOCAL_USER/.cache/codex-runtimes/codex-primary-runtime/dependencies/python/python.exe -B -m unittest discover -s tests/native_oracle -p test_png_oracle.py -v` — **19 passed, 0 failed**, repeated independently, 12.502 seconds. The final oracle and test hashes are unchanged from this execution.
4. Standalone generated Rust probe `target/p03-c-evidence/review-pointer-probe-20261017T111307803532Z/probe.rs`, importing the current production `trace.rs` by absolute module path, was compiled with the fixed rustc, existing locked native dependency rlibs and their existing Windows import-library directories, then executed — **compile exit 0, probe exit 0**. Its JSONL was independently parsed and asserted to preserve `move → primary down → move → primary up → gone` order, pressed states, dimensions `[2400,1600]`, ppp `2`, time `1.25`, and source pointer `[720,400]`. The copied source equals frozen T04. This verifies trace serialization with synthetic events; it is not physical pointer input evidence. An initial direct-rustc probe compile failed to find the existing Windows import libraries; adding the same existing dependency native search directories fixed the probe environment. No dependency was installed and no product code was changed.
5. A read-only Python validation imported the standalone PNG oracle and rechecked the complete final native report: **all source hashes, recomputed build ID, copied EXE hash and all eight stdout/stderr hashes match**; all four bound command exit codes are zero. Both exported sources match the known frozen T04 SHA256 and exact bytes. Both PNGs independently pass again using the frozen repository source. Trace JSONL exactly equals the report trace, the review ticket/document/revision/file/regions correspond, and the reviewed frame matches the sidecar regions. These assertions all passed.

The full native harness command itself was run by the implementation agent, not rerun by this reviewer. Its actual files and results were independently read, hashed and recomputed as described in item 5; conclusions are not based solely on an agent-reported PASS.

## Evidence findings and fixes

- **Resolved P1 — hidden output payload:** the original checker accepted arbitrary trailing IDAT bytes, a second zlib stream and extra decompressed data. The current checker bounds inflation, requires a complete zlib stream, rejects unused/unconsumed data, requires exact scanline length and validates filter bytes. Regression tests reject all three reproduced bypasses. Additional independent probes in the earlier pass accepted valid split IDAT and rejected hidden data in a second IDAT, short scanlines, invalid filter and missing zlib checksum.
- **Resolved P2 — source parsing stall:** the original repeated list search caused measured quadratic runtime before IDAT. `seen_data` removes that repeated scan, and the 40,000 ancillary-chunk bounded-time regression passes.
- **Resolved harness identity gap:** both EXE-emitted sources must now equal frozen T04, and both independent oracle calls use the frozen repository path directly. The final artifacts also independently match frozen SHA256 `b50f6070f4704ef7753ba58493b8e629b51507723ac4c4c8ccff9b629461f83d`.
- **Resolved inherited target-directory ambiguity:** the build command now explicitly specifies `--target-dir` matching the path from which the EXE is copied. The reviewed execution had no `CARGO_BUILD_TARGET`, `CARGO_TARGET_DIR` or `RUSTFLAGS` override in the reviewer shell. This report does not claim arbitrary cross-target configurations are supported.
- **Synthetic privacy boundary:** trace import is blocked before file reads, startup source is bundled T04, and the checked trace source/output artifacts are synthetic. The oracle checks exact full-image RGBA composition, dimensions, opaque cover, format/chunks, CRC and compressed-stream bounds. No original source metadata or hidden payload was accepted in the reviewed output artifacts.
- **Honest evidence scope:** self-test output says `same_core_consistency=PASS independent_file_oracle=NOT_RUN`; independent oracle results are separately computed by Python. The actual adapter log identifies **Dx12 / DiscreteGpu / NVIDIA GeForce GTX 1650** and a completed window lifecycle. This establishes the bound process/adapter lifecycle, not screenshot-verified visual correctness.

## Remaining limitations and unmeasured acceptance

No unresolved blocking code finding was identified within this batch. Full product and G01–G10 acceptance remain open:

- Both real smoke frames have ppp `1`, null pointer and empty pointer-event lists. Their creation/review is programmatic. They provide no physical pointer or mixed-DPI evidence.
- Synthetic headless Pointer tests exercise the production event handlers, but physical two-monitor pointer mapping, 100%/200% at actual mixed/fractional DPI and live cross-monitor DPI transitions remain NOT_VERIFIED.
- Source-pixel texture-content and mesh tests pass. Actual displayed pixels, GPU screenshot readback, visual tile seams/scroll behavior and hardware resource/performance budgets remain NOT_VERIFIED. Screenshot-tool timeout and UIA's absent canvas representation are unresolved observation limits, not PASS.
- Narrator, keyboard usability observed by a human, accessibility and ordinary human review of unmasked details remain NOT_VERIFIED.
- Clipboard cross-application readback, physical Save dialogs and real OS failure/cancellation paths were not independently exercised here.
- Only synthetic T04 is used by this native file/window evidence run. Other frozen fixtures and large-image acceptance are not certified by this report. JPEG callers must use and verify the frozen normalized PNG anchor; the generic PNG oracle cannot determine a source PNG's historical JPEG provenance by itself.
- The added Pointer trace fields have direct synthetic serialization evidence, but no recorded physical press/release trace in the smoke run. Review-event linkage was independently checked for these artifacts; the harness does not yet automatically assert all of those relational checks.

No Windows setting, secret, permission or product source was changed by this reviewer. No upload, push or release was performed.

## Exact reviewed source SHA256

| File | SHA256 |
|---|---|
| `crates/shareready-core/Cargo.toml` | `2be34408f6bba4a36543832137fef87f928f2ba3ba8fdccd302d2815e0ed8352` |
| `crates/shareready-core/src/lib.rs` | `c9e2bec7a65b6e0e764548123c877e492c61e954d4bf42d5157c85b90cfc5d36` |
| `crates/shareready-editor/Cargo.toml` | `682d459e4d020ed6b0206952319ec26553f731e56b6ee1dafcb60baa04013a07` |
| `crates/shareready-editor/src/geometry.rs` | `ec99f1bf59ba2886d690ceab74e762f067dd48cf4012a71a8d3472da96165a5e` |
| `crates/shareready-editor/src/lib.rs` | `e5399f3afb50ab82e9638b3773d009f4b30a47f18b67d9e08cd1d79bb468f477` |
| `crates/shareready-editor/src/viewport.rs` | `1b0babe65c9cd45209f80fb5550407ce555699e0b688611c2ffba3248b5bbff9` |
| `docs/research/fixtures/T04_EDGES.png` | `b50f6070f4704ef7753ba58493b8e629b51507723ac4c4c8ccff9b629461f83d` |
| `poc/native-egui/Cargo.lock` | `20ab491a04245bee12b2687f7fa85f5a1bddcc4302212203f02c55235619b51e` |
| `poc/native-egui/Cargo.toml` | `4b155b230226332237288ddccc0309c9bc78b95150f2742f0718baf6c5189137` |
| `poc/native-egui/src/app.rs` | `49fe79354fce191da20720357f3bf2a2d91971179066f616b177c70f82378df6` |
| `poc/native-egui/src/display.rs` | `c449a41eefece48a20761ca6f378739d2ab559ff0e07b3043d2732bef39030f8` |
| `poc/native-egui/src/main.rs` | `4810f3fded5f71300c5c37d1847f38e4354aa7f216d413a057abc9ef873cc163` |
| `poc/native-egui/src/trace.rs` | `8cc2b9058579e4282771df2b2d380e21e90e4c0f651643d466666ef8f15e8453` |
| `tests/native_oracle/png_oracle.py` | `57ac3ae5a7f7c6ad0ed7972b05622b339eac0c6d4154f0d08713e14a673e387e` |
| `tests/native_oracle/run_native_checks.py` | `721047aaa8efaf3f026d8bc4280e8596a67c8aa490c0de2b53063c22fe9c9123` |
| `tests/native_oracle/test_png_oracle.py` | `5170834dbb9b2a80b9e22e9e23e51596f9aa50e7dfbe500e89bf31e9a08b9859` |

## Independently checked evidence identities

| Artifact | SHA256 |
|---|---|
| `target/p03-c-evidence/native-20261017T111145813275Z/report.json` | `570ff32a5a8bbd0cd3f029f5b2fa13cc12c30150ca896afe867bb7cf62a1b082` |
| `target/p03-c-evidence/native-20261017T111145813275Z/shareready-native-egui-poc.exe` | `c81cb0f1e5b9f6b12a40c497a319f58ff64bfa9b8753cd6c101019c8d4ce0553` |
| `target/p03-c-evidence/native-20261017T111145813275Z/window-trace/events.jsonl` | `c3ef7c90b9cd45e735d4b5598ea185d25828900d218dcf8e39432cd7887d409d` |
| `target/p03-c-evidence/native-20261017T111145813275Z/window-trace/review-1-r1.png` | `dd420118b7784e0a41ae11739a07089f6da640a51d88435de91d5badf889ce31` |
| `target/p03-c-evidence/native-20261017T111145813275Z/files/output.png` | `d19b1540029ff9578db3a89d11316eae39f6bbed02cb32da9cc5ce5ba69cd7a2` |
| `target/p03-c-evidence/review-pointer-probe-20261017T111307803532Z/probe.rs` | `e61d82757158095a4f6cf4155d108e3a2bfe420fb27e297d0a390741a739d4b0` |
| `target/p03-c-evidence/review-pointer-probe-20261017T111307803532Z/probe.exe` | `260b78142dc282f06b3562cbf03081673d7f0df3b5c1b1940f324597e6b15446` |
| `target/p03-c-evidence/review-pointer-probe-20261017T111307803532Z/trace/events.jsonl` | `156e3cc5677d5dbfb735810906bd4e75788a834f371b0ffb488d8f667ea177be` |

Review completed at `2026-10-17T11:23:14.186604+00:00`. Any subsequent product or harness byte change requires a new identity-scoped review; this file does not certify later changes.
