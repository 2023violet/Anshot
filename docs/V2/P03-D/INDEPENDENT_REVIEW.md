> 独立子代理审查记录的本地路径脱敏副本；原始记录哈希见 RESULTS.json。

# Independent P03-D / M05 review — 2026-10-17

Reviewer: separate `oracle_review` sub-agent, read-only product review. Only this review material and generated test/probe artifacts were written. No product source, V1 path, lock, frozen fixture, Windows setting, secret or permission was edited by this reviewer.

## Conclusion

**NO BLOCKING FINDINGS within the P03-D M05 bounded local batch.** The reviewed implementation satisfies the scoped machine-verifiable criteria for bounded file reading, prepared candidates, edit-preserving confirmation, redo protection, single pending action, close cancellation, gesture cleanup and modal action blocking. This is not full G01–G10, human usability, accessibility, security-audit or release acceptance.

The reviewed code is `poc/native-egui/src/input.rs`, the M05 portions of `app.rs`, and the single `mod input;` registration in `main.rs`. The exact source hashes below bind this conclusion. The Git baseline in the final report remains `94f7f419aee91faf6f846349d40a897d0d72d7d0`; reviewed working-tree bytes need not yet be present in that commit.

Decision and acceptance were read from `target/workflows/p03-d/decision-card.md` and `plan.json`. Plan progress is implementation-agent evidence; it is not substituted for this reviewer's executions below.

## Independently executed verification

Environment: `RUSTC=$LOCAL_USER/.rustup/toolchains/stable-x86_64-pc-windows-msvc/bin/rustc.exe`, `RUSTDOC=$LOCAL_USER/.rustup/toolchains/stable-x86_64-pc-windows-msvc/bin/rustdoc.exe`.

1. `$LOCAL_USER/.rustup/toolchains/stable-x86_64-pc-windows-msvc/bin/cargo.exe test --manifest-path poc/native-egui/Cargo.toml --locked` — exit **0**, **25 passed**, 0 failed, 6.58 seconds of test execution. This is an independent execution of the current implementation, not only a reported genuine-workflow result.
2. `$LOCAL_USER/.rustup/toolchains/stable-x86_64-pc-windows-msvc/bin/cargo.exe test --workspace --locked` — exit **0**, **34 passed** across Core unit tests (5), codec/security integration (9), Editor unit tests (18) and Editor integration (2). Other targets/doc-tests contain zero tests. Cargo reports the existing `shareready-e1` library/binary PDB filename collision warning; tests pass. No unrelated target refactor was performed.
3. Generated standalone Rust probe `target/p03-d-evidence/modal-probe-20261017T135338862190Z/probe.rs` — compile **exit 0**, execution **exit 0**, stdout `MODAL_POINTER_BUTTONS_AND_BLOCKED_ACTIONS_PASS`, empty stderr. The probe embeds the current `app.rs` unchanged apart from resolving its synthetic fixture include to an absolute path and adding a test entry point; it imports the current `display.rs`, `trace.rs` and `input.rs` by source path. Compilation uses the fixed rustc and the already locked native dependency rlibs/native Windows import libraries. Its retained source and executable hashes are below.

The supplemental probe exercises actual egui Modal widgets through PointerMoved/primary-down/primary-up events, locating the rendered button text in the output shapes. It verifies both **Keep editing** and **Discard edits** while the containing root UI is disabled. Keep preserves source, stamp, PNG and a pending Copy ticket; Discard replaces with the validated T01 candidate exactly once and rejects that stale ticket. It also invokes `copy`, `save`, `start_review` and `new_mask` while confirmation is pending and proves those calls preserve the document and pending Copy bytes, without invoking OS clipboard/file-dialog delivery.

## Acceptance review and evidence

