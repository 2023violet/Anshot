# AGENTS.md — ShareReady

## Project truth and scope

- The project has a real V1.3-03 Rust/WASM editor and Windows desktop host on `main`, plus historical research drafts. Do not present release, security audit or human usability testing as independently complete when they are not.
- The name **ShareReady is temporary** due to an existing product with a similar name and scope. Do not assert brand clearance.
- **Rust is mandatory for safety-critical image transformation and export.** Implemented delivery uses Rust/WASM for image transformation and WinForms + WebView2 for the Windows desktop host.
- All product work is governed by Stage 00–07 evidence gates. Primary artifacts are `README.md` and the linked documents under `docs/`. No nonessential framework or dependency should be added.
- Historical Stage 00/01 gate records remain evidence of past decisions, not a statement that the existing product implementation is absent. Maintain explicit authorization and verifiable evidence for future high-risk changes.

## Required decision-to-execution workflow

For multi-step or consequential work, **read the upstream SKILL.md in full** from the project authors' repository: `https://github.com/2023violet/deciding-and-executing/blob/main/skills/deciding-and-executing/SKILL.md`. Read the relevant stage reference files, and on milestone closure read `skills/closing-out/SKILL.md`.

At initial adoption, inspected upstream repository main commit: `1eb9624ee2037e4a2b253ee8120fa274dad1ef21` (2026-10-14). It identifies deciding-and-executing `v4.1.0` **candidate**, not proven stable. Recheck provenance/version before execution; do not silently change versions. Details: [workflow adoption](docs/governance/DECIDING_AND_EXECUTING.md).

- Use **standard** workflow for multi-stage product design and research; elevate to **high** for production changes, destructive actions, security configuration or external release.
- Stage ①: compare alternatives and document a numbered decision card; Gate A checks goal, acceptance criteria, assumptions and stop conditions.
- Stage ②: plan verifiable steps, covering every acceptance criterion, validating the riskiest assumption first; Gate B reviews command/side effects.
- Stage ③: execute only the approved next step; capture evidence with artifact/version identity.
- Stage ④: review independently, distinguish command pass from real UX/security validation.
- Stage ⑤: use closing-out for documentation knowledge sync and archive; never delete workspace without explicit review and authorization.
- **Outside the continuous local development authorization below, Gate A/B previews require separate human acknowledgment. Never self-confirm a token in the same exchange.** When script execution is unavailable, use manual stage checklists and label the gate `MANUAL / NOT SCRIPT-VERIFIED`, **not PASS**.
- Do not create arbitrary `.decision/` files by hand; initialize with the current genuine skill CLI when execution is available.
- Always distinguish `DOCUMENTED`, `MEASURED`, `HYPOTHESIS`, `NOT_VERIFIED`.

## Continuous local development authorization

The user explicitly approved this policy on 2026-10-17; the approval and exact scope are recorded in [the authorization record](docs/governance/AUTONOMOUS_LOCAL_DEVELOPMENT.md). The agent may autonomously plan, implement, test and review reversible local batches within the approved V2 product roadmap. For these batches only, the agent may generate and confirm genuine workflow Gate A/B and review tokens, and apply non-destructive documentation archives, without another per-token human reply. This is an explicit exception to the human-acknowledgment requirements of this AGENTS.md and the adopted author workflow for that bounded scope. Record the policy approval and each agent confirmation honestly; never describe an agent-confirmed token as a human-reviewed token.

Retain main-only development, expected-HEAD checks, preservation of unrelated work, Rust safety-critical transformations/export, synthetic fixtures, locked dependencies, acceptance coverage and exact-artifact evidence. Production-impacting local code changes retain high-risk evidence and independent review requirements; the exception only removes repeated human confirmation for reversible local work. Independent review may use a separate sub-agent with the change and acceptance criteria; do not treat the implementer's own review as independent.

The exception does not authorize deleting/moving user files or the V1 path, rewriting history, force-pushing, changing secrets/authentication/permissions/security or privacy settings, installing drivers, purchases, external messaging, uploading user data, remote publishing, pushing or releasing. Surface the concrete impact and obtain express human confirmation before such actions. Never silently relax success criteria, replace frozen goldens, claim unobserved human usability/accessibility tests passed, or claim the full product complete from partial machine tests. Preserve unresolved environment and human-evidence gaps; continue independent authorized work where possible.

## Change safety and review

**Main-only repository policy:** use `main` as the sole development, test and delivery branch; do not create additional long-lived branches or Draft PRs for routine changes. Make focused changes with expected-HEAD checks, exact-code tests and review evidence. Do not rewrite history, force-push, delete files, change secrets/auth/permissions or deploy without separately surfaced impact and express confirmation. Never bypass safety gates due to general repository write authorization.

Product privacy baseline: local image processing, opaque pixel replacement for private areas, new-image export without original source layers/metadata, synthetic test fixtures, and honest limitation wording. None are verified until tests are run.

Before implementing or releasing, verify current requirements, source status, exact code changes, security/functional test evidence, browser behavior and visual QA. Do not convert unobserved human tests into synthetic positive results.
