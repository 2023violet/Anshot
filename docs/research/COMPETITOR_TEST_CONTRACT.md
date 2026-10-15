# ShareReady comparator research contract — v0.1

**ID:** SR-RES-001 · **Status:** PROPOSED / NO TEST RUN · **Date:** 2026-10-14

## Claims must be typed

`DOCUMENTED`: official docs, source or verifiable screenshots (provide URL and date).  
`MEASURED`: personally executed repeatable task with recorded tool/version/environment.  
`NOT_VERIFIED`: not tested or the source does not substantiate it.  
`FAILED_TEST`: an observed test failure (with reproduction steps).  
`HYPOTHESIS`: predicted advantage, not evidence.

No GitHub star count, online review, developer demo, benchmark number, user quote or screenshot may be invented. Zero observations are not zero failures.

## Competitor baseline (candidate, re-check at time of measurement)

| Target | URL | Why compare? | Tested now? |
| --- | --- | --- | --- |
| Redacted | https://redacted.app/features | Web-local redaction, export and review | NO |
| Redaction Tools | https://redactiontools.com/redact-screenshot | Paste-review-copy flow | NO |
| ShareX | https://getsharex.com/docs/image-editor | Windows-native screen capture/edit | NO |
| System screenshot editor | OS/version must be recorded | Zero-install alternative | NO |
| Screenshot Redactor | https://github.com/cleanroom-ai/screenshot-redactor | Open-source local detection | NO |
| Existing ShareReady (naming conflict) | https://shareready.app/ | Shares name and pre-share positioning | NO |

No product entry is marked functionally inferior on the basis of this table. Record separate documented behavior before the first task session.

## Synthetic task fixtures (not yet created)

| ID | Fixed task | Expected evidence |
| --- | --- | --- |
| T01 | Chat screenshot: hide name, phone, avatar, notification preview | Redacted pixels and no missing required region |
| T02 | Order screenshot: hide delivery address and contact; preserve item/amount | Required/retained ground truth |
| T03 | Support screenshot: hide username path, demo token and visible account info | No leak in final exported image |
| T04 | Tiny labels at image boundary/high DPI | Per-pixel opaque replacement for marked area |
| T05 | JPEG with **fictional** EXIF/GPS values | Fresh export drops original metadata and layers |

Fixture artifacts must have explicit synthetic origins and license provenance. No production user screenshots should be collected.

## Procedure and evidence fields

1. Observe current workflow **before** explaining our proposed solution; screen for users who actually shared images in the last 30 days.
2. Record `participant_id` (pseudonym), consent status, anonymized screening, date, OS/browser/version, competitor/version, task ID, randomized order, instructions and ground truth.
3. For each task measure: completion, duration (start/end defined), interaction count, omissions, over-redaction, output integrity, confidence (1–5), and 1-line observed impediment.
4. Compare a baseline against a **clearly labeled** low-fidelity future prototype. Counterbalance order and use identical tasks/fixtures; record practice effects.
5. Keep consent records private/off-repository. Publish only aggregate counts and non-identifying illustrative examples with synthetic data.
6. Verify functionality separately from usability: inspect exported pixels, image metadata, and whether browser makes unexpected network requests. Never substitute self-reported confidence for actual leak checks.

### Suggested decision thresholds (not statistical population claims)

- Recruit approximately 10 participants, with at least 6 who independently describe a real recent related difficulty.
- Test at least two directly comparable competitor paths on identical tasks.
- Target ≥20% improvement in median task duration at non-worse omission/completion rates **or** a reproducible reduction in missed sensitive regions without a disproportionate time cost.
- If data is missing, report `NOT_VERIFIED` or `BLOCKED_BY_RECRUITMENT`. Do not infer market sizing, retention or willingness to pay.

## Next verification tasks (blocked behind Gate A/B)

- [ ] Revalidate competitor documentation against current versions.
- [ ] Prepare five synthetic fixture files and expected-region ground truth.
- [ ] Produce a consent-based recruitment and session script.
- [ ] Perform side-by-side existing product measurements (not done).
- [ ] Produce Stage 01 PRD v0.1 after recording what was and was not proven.

## Record template (CSV header, not measured data)

`participant_id,consent_ok,session_date,tool,tool_version,os,browser,browser_version,task_id,order_index,prior_experience,completed,duration_s,action_count,required_regions,missed_regions,over_redacted_regions,export_pixel_pass,metadata_pass,confidence_1to5,observed_issue,evidence_ref`

**Schema decision:** The record header above is the canonical Stage 01 field list; use the same spellings across all study documents. Each record needs a participant-task-tool identity, not real personal identifiers.

**Gate state:** `NO_TEST_RUN`. Do not advance based on this protocol alone.
