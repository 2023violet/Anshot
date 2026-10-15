# ShareReady E1 hardening + E2.3 engineering handoff (2026-10-15)

**Status: IMPLEMENTED ON DRAFT BRANCHES / NEW CI OUTCOME NOT YET INDEPENDENTLY OBSERVED / E1e INDEPENDENT REVIEW PENDING.**

This is an implementation and acceptance-test handoff, **not** an E1e security signoff or authorization to merge, deploy or release. Use *only* four SHA256-frozen synthetic fixtures; never upload personal screenshots. Original E1 decision gates, Skill pinned version `1eb9624ee2037e4a2b253ee8120fa274dad1ef21`, six-step plan, and Stage 01 A1 human research state have not been changed.

## Target branches
- E1 security-fix Draft [PR #10](https://github.com/2023violet/ShareReady/pull/10): `spike/e1-rust-wasm-feasibility`, based on research branch.
- E2 workbench Draft [PR #12](https://github.com/2023violet/ShareReady/pull/12): `feature/e2-workbench-integration`, based on E1 branch.
- Neither PR is merged. `main` is unchanged by this work.
- The E1 changes below were written to **both** development branches as identical source blobs to keep the currently unmerged E2 preview protected.

## Implemented fixes and measurable checks

| ID | Code change | Verification requirement |
| --- | --- | --- |
| P0-01 | E1 `e1-spike/web/index.html` and E2 `e2-workbench/app.js` snapshot current PNG Blob, source revision and output identity *before* `navigator.clipboard.write`, recheck after await/rejection, and report **stale** rather than copy success after edits/navigation | E1 real Chromium controlled delayed Promise negative test in `verify_clipboard_and_memory.py`; E2 delayed-copy test in `verify_e2_3_browser.py`; retain original real Clipboard success/denial and safe download fallback |
| P1-01 | Rust unit malformed inputs now use a **nonempty valid mask**, reaching PNG/JPEG decoder rather than returning from zero-mask guard; integration includes valid-magic truncated PNG/JPEG and CRC-correct 6000×6000 PNG header exceeding 24MP limit | `cargo test --manifest-path e1-spike/Cargo.toml --locked --offline` and `--test codec_security`; assert exact header/limit error classes |
| P1-02 | E1 core CI no longer regenerates `Cargo.lock`. `cargo fetch --locked` and Git diff guard enforce the *committed* graph, consistent with native and WASM builds. E1/E2 `Cargo.lock` blob is the same: `4923b847fd0deeb924b33dea6abac9e88a5a7ce7`. | New `e1-core-rust-tests.yml` run on changed head; no silent version mutation or `cargo generate-lockfile` |
| E2.3-01 | Visible keyboard-create control/shortcut **N** makes a 180×64 source-pixel opaque mask centered in the current visible image (clamped); history, selection and Rust output use the standard path | Actual Chromium three viewports, Enter and N creation, source-pixel move/resize, Undo/Redo and decode/reopen final PNG |
| E2.3-02 | Mask/list focus preserved through rerender, semantic `aria-pressed` and descriptions added, live status in review dialog; review focus enters dialog and editor becomes `inert`, back restores focus | Keyboard-only 1440, 1024 and compact 800 layouts; no hidden inspector focus loss; expected focused element inspected in DOM |
| E2.3-03 | New full CI case uses Chrome DevTools **Emulation.setPageScaleFactor** to exercise native Chromium visual-viewport scaling (1.25/1.5/1.2), not only CSS `zoom`, and verifies pixel output | `visualViewport.scale` asserted; output PNG dimensions, actual masked pixels, trusted localhost-only requests |
| E2.3-04 | New full CI case performs a genuine Chromium PNG Clipboard write and **trusted Ctrl+V paste**; compares decoded RGBA hash instead of assuming byte-identical PNG; also performs a deterministic mocked-write race | Actual paste event and matching pixels required for PASS; if headless/policy blocks paste, report FAIL/NOT_VERIFIED, **not** a fabricated success |

## CI to inspect on exact new HEAD

- E1 `.github/workflows/e1-core-rust-tests.yml` Rust and committed-lockfile test.
- E1 `.github/workflows/e1-wasm-real-browser.yml` Chrome and delayed-copy negative case. **Existing workflow restores a historically hardcoded E1 core artifact; do not interpret passing steps as renewed chain-of-custody for the new HEAD without comparing the relevant source/blob hashes and genuine Skill evidence.**
- E2 `.github/workflows/e2-workbench-integration.yml` native Rust tests, rebuild real WASM, `verify_e2_browser.py`, `verify_e2_precision_browser.py` and new `verify_e2_3_browser.py`.
- E2 `.github/workflows/e2-workbench-fast-browser.yml` accelerates feedback but must **not** substitute for exact-source full WASM CI.

The GitHub connector exposed source files and Draft PR metadata, but did not expose *push-triggered* workflow-run discovery for the newest commits. This handoff therefore **does not claim those new CI jobs have passed**. Browser-menu Ctrl+Plus zoom, assistive technology/device testing, and other engines still require separate proof.

## Critical safety and review limits

- The current UI only accepts SHA256-whitelisted T01_CHAT, T02_ORDER, T04_EDGES and T05_JPEG_EXIF synthetic files. Unknown/private file hashes fail before image processing. This is a **development safeguard**, not a privacy certification.
- A successful `Clipboard.write` already in flight can write stale bytes at the OS/browser level. Revision validation stops false *current-output* success UI, but cannot revoke an OS Clipboard write. The user must regenerate and explicitly copy after editing.
- Copying to Clipboard does not prove paste-back. UI wording explicitly distinguishes write success from verified paste, while CI exercises a real paste flow.
- **E1e still has zero submitted independent reviews** as last checked. R01–R10 image/metadata/decoder/permissions/license review has not been signed. Keep Issue #11 OPEN, E1e Skill step TODO, both PRs DRAFT, synthetic-only access, no merge, no deployment, no release or real personal screenshots.
- A technical PASS does not validate Stage 01 A1 human demand or authorize E2.3 production rollout.

## Reviewer actions (separate from implementing agent)

Review precise new commit SHAs on PR #10 and #12, inspect exact CI logs and artifacts, independently challenge source/revision race paths, malformed codec inputs and test oracle independence. Record review identity and R01–R10 verdict with evidence. Do not mark `E1_SPIKE_INDEPENDENT_REVIEW.md` signed or advance the original Skill without an actual independent reviewer.