- **Read bound:** the file reader checks metadata and then reads through `take(Core MAX_INPUT_BYTES + 1)`. An oversized stream stops at its one detection byte; exact-limit/short-read and partial-read-then-error cases pass. Native entry tests reject missing, malformed and oversized files while retaining source, masks, revision, history, reviewed PNG and operation bytes.
- **Validated candidate first:** reading and Core-backed display decoding occur before creating a replacement decision. `PreparedSource` retains the validated bytes/texture. The current editor is replaced only after an explicit Discard result, except for a genuinely clean document. Failure and Keep leave the prior document and live ticket intact.
- **Protection includes redo and previews:** `has_edits` includes Undo, Redo, GUI drag and Editor gesture state. A mask-free document with redo still prompts. Entering replacement/close confirmation cancels only the uncommitted gesture/preview, retains committed masks/history, and clears the active gesture so a swallowed release cannot leave the Editor stuck. Existing output invalidation at gesture start remains the earlier contract; this batch does not restore stale output on gesture cancellation.
- **One pending action:** additional import attempts are rejected before reads while a confirmation exists. Multiple dropped paths are rejected before preparing any candidate. Repeated close requests retain the existing pending action and send CancelClose rather than bypassing it.
- **Close and modal:** real egui RawInput Close events produce CancelClose when protection is needed. Keep retains committed history; explicit Discard emits Close with the one-use `close_confirmed` path. Pending confirmation blocks keyboard editing, canvas pointer handling, source reads, review and OS delivery, while the main UI and Inspector are disabled. The additional Pointer probe confirms the Modal buttons remain operable.
- **Preservation:** comparing the prior P03-C `RESULTS.json` source map against current bytes found only `app.rs` and `main.rs` changed among the prior files, with new `input.rs`. Removing exactly the new `mod input;` line from current `main.rs` reproduces its P03-C SHA256. Core, Editor, display, trace, native manifests/lock, oracle/harness and T04 remain byte-identical to the P03-C record. `git diff --name-only` over `desktop-windows`, `web-app`, `e2-workbench`, `e1-spike`, Core, both lock paths and frozen fixtures returned no changes.

## Independently rechecked final EXE evidence

Final current-batch report: `target/p03-c-evidence/native-20261017T135217353702Z/report.json`. Its directory is the reused harness default; this is a **new M05 artifact**, not the previous P03-C EXE.

- Build ID: `7abe58dff940b983c5bede944a73a32e2e3bb96ffb7c378bfd400ae22959e787`.
- EXE SHA256: `1c310bf769a82b58e8099d89ceb018bfe5830e233493ca131fc59342434530cc`.
- Report SHA256: `7cb54a4868398c06a197bf837e423b6e27a6f4af66b69b5f165359d23777d289`.

A read-only Python validation independently recomputed all current source hashes and build ID, verified copied EXE and all eight stdout/stderr hashes, and checked all four bound command exit codes are zero. Both EXE-emitted sources equal frozen T04. Both output PNGs passed the independent file oracle again using the frozen repository source; report fields equal the freshly computed results. The trace header build matches; the review record filename/regions/document/revision correspond to the sidecar and reviewed frame. Bound logs identify `Dx12 / DiscreteGpu / NVIDIA GeForce GTX 1650` and the native lifecycle marker. All assertions passed.

The implementation agent ran the full build/window harness; this reviewer independently examined its actual files and recomputed the assertions. The reviewer did not rerun that complete window harness. The smoke process programmatically sets `close_confirmed` for its synthetic lifecycle shutdown. That shutdown is **not user confirmation evidence**, and this review does not count it as an observed user's close/keep/discard interaction.

## Remaining limits and meaningful untested paths

No remaining blocking code defect was reproduced in this scope. The following limitations stay open:

