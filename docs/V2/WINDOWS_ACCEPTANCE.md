# SR-V2-00 — manual Windows validation checklist

**Every item NOT_RUN until a real operator observes it.** Existing WebView2 smoke is not Explorer drag or Narrator evidence.

| Test | Required operator action | Artifact |
|---|---|---|
| W01 | Win10/11 extract V1.3 ZIP, verify inner ZIP SHA, launch | Windows version and true startup |
| W02 | Native file chooser PNG/JPEG/invalid and cancel | Actual image/retained edit |
| W03 | True Windows Explorer drag into window | Not a simulated DOM drop |
| W04 | Ctrl+V image copied from third-party app | Format, pixels and focus |
| W05 | T04 4 corners at 100/125/150/200% DPI | Source-pixel golden parity |
| W06 | Move window across monitors of different DPI | Pointer and scaling behavior |
| W07 | 8 handles, keyboard nudge, undo/redo, clear-cancel | Actual mask state |
| W08 | Save dialog cancel, replace, readonly and source-path | Target file checksums |
| W09 | Paste exported clipboard image into Paint and another app | Both decoded RGBA comparisons |
| W10 | Replace/undo while Clipboard/Save is pending | No stale current success |
| W11 | Narrator + keyboard complete task | Focus, labels and alternative to canvas pointer |
| W12 | Genuine 24MP image and repeated export | RSS, GPU and response measurements |
| W13 | Perform offline and network-observed session | No unexpected image egress/log leak |

Every item records device/build, operator and date, input SHA, source commit, steps, actual observation, failure/retry, proof file and PASS/FAIL/NOT_RUN. Never synthesize an operator signature from code or CI results.