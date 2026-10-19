# ShareReady V2 — Rust Native development baseline

> **Frozen historical reference (2026-10-19).** The product is being redefined under the 01–07 design workflow in [`ui-design/`](../../ui-design/). The V2-03 egui POC and the P03 evidence below are kept as technical reference; Core/Editor/platform crates continue as the Rust trust core.

Current phase: SR-V2-03 local native development. Historical Windows CI demonstrated an egui wgpu/DX12 software-adapter window; it does not certify later uncommitted changes. Physical Windows G01–G10 acceptance remains open. The V1.3 JS/WebView2 app remains the user-facing implementation; no GUI candidate is finally frozen.

- [P03-K bounded background source intake](P03-K/README.md): latest completed local machine evidence, independently accepted with limits. The sealed candidate and bounded worker preserve old editing on failed/cancelled intake; current fixed source/review windows, full PNG/controlled RGBA, ordinary/feature suites, performance and frozen goldens ran. Physical/human/security/package/release acceptance remains open; J is a historical artifact.
- [P03-J bounded background review](P03-J/README.md): historical locally measured and independently reviewed batch, accepted with limits. A single worker performs Rust PNG encoding and CPU display preparation; current-key/draw authorization, cancellation, repeated Close and exact full-output/window regression passed. Physical acceptance, synchronous import/delivery and full product gates remain open.
- [P03-I 24MP measurements and generated-PNG readback](P03-I/README.md): historical regressed baseline, eight real-window cases, bounded large artifacts, exact GPU/output pixels and WASM/frozen-golden regression; subsequent J changes require their own verification. Budgets, physical acceptance and full product gates remain open.
- [P03-H Chinese workbench and GPU visual evidence](P03-H/README.md): historical regressed snapshot, licensed CJK font, theme/background fixes, fixed inspector and eight actual synthetic app frames; I rechecks these states after its code changes.
- [P03-G isolated clipboard capability stop](P03-G/README.md): real OS183/OS5 creation failures; no elevation or clipboard-content fallback. H later regresses the test-only configuration; the isolation capability remains unavailable.
- [P03-F bounded clipboard intake and copy recovery](P03-F/README.md): historical regressed snapshot, PNG/DIB normalization, Windows memory bounds and focused Ctrl+V bridge; actual clipboard/cross-app and full product gates remain open.
- [P03-E Windows new-file save](P03-E/README.md): historical M06-A handle-bound rollback, no overwrite and failure/cancel/retry evidence.
- [P03-D bounded import and edit preservation](P03-D/README.md): historical M05 file limits, prepared replacement candidates and discard/close protection.

- [P03-C local implementation and evidence](P03-C/README.md): M01/M02/M03, physical zoom geometry, original-pixel visible tiles, keyboard/gesture fixes and independent PNG oracle; complete G01–G10/human acceptance remains open.
- [Continuous local development authorization](../governance/AUTONOMOUS_LOCAL_DEVELOPMENT.md): user-approved reversible V2 implementation/testing and honest agent-confirmed gates; external/destructive actions remain separately gated.

- [Live B0 execution gates](P03-B/README.md): Gate A/B confirmed; fixed EXE selftest/DX12 on GTX 1650 and 33 local Rust tests passed; automated native keyboard/UIA review/history state recorded; pointer/mixed-DPI/Narrator remain unverified.
- [P03 A review package](P03-A/README.md): pinned EXE/source identity, G01-G10 gap matrix, physical Windows protocol, minimal changes and Gate A/B candidates; no test or gate approval.
- [Product modification and optimization plan](PRODUCT_MODIFICATION_PLAN_2026-10-17.md): proposed scope, Rust native roadmap, task packages, acceptance criteria and execution gates; not implementation or release approval.
- [V2_03_RENDERER_VALIDATION.md](V2_03_RENDERER_VALIDATION.md): source-matched glow/DX12/auto Windows renderer matrix, precise CI evidence and human validation limits.
- [V2_03_NATIVE_GUI_POC.md](V2_03_NATIVE_GUI_POC.md): true Rust GUI source, Windows POC evidence and outstanding physical acceptance.
- [V2_02_EDITOR_DOMAIN.md](V2_02_EDITOR_DOMAIN.md): source bytes, selection, eight handles, gestures, history, versioned Rust PNG artifacts and OS ticket boundaries.
- [V2_01_WORKSPACE_MIGRATION.md](V2_01_WORKSPACE_MIGRATION.md): Rust Core/Editor integration, code ownership and verification.
- [V1_BASELINE.md](V1_BASELINE.md): exact product SHA, CI and Windows artifact evidence, rollback boundary, golden fixtures.
- [MIGRATION_MATRIX.md](MIGRATION_MATRIX.md): F01–F12 and regression scenarios, future Rust ownership.
- [GUI_POC_CONTRACT.md](GUI_POC_CONTRACT.md): egui/eframe versus iced verification and STOP criteria.
- [WINDOWS_ACCEPTANCE.md](WINDOWS_ACCEPTANCE.md): manual Windows operations that no CI smoke can certify.
- [Golden verifier](../../tests/golden/verify_v1_baseline.py): compile V1 Rust and compare full-frame pixels for all five synthetic fixtures.

Development policy: main-only; no branching, force push, release or deletion of V1. The existing C#/WebView2/JS implementation remains operational until V2-08 acceptance. Rust Native GUI candidate is not approved by mere framework selection. No human test, security certification or public release is asserted here.
