# ShareReady — Rust-first image-sharing preparation tool (working title)

> **STATUS: PRODUCT DISCOVERY / NOT RELEASED.** This repository currently contains product-governance and research drafts, **not an application**. 中文见下。

ShareReady is a **temporary project code name**, not a cleared public-facing product name. An unrelated image-sharing product already uses the name; trademark/branding checks remain open.

## What we are investigating / 正在研究什么

我们正在验证：普通人在分享截图前，能否比使用已有截图工具更快、更准确地遮挡自己不想公开的信息，并检查**实际导出的图像**。目前没有证据证明我们已优于 Redacted、ShareX 或系统截图工具。

- **Rust-first**: Rust 将负责图片处理与安全关键逻辑。Rust/WASM 浏览器端为推荐候选，**尚未冻结具体 UI 框架**。
- **Free and open source**: repository currently licensed under [MIT](LICENSE); no paid tier planned for MVP.
- **Local by default**: 不上传用户图片、不依赖账号；安全承诺必须经过后续测试才可发布。
- **No false safety claim**: 我们不会承诺自动发现所有私人信息或“绝对匿名”。

## Current stage / 当前阶段

`Stage 00: CONDITIONAL GO` → `Stage 01: GATE A/B PASS / S01–S03 DONE / S04 IN_PROGRESS / A1 NOT_VERIFIED`.

**No Rust application code, human usability benchmarks, user interviews, deployment or release is claimed.** We do have scripted third-party browser results, which are not human performance claims.

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
| [Public evidence ledger](docs/research/STAGE01_PUBLIC_EVIDENCE_LEDGER.md) | Competitor claims and explicit evidence gaps |
| [A1 problem evidence & counterevidence](docs/research/STAGE01_A1_DESK_EVIDENCE_2026-10-14.md) | External user discussions, promotion bias, and falsification plan |
| [Stage 01 internal readiness self-review](docs/reviews/STAGE01_READINESS_REVIEW_2026-10-14.md) | Same-agent desk review, not an external independent audit |
| [Participant study pack](docs/research/STAGE01_PARTICIPANT_STUDY_PACK.md) | Consent, neutral interview, counterbalanced UX tasks |
| [Rust-first ADR candidate](docs/architecture/ADR-0001-RUST-FIRST-DELIVERY-CANDIDATES.md) | Architecture alternatives without premature dependency lock |
| [Skill CLI gate runbook](docs/governance/STAGE01_GATE_RUNBOOK.md) | How to complete real Gate A/B in executable checkout |
| [Competitor test contract](docs/research/COMPETITOR_TEST_CONTRACT.md) | Reproducible comparative UX/safety research protocol |

## Design-to-delivery route / 交付顺序

Stage 00 research → Stage 01 PRD → Stage 02 UX → Stage 03 visual exploration → Stage 04 design system → Stage 05 high fidelity → Stage 06 Rust engineering → Stage 07 verified open-source release.

Each stage needs evidence and an explicit exit decision. Product discovery is not a license to implement. Stage 01 work is governed by the linked decision card and the authors' [deciding-and-executing](https://github.com/2023violet/deciding-and-executing) skill.

## Contributing / 参与

At present, reproducible task scenarios, references to competitor behavior, and consent-based usability research feedback are more valuable than feature pull requests. **Use invented/synthetic screenshots only; never publish real private screenshot data in an issue.**

## License

[MIT](LICENSE). Third-party code and assets require independent licensing review.
