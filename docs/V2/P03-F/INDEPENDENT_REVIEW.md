> 独立审查本地路径脱敏副本；原始记录哈希见 RESULTS.json。

# P03-F independent review

Date: 2026-10-17. Reviewer: separate `save_review` agent, not the implementer. Scope: bounded PNG/DIB intake, owned-memory Windows boundary, Ctrl+V input bridge, shared GUI candidate handling, guarded Copy callback, necessary CI/lock/golden-verifier changes, and the final exact native artifact. Authority: `docs/governance/AUTONOMOUS_LOCAL_DEVELOPMENT.md`. Product source, locks and fixtures were read-only during review. Reviewer programs, logs and synthetic outputs were written only under ignored `target/`; tests did not read or modify the actual system clipboard.

Verdict: **NO REMAINING BLOCKING FINDINGS IN THE FINAL SCOPED LOCAL CHANGE**. One concrete PNG/JPEG format-confusion defect was found and corrected before this verdict. Evidence supports the bounded code and synthetic/local test scope below. It does not establish real system Paste, physical message routing, cross-application alpha or human acceptance, and is not a full product/release/security-audit approval.

## Final identity

- Branch/HEAD: `main`, `94f7f419aee91faf6f846349d40a897d0d72d7d0`, plus preserved uncommitted work. HEAD alone does not identify this change.
- Final 32-input build identity: `d9c51e49b35533252ff6176a15e84f2ed10740e4a4370f31198bc42396f0a372`.
- Final EXE SHA256: `eabf15ee3c83092ffa8152520019e8802cc11339e40696de3e21631c69379790`.
- Final harness: `target/p03-c-evidence/native-20261017T144102054005Z/report.json`, SHA256 `339dc932f87aeacb7c3632bde9937c38c81df86c3d2f7b5d67cc2d45cc357b29`.
- Independently generated `target/p03-f-evidence/reviewer-verified.json`, SHA256 `b2e76e22c103b2ebc79baab18283ee0a424a1e665d7c16d165f66c737b4c0008`, contains all 32 rehashed source/manifest/lock/fixture inputs, extra reviewed file identities, artifact/oracle results, frozen V1 results and reviewer-log hashes.
- Earlier build `89a3a5d1b7dcaef88f94543aac4981271f1c2da703823aad4810978516b958a6` is pre-fix evidence and is not the final accepted identity.

Key source SHA256 values:

| File | SHA256 |
| --- | --- |
| `crates/shareready-core/src/clipboard.rs` | `b1426568bb5f7127ed4a4182945a3050156dd902d6c2b5cffb6314c398ed0a59` |
| `crates/shareready-core/tests/clipboard_dib.rs` | `941fa0e99cffbd8638f28d0c86e15dc2d793233bd655aaecebd3fc3cbdadc118` |
| `crates/shareready-platform-windows/src/clipboard.rs` | `9f58b50536799d752a8b2d87dd191a1b6188a30b60653df2acb43c221e20799f` |
| `poc/native-egui/input-bridge/src/windows.rs` | `028e7874d2d6719b9c049ded5a5038ecec2edeff73435c9d8aee07ef9967c199` |
| `poc/native-egui/src/app.rs` | `e73624dd4c66516d5c349c04979f503db227307b5270fe32b87007133c9d0f7b` |
| `.github/workflows/v2-03-native-poc.yml` | `df0d43e238c464d8f883bd9a5f9e7a919da167b56d3b88934f3f3c44f3ea3f55` |
| `tests/golden/verify_v1_baseline.py` | `52b4d76c1c81c746475dd68eca130ec3b38cc07735c7dd20b7700135d56eefe9` |

## Finding corrected during review

