# SR-V2-00 — V1.3 exact baseline and rollback evidence

**Status**: baseline identity and rollback source recorded. Golden lock and exact-HEAD CI status must be obtained from the actual Actions run. This document is not release approval or a security certification.

## 1. Immutable historical baseline

- Repo: 2023violet/ShareReady; main is the sole remote development branch.
- Main at beginning of V2-00: 0e6eb1c258c3564f90ca663aa01de5c9a935c147. This commit deleted the completed branch cleanup workflow; it was not the HEAD of the earlier product build.
- Previously successful integrated V1.3 source: 9f49a09c3fe1d1977d9a024ced548990c16dbe75, Actions Run #38998364601, Linux Rust/Chrome and Windows WebView2 desktop jobs SUCCESS. Evidence: https://github.com/2023violet/ShareReady/actions/runs/38998364601 .
- Historical Windows Artifact #12088216693, shareready-v1303-windows-native-x64. Windows CI SHA256 of *inner product ZIP* ShareReady-V1.3-03-Windows-Native-x64.zip: 40a167c622a1f011c2ef0c38ad9102866af49d642397d48ce40048436d75c6a7.
- SHA256 of GitHub Actions *outer artifact ZIP*: 072e48050131015333436b1c3c1827bd2fa0ad34de8108ba8a14aae40167217d. Outer artifact digest is NOT the product ZIP checksum.
- Artifact retention expiry reported 2026-11-16T03:24:17Z. No evidence of indefinite off-GitHub binary archival: LONG_TERM_BINARY_ARCHIVE_NOT_VERIFIED.
- Historical Chrome evidence artifact #12088535226 and Rust/WASM web assets artifact #12087874030 expire 2026-10-31.
- No later CI can be claimed as having tested a different commit unless its own HEAD and jobs are verified.

## 2. Application ownership (no product deletions at this stage)

e1-spike/ contains the actual Rust image decoder, EXIF normalization, opaque pixel overwrite, PNG codec and CLI/WASM. web-app/ contains the currently delivered JavaScript canvas, zoom/pan, eight handles, undo/redo, current-output checks, copy/save state. desktop-windows/ is the current C# WinForms+WebView2 OS host for file dialogs, clipboard and window. e2-workbench/ is a historical browser experiment, not the current shipped V1.3 editor.

Legacy web-app/Start-ShareReady.ps1 localhost launcher has outstanding old Host/Header/EOF security evidence; it is NOT an approved V2 run path and must be independently fixed or retired before distribution.

## 3. Repeatable five-image golden contract

Fixed synthetic fixture inputs T01_CHAT.png, T02_ORDER.png, T03_SUPPORT.png, T04_EDGES.png, T05_JPEG_EXIF.jpg are tied to docs/research/fixtures/manifest.json SHA256 + dimensions + masks. tests/golden/v1_3_baseline.json freezes those values plus decoded RGBA hashes. The verifier compiles and runs the actual Rust V1 executable, uses Pillow normalization for PNG and the V1 Rust EXIF-normalized preview for JPEG, independently overwrites half-open rectangles using (17,24,39,255), and compares EVERY pixel in the decoded Rust PNG output. It also checks PNG chunks/CRC/no trailing bytes/no copied metadata. PNG byte SHA is evidence; decoded RGBA SHA is the portable behavioral oracle.

Rebuild and verify (Pillow==12.3.0):
- cargo build --manifest-path e1-spike/Cargo.toml --locked --release
- python tests/golden/verify_v1_baseline.py
Only the initial machine capture may pass --capture; capture evidence is NOT a frozen contract. Strict mode must match pinned expected_pixel_sha256 for all five fixtures and be executed at the final HEAD.

## 4. Risk inheritance and explicit NOT_VERIFIED items

Carry forward: zero-effective-mask fail-closed; malformed PNG/JPEG and extreme dimensions; EXIF 1–8, unknown metadata and trailing payload; 1px/negative/edge rectangles; async copy/save revision mismatch; OS clipboard can retain a previously written image; legacy HTTP host security; clipboard/save error recovery.

Still NOT_VERIFIED: real Windows Explorer drag, native dialog human interactions, genuine cross-app PNG paste, multi-monitor 100/125/150/200% DPI, Windows Narrator, 24MP peak RAM/GPU, full adversarial fuzz and independent security review. There is no evidence of full device usability certification.

## 5. Rollback rule

Do not remove e1-spike/, web-app/, desktop-windows/, synthetic fixtures or essential security tests before Stage V2-08. Preserve source commits and rollback instructions. If V2 fails, use a verified preserved V1 ZIP or reproduce from the pinned source; do not rewrite main history or claim the expiring Artifact persists indefinitely. No Tag, Release, deployment or deletion is authorized by V2-00.

Next phase: SR-V2-01 — separate Rust Core + Editor while retaining V1's working binary and these exact regression tests.
## JPEG decoder truth caveat

First capture at cabfb006 recorded T01–T04 PASS but T05 failed when comparing Pillow JPEG source decoding directly to Rust image-rs pixels. This is not equivalent to proving product corruption. Rework b4a82a5 introduced a test-only Rust normalized JPEG preview; Pillow still independently applies masks, decodes the exported PNG and checks every output pixel. T05 Pillow original RGBA SHA256 88e40c5acfd8565469eb7f8dc7cc289d17180ea1ca8edbc5463c3e4152f7543e versus Rust normalized preview SHA256 819b7a2b2240b8bbf9af9bab9b40ca68fc10e7ec7b9f8f1996e4a66792eb90bd (not equal). The frozen T05 output RGBA SHA is d2d6586997927592c77ee7c96529d58901f529deb1a649804a55e25a6e0d49b5. V2 decoder migration must treat this as a compatibility decision, not silently waive it.
