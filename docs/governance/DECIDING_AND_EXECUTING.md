# Deciding-and-executing — ShareReady adoption record

**Record:** SR-GOV-001 · **Date:** 2026-10-14 · **Status:** ADOPTED AS WORKFLOW, AUTOMATED GATES NOT YET EXECUTED.

## Source of truth

- Upstream: [2023violet/deciding-and-executing](https://github.com/2023violet/deciding-and-executing)
- Inspected upstream `main` SHA: `1eb9624ee2037e4a2b253ee8120fa274dad1ef21`.
- Source instruction: [`skills/deciding-and-executing/SKILL.md`](https://github.com/2023violet/deciding-and-executing/blob/1eb9624ee2037e4a2b253ee8120fa274dad1ef21/skills/deciding-and-executing/SKILL.md).
- Observed metadata version: `4.1.0`; upstream README/CHANGELOG expressly describe it as a **candidate**; no claim of real Agent comparative evaluation.
- Companion: [`closing-out`](https://github.com/2023violet/deciding-and-executing/tree/1eb9624ee2037e4a2b253ee8120fa274dad1ef21/skills/closing-out).

## Workflow contract

`Direction → Gate A → Plan → Gate B → Execute → Review/Retro → Archive/closing-out`.

Read relevant `references/stage-N-*.md` at each stage. The skill defines `.decision/decision-card.md`, `.decision/plan.json`, artifact evidence, `preflight` and `coverage` through `scripts/de.py`, with two-step previews and confirmation. **Never claim a gate passed because a draft file exists.**

A standard- or high-risk task needs a real, runnable installation (project-local or agent environment) of **the exact inspected version**, plus an actual Git checkout, before using automated state transitions. First commands in a new session: `status` if an initialized `.decision/` exists, and `preflight` before executing any `verify_cmd`. Manual card review cannot substitute for recorded `de.py` gate evidence.

## Limitations at this handoff

The GitHub connector can read/write repository files, but it cannot run upstream Python scripts. The available runtime cannot resolve github.com directly, so the upstream `de.py` was **not** executed on a local clone in this handoff. This is **not** equivalent to a script-verified gate.

Current outcome: `STAGE01_GATE_A_CANDIDATE_HUMAN_APPROVED / CLI_GATE_A_PREVIEW_REQUIRED` (2026-10-14; see [approval record](../decisions/STAGE01_GATE_A_APPROVAL_RECORD.md)). No `.decision/` machine-state folder was fabricated, no `gate A --confirm` was executed, and no verification result was invented.

## Next authorized transition

1. **Completed (human candidate level):** The user approved the presented Stage 01 decision candidate; retained in [the approval record](../decisions/STAGE01_GATE_A_APPROVAL_RECORD.md). This is not an actual `de.py gate A` run.
2. When a real execution checkout and exact upstream skill are available, initialize the actual `.decision/` workspace with `de.py init --risk standard`, enter the reviewed card, run `check card`, show the real `gate A` preview and request acknowledgment **again** before calling `--confirm`.
3. Then create the Stage 01 PRD/research execution plan, test A1 first, `check plan`, and request Gate B approval before carrying out experiments or product code.
4. At closure, use the current companion `closing-out` with explicit evidence layers: static audit, repository fact checks, runtime validation and human usability testing.

The remote draft PR documents the approach; it **does not** bypass script or human gates.

## Stage 01 drafts available after human candidate approval

- [PRD draft](../product/STAGE01_PRD_DRAFT_v0.1.md)
- [Research execution plan](../research/STAGE01_RESEARCH_PLAN_v0.1.md)
- [Gate B candidate plan](../plans/STAGE01_GATE_B_CANDIDATE.md)
- [Executable checkout and gate runbook](STAGE01_GATE_RUNBOOK.md)

These are authored research/planning drafts, **not executed user research** or passed gates.

## Current execution update — 2026-10-14 PDT

The initial local DNS blocker was resolved using an isolated GitHub Actions runner. **This section supersedes all earlier pending/blocked states in this historical adoption record.**

- Real Gate A confirmation: [run #38676305998](https://github.com/2023violet/ShareReady/actions/runs/38676305998), owner-approved token matched, `status.gates.A=valid`.
- Real Gate B confirmation: [run #38676996877](https://github.com/2023violet/ShareReady/actions/runs/38676996877), owner-approved token matched, `status.gates.B=valid`.
- S01 assessment finished and S02 started through real Skill transitions: [run #38678816777](https://github.com/2023violet/ShareReady/actions/runs/38678816777), latest machine-state [artifact](https://github.com/2023violet/ShareReady/actions/runs/38678816777/artifacts/11947187397).
- Five synthetic screenshots and ground truth actually generated, not just specified: [fixture manifest](../research/fixtures/manifest.json), [CI run](https://github.com/2023violet/ShareReady/actions/runs/38679131718).
- Product evidence: `A1=NOT_VERIFIED`, `recruited=0`, `competitor_hands_on=0`. Fixture visual inspection, original OS/version competitor task runs, user interviews and the final independent reviewer remain outstanding.
- **No Rust product code, merge, deploy or release is approved.** PR remains Draft. The earlier text is preserved as a historical account, not current gate state.

## Latest machine state and research limitations — 2026-10-14 PDT

Real Skill [CI 38680774786](https://github.com/2023violet/ShareReady/actions/runs/38680774786) confirmed `A=valid/B=valid; S01 DONE; S02 DONE; S03 IN_PROGRESS`. Its machine-state [artifact 11948115660](https://github.com/2023violet/ShareReady/actions/runs/38680774786/artifacts/11948115660) has an expiration date, so future runs must verify the exact archive and underlying Git hashes. Later research-only commits have not altered the approved decision card or machine plan.

Real online Chromium navigation and synthetic file import were proven in [CI 38680949676](https://github.com/2023violet/ShareReady/actions/runs/38680949676) and [CI 38681111604](https://github.com/2023violet/ShareReady/actions/runs/38681111604). Four real competitor PNG exports and scripted mask observations were saved by [CI 38681392251](https://github.com/2023violet/ShareReady/actions/runs/38681392251), with recheck in [CI 38681695615](https://github.com/2023violet/ShareReady/actions/runs/38681695615). [S03 report](../research/STAGE01_S03_LIVE_BROWSER_PILOT_REPORT.md) distinguishes actual file measurements, headless automation artifacts and unresolved manual-gesture behavior. **S03 remains IN_PROGRESS; no measured human usability or valid cross-product superiority claim.**

Real participant interviews = 0; A1 remains OPEN. Rust product code, merging and release remain forbidden.

## Latest verified machine state (2026-10-15)

The actual pinned author Skill was restored and run in [CI 38689148605](https://github.com/2023violet/ShareReady/actions/runs/38689148605). Immutable Gate A/B approval provenance was rechecked, actual PNG binaries were reread against frozen synthetic source image hashes and the S03 research report was added as a hashed artifact. **S01–S03 = DONE; S04 = IN_PROGRESS; S05a onward TODO.** [Machine-state artifact #11951211324](https://github.com/2023violet/ShareReady/actions/runs/38689148605/artifacts/11951211324), ZIP SHA256 `23a2d6822e37459922b455da4aaec3c632914fb23af4d548a6724f42bdbd5a82`, expires 2026-11-14 UTC.

[SR-RES-008](../research/STAGE01_S03_REPRODUCED_BASELINE_2026-10-15.md) is the corrected actual third-party export baseline. [S04 protocol](../research/STAGE01_S04_RESEARCH_PROTOCOL.md) and [consent](../research/STAGE01_S04_CONSENT_AND_SCREENERS.md) are drafts awaiting external review, not proof of recruitment. Participants contacted/recruited/consented/interviewed = **0**. A1 still OPEN. No Rust product implementation, merge, deploy or release.