The first GUI `ClipboardImageFormat::Png` branch forwarded bytes directly to the general PNG/JPEG decoder. Actual JPEG bytes tagged as registered PNG therefore became a replacement candidate, contrary to criterion 3's invalid-PNG rejection rule. The reviewer reported this source-level counterexample. The implementer added a frozen T05 JPEG regression; its actual pre-fix failure is recorded in `png-tag-red.txt` (exit 101, unexpected pending candidate). The final branch requires PNG signature and <=25 MiB before general decode. The final test checks unchanged document/revision/source/PNG/history and no candidate. The reviewer independently reran the complete native workspace after the fix: this test and all other tests passed. No success criterion was relaxed; the old artifact was replaced by a newly verified final identity.

## Actual independent executions

Rust commands used `$LOCAL_USER/.rustup/toolchains/stable-x86_64-pc-windows-msvc/bin`, with explicit sibling `RUSTC` and `RUSTDOC`. Python used `$LOCAL_USER/.cache/codex-runtimes/codex-primary-runtime/dependencies/python/python.exe`.

| Reviewer execution | Observed result | Evidence |
| --- | --- | --- |
| `cargo test -p shareready-core -p shareready-platform-windows --locked` | exit 0; Core 5+7 tests, platform 12 tests | `reviewer-core-platform.log` |
| `cargo test --manifest-path poc/native-egui/Cargo.toml --workspace --locked` | exit 0; final GUI/controller 41 tests, bridge 4 tests; doc tests exit 0 | `reviewer-native.log` |
| `cargo test --workspace --locked` | exit 0; 53 workspace tests; doc tests exit 0 | `reviewer-workspace.log` |
| direct `rustc --edition 2021` of `reviewer_dib_probe.rs`, linked to the actual root Core rlib, then execution | exit 0; 288 independent direction/stride/RGBA cases and 10,000 bounded malformed-header no-panic cases | `reviewer_dib_probe.rs`, `reviewer-dib-probe.log` |
| `python tests/golden/verify_v1_baseline.py --report target/p03-f-evidence/reviewer-v1-golden.json` with TEMP/TMP inside the evidence directory | exit 0; all five frozen V1 cases verified | `reviewer-v1-golden.log`, `reviewer-v1-golden.json` |
| `python target/p03-f-evidence/reviewer_verify.py` | exit 0; all final source/build/EXE/log/PNG/trace/lock/CI/verifier identities checked; exact final EXE independently executed again | `reviewer-verified.json`, `reviewer-selftest.stdout.txt` |
| `git diff --check` | exit 0; existing Windows LF/CRLF notices only | observed command output |

The 19 independent PNG-oracle regression tests in the final harness were not rerun separately by the reviewer in this batch; their actual exit-0 logs/count and all log hashes were revalidated from disk, the oracle code was inspected, and the reviewer independently executed its full-image/structure checks on both final PNGs and a newly generated self-test output. These distinctions are intentional.

Reviewer log SHA256 values:

- core/platform: `d181e191bb126eb81898878f4e1d1d08935097822304ae26043b7dc7b2170da2`
- native/bridge: `d206815fad7edbfe496fc7b58ffb72cc68cd02b36069bcdadaca0f54060a204f`
- workspace: `412d6d768d4362fe11f913775edb2cfc459be9bd2d023dfe57cd87f3f73fc408`
- independent DIB probe: `73dff0c843c42e9fc00ac64f6044f6907c9ebbe8aa267c9720057036e8e1cf0d`
- independent V1 goldens: `8f43d35162fd7d7ed8988d87c875ac99dbfa18294ffaf887f9d4d7b079b2867e`

## Criteria and boundary review

