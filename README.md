# ShareReady — Rust-first image-sharing preparation tool (working title)

> **STATUS: V1.3-03 ENGINEERING INTEGRATED / NOT RELEASED.** This repository contains a working Rust/WASM screenshot redaction editor and Windows WebView2 desktop host, alongside historical research records. Exact-code CI passed on 2026-10-17; this is not an independent security certification or human usability sign-off.

ShareReady is a **temporary project code name**, not a cleared public-facing product name. An unrelated image-sharing product already uses the name; trademark/branding checks remain open.

## What we are investigating / 正在研究什么

我们正在验证：普通人在分享截图前，能否比使用已有截图工具更快、更准确地遮挡自己不想公开的信息，并检查**实际导出的图像**。目前没有证据证明我们已优于 Redacted、ShareX 或系统截图工具。

- **Rust-first**: 已实现 Rust 图片解码、实心遮挡与 PNG 输出；浏览器使用 Rust/WASM，Windows 桌面使用 WinForms + WebView2 承载同一编辑器。
- **Free and open source**: repository currently licensed under [MIT](LICENSE); no paid tier planned for MVP.
- **Local by default**: 不上传用户图片、不依赖账号；安全承诺必须经过后续测试才可发布。
- **No false safety claim**: 我们不会承诺自动发现所有私人信息或“绝对匿名”。

## Current engineering baseline / 当前工程基线

