# ShareReady V1 design system and handoff

**2026-10-15 — VISUAL ACCEPTED, COMPONENT SPEC PROPOSED**

## Structure

One task / one image / one final PNG. 68px topbar, ~64px tools, dominant canvas, ~300px inspector, ~44px footer. Review surface is distinct from editing; no Dashboard, user accounts, cloud history or project nav.

## Design tokens

| Role | Value |
|---|---|
| Surface | #FFFFFF |
| Canvas | #E9EDF2 |
| Ink | #1E293B |
| Muted | #64748B |
| Divider | #DCE3EB |
| Primary accent | #345CC7 |
| Active-tool accent | #EEF3FF |
| Error | #C33D45 |
| Opaque exported fill (candidate) | #111827 |
| Spacing | 4/8/12/16/24/32px |

System fonts only; do not add external CDN fonts to the privacy-sensitive release without review.

## Components and states

| Component | States / contract |
|---|---|
| Main CTA | normal / loading / disabled; never generates original image as fallback |
| Tool toggle | mask / select / pan, focus and pressed semantics |
| Mask rectangle | default / selected / move / resize; real pixel coverage happens in Rust, not via CSS |
| Region list | N region IDs, synced selection, deletion and undo; do not OCR private texts |
| Zoom viewport | fit / 100% / zoom / pan; source coordinates independent of CSS |
| Import | idle / loading / rejected; file chooser, paste and drop planned; mock cannot import |
| Output review | render / ready / stale / error; must decode final bytes in real product |
| Clipboard | real success / denied / unsupported with download alternative |
| Settings and reset | small overlay, ESC and focus return, clear discard warning |
| Status toast | factual messages; fake security or copy success forbidden |

## UI fidelity and remaining gaps

- Primary 1440x900; desktop 1024x768; 800px compact concept only, not full mobile support.
- 33 **same-agent local Chromium mock QA** checks passed including mask handle resize, undo/redo, pan, viewports, settings and review feedback. Independent accessibility/browser and security testing have NOT passed.
- Keyboard-only area adjustment, focus trapping, DPR2 pointer mapping and browser zoom 125%/150% remain engineering/design QA tasks.
- All real secure image import, EXIF removal, clipboard and actual PNG export are NOT IMPLEMENTED.

Refs: [functional contract](../product/SHAREREADY_V1_FUNCTION_CONTRACT_CANDIDATE.md) and [offline HTML mock](prototypes/ShareReady_V1_UX_Prototype.html).
