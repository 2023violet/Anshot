# AGENTS.md — ShareReady

## Project truth and scope

- **Product 0.1.1 implemented (2026-10-19; 0.1.1 polish 2026-10-20).** ShareReady is a Rust-trusted Windows desktop tool for *capture → annotate → redact → share*, built in `apps/desktop` (Slint 1.18.1) over `crates/shareready-{core,render,editor,platform-windows}`. Stages 01–07 are documented in `ui-design/`; Gates 01–05 were user-confirmed, Gates 06–07 agent-confirmed under the user's 2026-10-19 delegation (ACCEPT_WITH_LIMITS). It is unsigned and unreleased; human, multi-monitor/DPI and Narrator tests remain NOT_VERIFIED.
- The V1.3 editor (`web-app/`, `desktop-windows/`, `e1-spike/`) and the V2-03 egui prototype (`poc/native-egui/`) are **frozen historical references**: keep them building, add no new features, do not delete them without express user confirmation. Snapshot: local tag `pre-redesign-2026-10-19`.
- Positioning: learning / portfolio project. Do not present release, security audit, or human usability testing as complete when they are not.
- The name **ShareReady is temporary**; an existing product uses a similar name. Do not assert brand clearance.

## Rust trust boundary (non-negotiable)

- Image decoding, pixel transformation, redaction, annotation rasterization into the output, metadata stripping, export, save and clipboard delivery happen **only in Rust**. The UI layer is also Rust native; the GUI framework (egui / iced / Slint) is chosen at Gate 03 by a native feasibility spike.
- Reuse and extend `crates/shareready-core`, `crates/shareready-editor`, `crates/shareready-platform-windows`. New capabilities get new Rust modules; never break the five frozen goldens (`python tests/golden/verify_v1_baseline.py`).
- Privacy baseline: local-only processing, opaque pixel replacement for private areas, new-image export without source metadata, synthetic test fixtures, honest limitation wording. None are verified until tests run.
- No nonessential dependencies; keep lock files locked (`--locked`).

## Workflow: AI-driven product UI design, stages 01–07

| Stage | Deliverable in `ui-design/` | Gate |
|---|---|---|
| 01 Product definition | `01_PRD.md` | users, value, MVP scope / non-goals, ≥1 end-to-end task |
| 02 UX architecture | `02_UX_ARCHITECTURE.md` + low-fi wireframes | flows cover enter / act / finish / fail / recover |
| 03 Visual directions | `03_VISUAL_DIRECTIONS.md` + 2–3 directions + native framework spike | direction and GUI framework chosen with reasons |
| 04 Design system | `04_DESIGN.md` (tokens also as Rust theme constants) | baseline frozen, versioned |
| 05 High fidelity | `05_HIGH_FIDELITY/` | all key states, min/common window sizes |
| 06 Implementation | `06_IMPLEMENTATION.md`, code in `apps/desktop/` | runs, tests pass, real screenshots vs design |
| 07 Validation | `07_UI_QA_REPORT.md` | key tasks done by real operation, blockers fixed and retested |

- Each Gate is **confirmed by the user** and the confirmation is recorded at the end of the stage document. The agent never self-confirms a Gate.
- Problems found later go back to the affected stage and are revised there.
- HTML/React mockups are visual references only; native feasibility must be proven in Rust.
- This workflow replaces the earlier deciding-and-executing Gate A/B token process and the per-batch evidence packages (kept as history under `docs/`).

## Evidence (milestone level)

- One short evidence record per Gate: full SHA, commands and exit codes, results, known limits.
- Independent review once each at Gates 03, 06 and 07, run by a separate sub-agent and labeled as an agent review, never as human review.
- Always distinguish `DOCUMENTED`, `MEASURED`, `HYPOTHESIS`, `NOT_VERIFIED`. Machine results never stand in for human tests.

## Change safety

- **Main-only**: `main` is the sole development branch; no long-lived branches or routine Draft PRs.
- Within this roadmap the agent may plan, implement, test and commit reversible local work without per-step confirmation (policy approved 2026-10-17, see `docs/governance/AUTONOMOUS_LOCAL_DEVELOPMENT.md`; scope updated 2026-10-19 to this redefinition).
- Requires express user confirmation first: deleting or moving user files or the V1/POC paths, rewriting history, force-pushing, pushing, tagging remotely, releasing or publishing, external messaging, changing secrets/auth/permissions/security settings, installing drivers, purchases, uploading user data, replacing frozen goldens or relaxing acceptance criteria.
