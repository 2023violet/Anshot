# SR-V2-00 — V1.3 to Rust Native acceptance matrix

V1 implementation is supported by source and historical machine runs at their exact commit; manual Windows E2E remains limited. All new V2-native implementations are NOT_IMPLEMENTED at Stage 00.

| Feature | Current implementation | Future Rust owner | Acceptance gate | Phase |
|---|---|---|---|---|
| F01 PNG/JPEG choose, Explorer drag, paste | JS + C# | desktop/platform/intake | Three genuine Windows sources, cancellation/invalid recovery | 04/05 |
| F02 25MiB/24MP, EXIF, decoder | Rust e1-spike | core/decode | Bad headers/limits fail closed; T05 orientation | 01 |
| F03 masks, move, 8-way resize, 1px | JS + Rust | editor/commands + desktop/canvas | Pixel-precise opaque output, edge/negative rectangles | 02/05 |
| F04 canvas ↔ mask inspector | JS | editor/selection + desktop/inspector | Accurate selection/focus/list synchronization | 02/05 |
| F05 undo/redo, clear/cancel | JS | editor/history | Undo coalescing, redo fork clearing, revision rules | 02 |
| F06 fit/100%, zoom/pan, DPI | JS/WebView2 | editor/viewport + desktop/canvas | Four corners match source pixels across real DPI | 03/05 |
| F07 review actual encoded PNG | Rust WASM + JS | core/export + editor/snapshot | Independent decode of final bytes; no visual-only masking | 01/06 |
| F08 save newly reencoded PNG | Rust + C# dialog | core + desktop/platform/save | Cancel/read-only/replace safety; source not overwritten | 04/06 |
| F09 real image clipboard + fallback | JS+C# | desktop/platform/clipboard | Cross-app pasted pixels match current artifact | 04/06 |
| F10 recovery, stale async state | JS | editor/session/operation | DocumentId + Revision + OperationId prevents false success | 02/06 |
| F11 local-only, no account/upload | CSP+WebView2 | desktop/platform + security audit | Network/process/log observation, honest limitation | 04/07 |
| F12 no overclaimed redaction | UI text | desktop/ui | Explicit manual-scope wording, usable keyboard path | 05 |

## Required negative/precision acceptance tests

B01 five synthetic golden full-image RGBA cases; B02 zero-effective/invalid masks must refuse; B03 EXIF 1–8 and hostile EXIF; B04 oversized/truncated input; B05 DPI + T04 four corner geometry; B06 8 handles and keyboard Undo/Redo; B07 redo branch and invalid old export; B08 rapid image replacement and late old result; B09 OS clipboard already accepted outdated bytes (not automatically reversible); B10 clipboard permission/busy; B11 saving cancelled/read-only/replacement/source collision; B12 PNG only IHDR/IDAT/IEND, CRC and no source metadata; B13 retries after errors; B14 24MP resource profiling; B15 no unapproved network/image-bearing logs; B16 keyboard/Narrator/IME.

No new V2 function is marked PASS by writing this matrix. No V1 branch, history, source, test or package is removed in V2-00. Main-only development and current-user-visible behavior must continue until Stage V2-08 fully accepted.

Every V2 implementation stage must record exact HEAD, relevant test command and result, exported image SHA and actual Windows observations, with NOT_RUN for absent evidence.