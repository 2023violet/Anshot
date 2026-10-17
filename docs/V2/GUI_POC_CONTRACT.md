# SR-V2-00 — Rust Native GUI feasibility and choice contract

Candidate A: egui + eframe; fallback candidate B: iced. The planned product is Rust-first, native-first, Windows x64 first, no JavaScript/C#/WebView2/.NET *application runtime* after V2 acceptance.

**Important**: Current phase only defines the experiment. Neither toolkit is proven or chosen, and no GUI test is marked PASS.

| POC ID | Real experiment | PASS evidence to capture |
|---|---|---|
| G01 | T04 source-pixel masks at fit/100%/200% | Windows pointer trace + decoded full-image golden |
| G02 | Eight resize handles, 1px, keyboard, undo/redo | Native interaction sequence + Rust mask and pixel truth |
| G03 | 100/125/150/200% DPI and mixed-DPI multi-monitor | Actual GUI/mouse source-pixel output parity |
| G04 | 24MP display and GPU texture limits | GPU bounds, fit/downsample/tile strategy, RSS, frame time |
| G05 | Windows real chooser/cancel and Explorer drag | Native OS observation, not DOM-simulated events |
| G06 | Clipboard paste in + PNG copy to Paint and second app | Target-app copied image readback decoded RGBA SHA |
| G07 | Save as, cancel, readonly, overwrite, source-path conflict | Actual file hash and accurate status, no stale success |
| G08 | Chinese IME, complete keyboard task and Windows Narrator | Human accessibility steps, names, focus and error feedback |
| G09 | Replace document / undo while copy or export pending | Revision + document + operation identity, old result invalid |
| G10 | Offline package, dependency and license boundary | Windows binary bill of materials, network trace, lock hash |

Record Windows build, device CPU/8GB RAM reference, GPU/driver, monitor DPI, rustc, Cargo.lock hash, full Git SHA, actual operator/time, samples, steps, measurements, observable errors, screenshots and hashes. Performance thresholds must be frozen *after* measurement; a theoretical benchmark is not evidence.

Decision gate: GO_EGUI only if every P0 experiment is passed on a runnable build with no safety regression. A first P0 failure gets one bounded remediation/retest; then SWITCH_ICED for equivalent tests. Failure of both leads STOP_NATIVE_REWRITE and preservation of the working V1.3 path. Rich Rust percentage does not override security, Windows input, clipboard, DPI or accessibility quality.

No OCR/AI, database, cloud, batch, login, telemetry, excessive dependencies or repository redesign is authorized by this POC.