- A physical Windows title-bar close/Alt+F4 → Keep/Discard sequence and the native file picker Cancel path were not manually observed. Their app state/event behavior is tested with synthetic egui input, including actual Modal Pointer clicks; native OS/human behavior is not promoted to PASS.
- Full-window main-toolbar/Inspector blocking is supported by disabled egui UIs plus direct pending-action guards. The additional probe checks Modal interaction and guarded operations, but does not exhaustively click every disabled widget in a real HWND.
- Reader tests exercise exact/short/error streams and oversized files. A live file growing between metadata and read was not raced empirically; the read-stage `take` bound is independently exercised by the stream test.
- G01–G10 physical multi-monitor/mixed-DPI pointer mapping, screenshot-verified displayed pixels, visual QA, Narrator/human accessibility and usability remain NOT_VERIFIED. Headless events and DX12 lifecycle are not substitutes.
- The M05 close design asks about discarding editable history even after PNG export, as required by D05. No editable project persistence was added. M04 image paste and M06 atomic Save/failure handling remain outside this batch and are not claimed complete.
- OS clipboard cross-application readback and real Save failures/cancellation remain outside this review. PNG/metadata/byte validation and synthetic fixture results are limited to the bound artifacts.

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
| `poc/native-egui/src/app.rs` | `9d206efdee8b475b8167ced6489d2dac4995fe47a47683c50bee2e7fb860c0f1` |
| `poc/native-egui/src/display.rs` | `c449a41eefece48a20761ca6f378739d2ab559ff0e07b3043d2732bef39030f8` |
| `poc/native-egui/src/input.rs` | `cf505a1ccc39086feb1e7ac0b9038e95c9037708c08922616b5dd3037d5ba4fa` |
| `poc/native-egui/src/main.rs` | `e9b4c88c6d375a8c8cb3e9438f0c80f21b6498d3827f10228f4ef2b949b8ea65` |
| `poc/native-egui/src/trace.rs` | `8cc2b9058579e4282771df2b2d380e21e90e4c0f651643d466666ef8f15e8453` |
| `tests/native_oracle/png_oracle.py` | `57ac3ae5a7f7c6ad0ed7972b05622b339eac0c6d4154f0d08713e14a673e387e` |
| `tests/native_oracle/run_native_checks.py` | `721047aaa8efaf3f026d8bc4280e8596a67c8aa490c0de2b53063c22fe9c9123` |
| `tests/native_oracle/test_png_oracle.py` | `5170834dbb9b2a80b9e22e9e23e51596f9aa50e7dfbe500e89bf31e9a08b9859` |

## Evidence and planning identities

| Artifact | SHA256 |
|---|---|
| `target/p03-c-evidence/native-20261017T135217353702Z/report.json` | `7cb54a4868398c06a197bf837e423b6e27a6f4af66b69b5f165359d23777d289` |
| `target/p03-c-evidence/native-20261017T135217353702Z/shareready-native-egui-poc.exe` | `1c310bf769a82b58e8099d89ceb018bfe5830e233493ca131fc59342434530cc` |
| `target/p03-c-evidence/native-20261017T135217353702Z/window-trace/events.jsonl` | `a0d4525263ad8586ee5c70f958c4964d7e5a70c00588057c57fea93c5babd048` |
| `target/p03-d-evidence/modal-probe-20261017T135338862190Z/probe.rs` | `105cfc9ca12c08d078ffb89c29ca8bd82a4c4d9446835d3f727b150a938783d6` |
| `target/p03-d-evidence/modal-probe-20261017T135338862190Z/probe.exe` | `5612153516f1475760931403bec2142bab04f8bf064261439d99bb7ebdec4015` |
| `target/p03-d-evidence/modal-probe-20261017T135338862190Z/stdout.txt` | `3d9e3b42ed796657b43f0c220221c877a0167e27b6c6b93ec26c32ca9092b7da` |
| `target/p03-d-evidence/modal-probe-20261017T135338862190Z/stderr.txt` | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| `docs/V2/P03-C/RESULTS.json` | `ab635c978ecffe5ad031f1b121d8439746f2dd27ffa390f545631152e2eb3991` |
| `target/workflows/p03-d/decision-card.md` | `4517d313d869cb313c6f8cb4aacc67c8c3ac926c7ab183d1fdc3fbd26b93ca88` |

Completed at `2026-10-17T13:56:00.568648+00:00`. A subsequent product-source or harness byte change requires a new identity-scoped review. Later genuine-workflow progress updates to the plan are documentation state updates, not changes to the code identities certified here.
