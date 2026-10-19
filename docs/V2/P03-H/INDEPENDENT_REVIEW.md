> 独立审查的本地路径脱敏副本；原始哈希见 RESULTS.json。

# P03-H independent review

**Verdict: NO_REMAINING_BLOCKING_FINDINGS_FOR_LOCAL_P03_H_BATCH.** The final Chinese workbench, font and synthetic GPU evidence are supported within the measured local scope. This does not close full M07/G08, G01-G10, accessibility, real clipboard integration, human usability or product/release acceptance.

Reviewer: separate `save_review` agent, not the implementer. Product source, locks and fixtures were read-only. Reviewer scripts/logs and synthetic output were written under ignored target only. No reviewer GUI, system clipboard, P03-G capability probe, elevation or publishing operation was performed. The only native EXE execution used the headless self-test branch.

## Final identity

- Branch/HEAD: main, `94f7f419aee91faf6f846349d40a897d0d72d7d0`, plus preserved uncommitted work. HEAD alone does not identify this implementation.
- Final 40-input build: `84a7f011ac420decfcc07f3a0c822abe9c2fa9dabec94070d7d7d44e18a93860`.
- Final EXE SHA256: `4eeab7fa9584291b1004fa1631e2fafc427bdb8f66072de91b57ba1302095a5b`.
- Harness: `target/p03-c-evidence/native-20261018T014537145596Z/report.json`, SHA256 `ef61869d967e8376501e03371a3ffa8d6605428017ef37b138cafd377a6d64c6`.
- Independent data: `target/p03-h-evidence/reviewer-verified.json`, SHA256 **`3adcabef86984ffc3a0fc394fd5a41ee9d48eb44e8f2517ff4dafa90d0be8807`**. It records all 40 source/lock/font/fixture hashes, harness streams, EXE, eight images, pixel oracles, tests and observations.

Earlier source-changing candidates are not the final identity. Six final images matched already viewed candidate images byte-for-byte; final error/discard images were viewed again after the recovery-text edit.

## Independent execution and checks

| Check | Actual result |
| --- | --- |
| Final native `cargo test --manifest-path poc/native-egui/Cargo.toml --workspace --locked` | exit 0; 47 native + 4 bridge tests, doc tests pass; `reviewer-native-final.log` |
| Root `cargo test --workspace --locked` | exit 0; 53 tests, doc tests pass; `reviewer-workspace.log` |
| Exact copied EXE `--self-test --self-test-output` to a new reviewer directory | exit 0; real handle-bound save marker and independent full-image/PNG verification; `reviewer-selftest.log` |
| `reviewer_font_verify.py` | exit 0; fresh public official pinned font/license downloads byte-equal to local assets; OTF metadata checked |
| `reviewer_font_cmap.py` | exit 0; all 204 non-ASCII characters in final app/main/fonts sources mapped by the pinned font cmap |
| `reviewer_verify.py` against final report | exit 0; 40 inputs/build/EXE/all harness streams/three PNG outputs/eight screenshots/locks/fonts/golden records and fresh self-test checked |
| Local image inspection and `git diff --check` | all eight final image identities observed; diff check exit 0, Windows LF/CRLF notices only |

Tests used the fixed MSVC toolchain with explicit RUSTC/RUSTDOC. The root test ran before the final native wording edit; its root workspace/dependency inputs were unchanged in the final map. Native tests were rerun after that edit.

The 19 oracle regression tests and five V1 goldens were actual implementer reruns whose identities/logs/records were checked, not separate reviewer reruns this batch. The reviewer independently recomputed the final saved-output, DX12-review and visual-review full-image oracles and a fresh final-EXE self-test output. The five golden records match the frozen expected records; JPEG retains its frozen Rust-normalized-source/compositor contract.

## Findings corrected

- **Screenshot/state association:** initial callbacks checked only stage/viewport and used arrival-time editor metadata. Final requests occur after layout, save a document/display/status snapshot and reject changed snapshots. Synthetic mode clears interactive input while retaining screenshot events; normal mode is unaffected. Stage/viewport and changed-state tests pass.
- **Chinese recovery:** final shared `image_rejected` gives a Chinese PNG/JPEG retry action and retained-image/history statement before diagnostic details. Final error/discard images show it. Missing-mask review now explains the next action; image-filter text is Chinese.
- **Layout/contrast:** actual first images showed the floating inspector obscuring toolbar/corners. Final fixed sidebar and vertical scrolling remove that overlap at both sizes. Theme text replaces low-contrast light green; clear color is opaque panel fill and follows locked eframe's gamma-space contract.

