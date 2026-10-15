# SR-ARCH-002 Rust-first technical blueprint candidate

**2026-10-15 — PROPOSED ONLY / NO RUST CODE AUTHORIZED**

## Boundary

- Rust core: validate and decode PNG/JPEG, normalize orientation, resolve half-open image-pixel rectangles, replace opaque target pixels, encode brand-new metadata-clean PNG, verify properties.
- UI host (Leptos CSR + WASM first SPIKE candidate, not frozen): import, editing canvas and region list, history/undo, zoom/pan, output-review and error state.
- Browser adapter: local file/paste/drop, ClipboardItem(image/png) with genuine success/deny feedback, final-byte download, no remote image transport.
- Conditional fallback: Tauri 2 if Web clipboard/performance evidence prevents a good task path.

## Safety invariants

INV01: coordinates normalized and clipped to image bounds in half-open source-pixel space.

INV02: mask union pixels are genuinely replaced by fixed opaque RGB and alpha=255; no CSS-only hiding, blur or metadata residual.

INV03: brand-new PNG output contains no original image payload or inherited EXIF/GPS/unknown chunks.

INV04: current editor revision must equal current export revision for review/copy/download; invalidate stale outputs after edits.

INV05: output-review image and clipboard/download all derive from same verified encoded PNG bytes.

INV06: malformed/oversized/failed rendering and clipboard permissions are fail-closed; do not silently fall back to the original.

INV07: actual privacy claims require network/CSP/dependency and clipboard paste-back audits, not narrative descriptions.

## Sequenced engineering plan; NONE of the code tasks are authorized yet

| Wave | Planned purpose | Evidence |
|---|---|---|
| E0 | Feature P0/P1 review + controlled Skill replan | approval token and plan diff after separate user acceptance |
| E1 | Tiny Rust/WASM geometry+codec+clipboard SPIKE | fixture roundtrip, browser capability/performance report; needs explicit coding approval |
| E2 | Actual import, canvas, region edit/history | integration tests and UI walkthrough |
| E3 | Real opaque pixel masks, PNG and review | T01–T05 region / metadata / stale-output tests |
| E4 | Clipboard/download and failure paths | real paste-back and permissions negative tests |
| E5 | Platform & security QA | independent network, browser, accessibility and legal license reviews |
| E6 | Merge, deployment and release | separate authorization, NOT IN SCOPE |

The original approved Gate B research plan still has S05a BLOCKED_BY_RECRUITMENT and S06 depending on S05 real participant evidence. UI approval alone cannot override this constraint or authorize full engineering. PR #1 remains Draft, main unchanged, no product Rust implementation or release.

Details and future AC01–AC12: [feature contract](../product/SHAREREADY_V1_FUNCTION_CONTRACT_CANDIDATE.md).