1. **DIB parsing and new PNG.** Fixed header length is checked before indexed reads; only 40/108/124 headers, 24/32-bit supported combinations, one plane, no palette and supported sRGB/system-sRGB layouts are accepted. Positive/negative heights select bottom-up/top-down row order. DWORD padding and BGR-to-RGBA mapping are correct. Checked signed height, positive width, 24 MP product bound, computed row extent and total input bound precede image allocation/access. The 40-byte bitfields mask block must be present; only standard RGB masks and supported explicit alpha are interpreted. BI_RGB's unused high byte becomes opaque; explicit bitfields alpha is preserved. Unsupported masks, profiles, color spaces, truncated headers/rows and huge dimensions fail closed. Reencoding starts from a fresh Rust RGBA image; normalized PNG must satisfy 25 MiB before candidate admission. Review tests cover all pixels rather than only dimensions. The independent probe additionally varied narrow widths (1-9), row padding, both directions, 24/32-bit layout and alpha over 288 cases. This is a supported subset, not every valid DIB format.
2. **Windows HGLOBAL ownership.** `GlobalSize` is checked against the selected format's limit, nonzero and isize bounds before `GlobalLock`, reserve or slice/copy. The pointer is used only while locked; the guard unlocks on success and error before `CloseClipboard`. System-owned handles are never freed. Tests allocate their own movable, zero-initialized HGLOBAL, repeat bounded copies, reject null/oversize limits and release only their own block. Actual clipboard selection prefers registered PNG, then CF_DIBV5, then CF_DIB; a bad preferred object returns error rather than falling through. No text/file-list fallback or retry loop was added. The clipboard guard covers every return after successful OpenClipboard. Win32 pointer/handle calls stay in the platform crate, with explicit live-allocation contracts and synchronous borrowing.
3. **Why the bridge is needed and what it proves.** Locked egui-winit 0.36.2 source was inspected: on Ctrl+V keydown it calls its text clipboard getter and returns before queuing an egui Key event. Locked winit 0.30.13 source was inspected: its documented message hook is called with a live MSG before TranslateMessage/DispatchMessage, and returning true suppresses that dispatch. The new hook checks WM_KEYDOWN, V, Control, absence of Alt, bound HWND, foreground HWND and text-edit focus; first press queues one atomic pending flag and repeat messages are swallowed without another request. The MSG reference is borrowed only for the callback and does not escape. Binding is protected by a mutex, pending is atomic, and poisoned/unbound cases leave messages alone. Four tests call the acceptance function with synthetic messages and real egui text-focus state; they do not exercise the actual physical hook. GUI consumption rechecks text focus and confirmation state, while `paste_with` blocks trace/confirmation before invoking the reader. Button action remains explicit. No global keyboard hook, listener thread or dependency-version upgrade was introduced.
4. **Candidate and Copy recovery.** File and clipboard bytes share display/decode preparation before replacement. Existing undo/redo/active gesture state follows the existing discard decision; cancellation preserves committed history/review. Invalid/no-image/busy inputs fail while retaining the source, revision and current PNG, and a later valid candidate can retry. Corrected PNG tagging is checked before generic decode. Copy callbacks require exact current Copy ticket and no confirmation before the supplied receiver runs. Tests prove stale/wrong-kind/blocked callbacks never invoke the receiver, failure preserves the PNG and offers Save new PNG, and retry receives the exact current Rust artifact. Real copy still uses the existing arboard writer and success wording says OS acceptance with cross-app readback unverified.
5. **Safety lint and dependencies.** GUI main and Editor keep existing `forbid(unsafe_code)`; the new safe Core clipboard module also forbids unsafe. The Core crate as a whole did not previously have that attribute, so no stronger claim is made. Unsafe raw-memory/message operations are isolated to the platform/input bridge. Root Cargo.lock is byte-identical to the pre-batch snapshot. Independent record-by-record native lock comparison permits only the new input-bridge path package plus its native dependency edge; every existing package version/checksum/dependency record is unchanged. Bridge egui/winit/raw-window-handle/windows-sys versions were already locked. New Windows features only expose needed APIs.

## Final artifact and necessary integration changes

The reviewer rehashed all 32 harness inputs, including new Core/platform/bridge code and all five frozen fixtures, recomputed build identity, verified the exact copied EXE, every harness stdout/stderr hash, trace identity and both PNG oracles, and checked that sources stayed identical after the independent EXE run. The additional self-test writes a new `target/p03-f-evidence/reviewer-selftest-1792248147507496000` directory and emits the real handle-bound save marker. It does not touch the actual clipboard.