Intentional initial RED failures for font/theme/non-fixture admission remain historical evidence. Glyph tests now compare actual rasterized atlas UVs against U+10FFFF replacement for both font families, including `×`; local epaint inspection confirms that `has_glyph` face comparison alone can misclassify valid glyphs sharing the replacement face.

## Font and image observations

Official pinned Noto CJK commit `f8d157532fbfaeda587e826d4cd5b21a49186f7c` font and OFL were independently downloaded and compared without sending private data. Font size is 16,437,364 bytes, SHA256 `2c76254f6fc379fddfce0a7e84fb5385bb135d3e399294f6eeb6680d0365b74b`. OTF tables confirm Noto Sans CJK SC Regular/400 and original Adobe/OFL metadata. No system font was copied or installed. The asset is fallback for both families, preserving existing Latin fonts. Cmap/rasterized samples do not prove every Unicode filename or IME.

| GPU image | Size | Actual reviewer observation |
| --- | --- | --- |
| edit | 1120x760 | Chinese/× visible, selected mask handles, sidebar clear of toolbar and source corners |
| multi | 1120x760 | Two masks and selected handles; coordinates/count match trace |
| review | 1120x760 | Two opaque masks, no canvas editing handles; readable actual-PNG review status |
| back-edit | 1120x760 | Handles return; wording accurately preserves reviewed PNG until another edit |
| error | 1120x760 | Chinese retry/preservation text visible before English diagnostic; edits retained |
| discard | 1120x760 | Readable centered Chinese modal, synthetic filename, keep/discard choices |
| small | 700x480 | Toolbar wraps to two rows; image/inspector remain separate and controls visible |
| dark | 700x480 | Opaque dark background, visible Chinese controls, source image colors retained |

All eight hashes/dimensions/states were checked against final trace/report. All are RGBA8 PNGs containing only IHDR/IDAT/IEND with opaque screenshot alpha. Document 1 advances from revision 1 to 2; later states preserve revision 2 and the reviewed artifact, with review_view true only for review. Trace mode deliberately disables import/Copy/Save controls: these pictures do not certify ordinary-mode enabled delivery-control appearance or system delivery.

## Privacy, preservation and limits

Normal launch creates no trace/screenshots. Explicit visual mode rejects mixed/duplicate flags. Trace checks exact T04 before creating evidence; imports and actual copy/save are blocked in trace mode. The audited path cannot start from a private image. Snapshot/tag checks prevent stale callbacks from certifying newer state. Pixel/export/ticket rules and existing cancel/history/focus/stale-result regressions remain green. Existing safety lint is preserved. Both locks match the already reviewed G state; H adds no dependency upgrade.

G remains stopped and was not rerun. Baseline records WGC timeouts and shell-only UIA; the reviewer did not repeat desktop operations. GPU readback establishes client-area rendering, not OS decorations/compositing, physical pointer/keyboard/focus, mixed DPI, dual-screen, timing or human experience. Real clipboard/Paint/second-application alpha, Explorer/dialog tasks, IME, Narrator/accessibility tree, full keyboard tasks, 24MP/performance/memory, offline packaging, complete licensing/security and ordinary user acceptance remain open. The raw font size is not a passed package/startup/memory budget.

The README draft was checked and accurately retains these limits. Final RESULTS, copied screenshots and manifest/checksums require a separate `reviewer-docs-verified.json` after binding. This original report must not change after its hash is bound. No remote CI/push/release is certified. Workflow tokens are agent confirmations under the recorded user policy, not human product acceptance.

Primary references: [official fixed font](https://github.com/notofonts/noto-cjk/blob/f8d157532fbfaeda587e826d4cd5b21a49186f7c/Sans/OTF/SimplifiedChinese/NotoSansCJKsc-Regular.otf), [matching OFL](https://github.com/notofonts/noto-cjk/blob/f8d157532fbfaeda587e826d4cd5b21a49186f7c/Sans/LICENSE), and inspected local locked eframe/epaint 0.36.2 contracts. Source checks, headless tests, GPU images and unperformed human/system tasks remain distinct.
