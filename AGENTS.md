# AGENTS.md — ShareReady

## Project truth and scope

- The project is in **product discovery**; do not present designs, Rust code, deployments or user tests as complete before independently verified.
- The name **ShareReady is temporary** due to an existing product with a similar name and scope. Do not assert brand clearance.
- **Rust is mandatory for safety-critical image transformation and export.** Rust+WASM browser UI is a candidate, not a frozen architecture.
- All product work is governed by Stage 00–07 evidence gates. Primary artifacts are `README.md` and the linked documents under `docs/`. No nonessential framework or dependency should be added.
- No product code should be started before Gate A and Gate B have genuine authorization under the workflow.

## Required decision-to-execution workflow

For multi-step or consequential work, **read the upstream SKILL.md in full** from the project authors' repository: `https://github.com/2023violet/deciding-and-executing/blob/main/skills/deciding-and-executing/SKILL.md`. Read the relevant stage reference files, and on milestone closure read `skills/closing-out/SKILL.md`.

At initial adoption, inspected upstream repository main commit: `1eb9624ee2037e4a2b253ee8120fa274dad1ef21` (2026-10-14). It identifies deciding-and-executing `v4.1.0` **candidate**, not proven stable. Recheck provenance/version before execution; do not silently change versions. Details: [workflow adoption](docs/governance/DECIDING_AND_EXECUTING.md).

- Use **standard** workflow for multi-stage product design and research; elevate to **high** for production changes, destructive actions, security configuration or external release.
- Stage ①: compare alternatives and document a numbered decision card; Gate A checks goal, acceptance criteria, assumptions and stop conditions.
- Stage ②: plan verifiable steps, covering every acceptance criterion, validating the riskiest assumption first; Gate B reviews command/side effects.
- Stage ③: execute only the approved next step; capture evidence with artifact/version identity.
- Stage ④: review independently, distinguish command pass from real UX/security validation.
- Stage ⑤: use closing-out for documentation knowledge sync and archive; never delete workspace without explicit review and authorization.
- **Gate A/B previews require separate human acknowledgment. Never self-confirm a token in the same exchange.** When script execution is unavailable, use manual stage checklists and label the gate `MANUAL / NOT SCRIPT-VERIFIED`, **not PASS**.
- Do not create arbitrary `.decision/` files by hand; initialize with the current genuine skill CLI when execution is available.
- Always distinguish `DOCUMENTED`, `MEASURED`, `HYPOTHESIS`, `NOT_VERIFIED`.

## Change safety and review

Use a new branch and a draft PR for candidate decisions. Preserve `main` until review gates are satisfied. Do not rewrite history, force-push, delete files, change secrets/auth/permissions or deploy without separately surfaced impact and express confirmation. Never bypass safety gates due to general repository write authorization.

Product privacy baseline: local image processing, opaque pixel replacement for private areas, new-image export without original source layers/metadata, synthetic test fixtures, and honest limitation wording. None are verified until tests are run.

Before implementing or releasing, verify current requirements, source status, exact code changes, security/functional test evidence, browser behavior and visual QA. Do not convert unobserved human tests into synthetic positive results.