The saved T04 PNG is SHA256 `d19b1540029ff9578db3a89d11316eae39f6bbed02cb32da9cc5ce5ba69cd7a2`, RGBA SHA256 `b983aa4b8039d92d57d76370af66579b3538f9ce3348ab635545b44e6db96865`. The automatic actual-window review PNG is SHA256 `dd420118b7784e0a41ae11739a07089f6da640a51d88435de91d5badf889ce31`, RGBA SHA256 `6be3f5e6fcdd58affaf8ad199ecef518aa35fd18f0c5fd3133cfaa860c77fed4`. Both contain only IHDR/IDAT/IEND. These artifacts exercise the review/save image path, not the real clipboard reader or writer.

CI was compared exactly against the expected baseline delta: P03-E's three trigger paths and platform test remain; the native test now uses `--workspace`, which is necessary to execute bridge unit tests in addition to the root native package. Permissions, upload/action pins and renderer policy are unchanged. Remote CI is **NOT_RUN**; no push or dispatch was performed. No complete YAML parser/runner claim is made.

The golden verifier change was checked against exact expected source: add one platform suffix variable and use it for the CLI and preview executable paths; there are no comparator/contract/acceptance changes. Frozen `v1_3_baseline.json` and fixture manifest match HEAD. The reviewer independently ran all five current V1 outputs; the JPEG case retains the existing frozen contract's Rust-normalized source plus independent Pillow mask compositor (Pillow's independent JPEG decode differs and is not substituted). Tested V1 CLI SHA256: `f2961592b8df1689ffdca6a64ec839af326d28439a8711177c396d0593a92eea`; preview SHA256: `b5c1fca63522d24ad9153a7dfa7319f0d073fdd5a6580f3eb04a6992a72450cc`.

## Unverified scope retained

- The reviewer never read/wrote the live system clipboard. Actual OpenClipboard/GetClipboardData format selection, busy behavior, delayed rendering, OS format synthesis and cross-process ownership were source-reviewed but not end-to-end tested. Owned HGLOBAL tests do not replace this evidence.
- GetClipboardData may invoke OS/producer allocation or blocking before a handle exists. The application's GlobalSize-before-copy bound does not cap that prior work. DIB RGBA conversion and PNG encoding allocate bounded intermediate data up to the 24 MP policy; the 25 MiB PNG limit is applied after encoding, not a 25 MiB peak-memory guarantee. No memory/performance benchmark is claimed.
- Synthetic bridge tests and source inspection are not proof of physical key routing through a real foreground HWND, IME/layouts, focus transitions, multiple viewports or every modified key combination. Text focus is checked at capture and consumption; real Windows interaction remains open.
- Real image Paste from Paint/screenshot tooling, Copy into Paint/another application, alpha across applications, clipboard-history/cloud-sync interaction and actual external format interoperability remain **NOT_RUN/NOT_VERIFIED**. No privacy/settings changes were made.
- The native DX12 lifecycle trace and saved/review PNGs are automatic machine evidence. There was no reviewer screenshot/visual inspection, physical pointer/mixed DPI/Narrator test or ordinary human task. No G06/full product closure follows from this review.
- Unsupported DIB compression, palettes, color profiles and nonstandard masks intentionally fail; broader format support is not claimed. No fuzzing completeness, malicious OS/driver guarantee, power-failure guarantee or external security audit is claimed by the 10,000 bounded cases.
- User approval is the recorded local-development policy; individual workflow confirmations are agent confirmations. Workflow closure/docs/archive are the parent agent's responsibility. This review does not publish, release, delete user files or self-confirm workflow gates.

## Primary references consulted

Microsoft [BITMAPV5HEADER](https://learn.microsoft.com/en-us/windows/win32/api/wingdi/ns-wingdi-bitmapv5header) supports orientation, channel masks, BI_RGB high-byte and color-profile interpretation. Microsoft [GetClipboardData](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getclipboarddata) describes untrusted data, retained system ownership, lock lifetime and implicit conversion; [GlobalSize](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-globalsize) describes size/error and allocation rounding. Locked local winit and egui-winit sources were inspected for actual hook/clipboard ordering. Documentation/source reasoning and measured tests remain separate evidence layers.
