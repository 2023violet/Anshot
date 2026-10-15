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

Current outcome: `STAGE01_GATE_A_PREVIEW_REQUIRED`. No `.decision/` machine-state folder was fabricated, no `gate A --confirm` was executed, and no verification result was invented.

## Next authorized transition

1. Present this decision card's Gate A scope/acceptance/stop preview to the human and record an explicit approval or revision.
2. When a real execution checkout and exact upstream skill are available, initialize the actual `.decision/` workspace with `de.py init --risk standard`, enter the reviewed card, run `check card`, show the real `gate A` preview and request acknowledgment **again** before calling `--confirm`.
3. Then create the Stage 01 PRD/research execution plan, test A1 first, `check plan`, and request Gate B approval before carrying out experiments or product code.
4. At closure, use the current companion `closing-out` with explicit evidence layers: static audit, repository fact checks, runtime validation and human usability testing.

The remote draft PR documents the approach; it **does not** bypass script or human gates.