- **唯一正式开发和交付分支**：`main`。后续开发以 `main` 为基线；不要创建长期功能分支或将已实现功能留在 Draft PR 中。
- **实际产品目录**：`crates/shareready-core/`（独立 Rust 图片核心）、`crates/shareready-editor/`（Rust 编辑状态机基础）、`e1-spike/`（V1 CLI/WASM 兼容层）、`web-app/`（当前 JS 编辑器）、`desktop-windows/`（Windows WebView2 宿主）。Windows 用户入口见 [desktop-windows/README.md](desktop-windows/README.md)。
- **最新通过的原生/浏览器 CI**：[V1.3-03 source-matched run #38995384142](https://github.com/2023violet/ShareReady/actions/runs/38995384142)，对应提交 `0e9e50a780f74906937f177c403db5ee2bfe9ec2`；后续提交须重新验证。
- **验收边界**：机器合成图片/Windows CI 不等于人工操作、跨设备 DPI、全面安全或正式发布验收；产品尚未发布。

## V2 Rust Native refactor (current planned track)

V2-00: V1.3 source, five-image full-frame RGBA golden, F01-F12 migration matrix and native GUI POC contract; all staged in [docs/V2/](docs/V2/README.md). V1 Windows/WebView2 app is retained; no Rust-native GUI implementation or human Windows acceptance is claimed. Main remains the only development branch. The JPEG reference for T05 is the V1 Rust normalized preview rather than assuming Pillow JPEG decoding is byte-identical.

## Historical Stage 01 research status / 历史研究阶段记录

`Stage 00: CONDITIONAL GO` → `Stage 01: GATE A/B PASS / S01–S04 DONE / S05a BLOCKED_BY_RECRUITMENT / A1 NOT_VERIFIED`.

这些是历史研究门禁和仍未完成的真人用户验证，不代表当前仓库没有产品实现。已存在 Rust 应用代码，但没有以此声称真人用户测试、部署或正式发布。

| Document | Purpose |
| --- | --- |
| [AGENTS.md](AGENTS.md) | Agent constraints, stage gates and collaboration expectations |
| [Skill adoption](docs/governance/DECIDING_AND_EXECUTING.md) | Exact upstream skill, version and honest execution limitations |
| [Stage 01 decision candidate](docs/decisions/STAGE01_GATE_A_CANDIDATE.md) | Direction alternatives, hypotheses, success criteria, stop rules |
| [Gate A human approval record](docs/decisions/STAGE01_GATE_A_APPROVAL_RECORD.md) | Human-approved token and machine Gate A confirmation |
| [Stage 01 PRD draft](docs/product/STAGE01_PRD_DRAFT_v0.1.md) | Product definition and unverified requirements |
| [Research plan](docs/research/STAGE01_RESEARCH_PLAN_v0.1.md) | Competitor study and participant test plan |
| [S02 fixture visual QA](docs/research/STAGE01_S02_VISUAL_AND_MATRIX_QA.md) | Real generated image review, benchmark fixture constraints |
| [S03 actual-browser pilot](docs/research/STAGE01_S03_LIVE_BROWSER_PILOT_REPORT.md) | Real third-party exported files, pixel comparisons and known automation limitations |
| [S03 scripted observations CSV](docs/research/STAGE01_S03_SCRIPTED_TRIALS.csv) | Oracle-assisted Chrome trials, not human UX statistics |
| [S03 repeated-mask event diagnostics](docs/research/STAGE01_S03_POINTER_ORDER_DIAGNOSTIC.md) | Historical automated drag failures, now resolved for tested tasks |
| [S03 authoritative matched export baseline](docs/research/STAGE01_S03_REPRODUCED_BASELINE_2026-10-15.md) | Two real tools, four downloaded PNGs and independent export inspection |
| [S04 ethics and recruitment protocol](docs/research/STAGE01_S04_RESEARCH_PROTOCOL.md) | Draft consent, balanced research design and stop conditions |
| [S04 screening and consent](docs/research/STAGE01_S04_CONSENT_AND_SCREENERS.md) | Not yet circulated; zero consenting participants |
| [S04 allocation slots](docs/research/STAGE01_S04_ALLOCATION_SLOTS.csv) | Ten UNASSIGNED balanced slots; not real people |
| [S04 internal adversarial review](docs/research/STAGE01_S04_REVIEW_AND_RECRUITMENT_FEASIBILITY_2026-10-15.md) | 15 open/privacy risks, real recruiting platform feasibility and no participant claims |
| [S04 machine readiness](docs/research/STAGE01_S04_READINESS.json) | Live outreach explicitly forbidden, all counters zero |
| [S05 operator handoff](docs/research/STAGE01_S05_RECRUITMENT_OPERATIONS_HANDOFF.md) | True human recruiting prerequisites; no outreach or payments authorized |
| [Public evidence ledger](docs/research/STAGE01_PUBLIC_EVIDENCE_LEDGER.md) | Competitor claims and explicit evidence gaps |
| [A1 problem evidence & counterevidence](docs/research/STAGE01_A1_DESK_EVIDENCE_2026-10-14.md) | External user discussions, promotion bias, and falsification plan |
| [Stage 01 internal readiness self-review](docs/reviews/STAGE01_READINESS_REVIEW_2026-10-14.md) | Same-agent desk review, not an external independent audit |
| [Participant study pack](docs/research/STAGE01_PARTICIPANT_STUDY_PACK.md) | Consent, neutral interview, counterbalanced UX tasks |
| [Rust-first ADR candidate](docs/architecture/ADR-0001-RUST-FIRST-DELIVERY-CANDIDATES.md) | Architecture alternatives without premature dependency lock |
| [Skill CLI gate runbook](docs/governance/STAGE01_GATE_RUNBOOK.md) | How to complete real Gate A/B in executable checkout |
| [Competitor test contract](docs/research/COMPETITOR_TEST_CONTRACT.md) | Reproducible comparative UX/safety research protocol |

## Historical V1 product scope and UX/UI candidates (review only)

The design candidates below document the historical planning baseline. They are **not the current implementation status, security certification, or evidence of A1 user demand**. The historical Stage 01 research obligations remain unverified where indicated.

| Design artifact | What to review |
| --- | --- |
| [V1 feature contract candidate](docs/product/SHAREREADY_V1_FUNCTION_CONTRACT_CANDIDATE.md) | 16 proposed P0/P1/excluded features and 12 concrete future engineering ACs |
| [UX/UI baseline candidate](docs/design/SHAREREADY_V1_UX_UI_BASELINE_CANDIDATE.md) | Single workspace, five screen states, visual tokens and interaction contract |
| [Offline interactive HTML prototype](docs/design/prototypes/ShareReady_V1_UX_Prototype.html) | Embedded synthetic demo chat; actual visual/interaction mock, NOT a safe-redaction image processor |
| [Design-track gate boundary](docs/governance/STAGE01_DESIGN_DIRECTION_CHANGE_PROPOSAL.md) | How to review UX without falsifying the outstanding S05/S06 dependency |

The HTML uses no CDN, account, real image import, real copy/download or Rust image transformation. The source is openable as a standalone file in a browser for review. Do **not** provide real private images to the prototype. Visual QA/Playwright mock tests are a design artifact only, not security testing.

## Design-to-delivery route / 交付顺序

The historical plan was Stage 00 research → Stage 01 PRD → Stage 02 UX → Stage 03 visual exploration → Stage 04 design system → Stage 05 high fidelity → Stage 06 Rust engineering → Stage 07 verified open-source release. Implementation has now reached V1.3-03; this does **not** imply all prior research and release conditions passed.

Changes should carry exact-code test evidence. High-risk security changes and any public release require an explicit review decision. Project governance references the authors' [deciding-and-executing](https://github.com/2023violet/deciding-and-executing) skill.

## Contributing / 参与

Develop and verify against `main` only; do not create long-lived feature branches for routine development. Keep security-sensitive changes reviewable and use synthetic test fixtures. **Never publish real private screenshot data in an issue.**

## License

[MIT](LICENSE). Third-party code and assets require independent licensing review.
