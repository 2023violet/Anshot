# ShareReady V2 — Rust Native development baseline

Current phase: SR-V2-00 / V1.3 freeze and native GUI POC contract. No Rust V2 GUI code is implemented.

- [V1_BASELINE.md](V1_BASELINE.md): exact product SHA, CI and Windows artifact evidence, rollback boundary, golden fixtures.
- [MIGRATION_MATRIX.md](MIGRATION_MATRIX.md): F01–F12 and regression scenarios, future Rust ownership.
- [GUI_POC_CONTRACT.md](GUI_POC_CONTRACT.md): egui/eframe versus iced verification and STOP criteria.
- [WINDOWS_ACCEPTANCE.md](WINDOWS_ACCEPTANCE.md): manual Windows operations that no CI smoke can certify.
- [Golden verifier](../../tests/golden/verify_v1_baseline.py): compile V1 Rust and compare full-frame pixels for all five synthetic fixtures.

Development policy: main-only; no branching, force push, release or deletion of V1. The existing C#/WebView2/JS implementation remains operational until V2-08 acceptance. Rust Native GUI candidate is not approved by mere framework selection. No human test, security certification or public release is asserted here.