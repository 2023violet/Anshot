> 独立复核的本地路径脱敏副本；原始哈希见 RESULTS.json。

# P03-G independent stop review

Date: 2026-10-17. Reviewer: separate `save_review` agent, not the implementer. Authority: the recorded local-development policy; this final review follows the explicit instruction to stop the isolation branch after two capability failures. Scope is read-only evidence/hash/static control-flow review. The reviewer did **not** execute either probe, any station/desktop/clipboard API, cargo tests, GUI operations or subsequent workflow steps. Only reviewer evidence files were written under ignored `target/p03-g-evidence/`.

**Verdict: `ISOLATION_CAPABILITY_UNAVAILABLE_STOPPED`.** Both bounded attempts built successfully and then failed at `CreateWindowStationW`. The failures are valid capability counterexamples in this environment; they are not success-criteria PASS results. No independent-station clipboard integration, restoration or cross-application acceptance was reached. The branch must remain stopped without elevation, ACL/token/privilege changes, opening/reusing an existing station, or WinSta0 fallback.

## Exact attempts and identities

Observed branch/HEAD: `main`, `94f7f419aee91faf6f846349d40a897d0d72d7d0`, with preserved uncommitted work. Each report, copied EXE, source snapshot, pre-execution review and every build/probe stdout/stderr hash was independently rechecked. Reported build exit is 0 and probe EXE exit is 2 for both attempts; the Python workflow command consequently returned failure, not a waived pass.

| Attempt | Measured result | Exact identity |
| --- | --- | --- |
| NULL auto-name + `CWF_CREATE_ONLY` | `CreateWindowStationW` OS183; isolation stdout empty | Report `target/p03-g-evidence/clipboard-20261017T145611514253Z/report.json`; SHA256 `b49d4ec15cbc1e7645df11574841068cd482010b3002f92faf2c5935eefa0a0d` |
| NULL-name copied EXE | SHA256 independently matched report | `e3542d87cf1ddaeb3a96634bafebc205848f525879d83b21834438ca0e040718` |
| NULL-name pre-execution review | Four source identities matched report and retained snapshots | `PRE_EXECUTION_REVIEW_NULL_NAME.json`, SHA256 `f4ab7d59f412dc13ca8722de052ac94f8346303d9fcd76599903b1e559de1cc2` |
| PID/time unique name + `CWF_CREATE_ONLY` | `CreateWindowStationW` OS5 AccessDenied; isolation stdout empty | Report `target/p03-g-evidence/clipboard-20261017T145943299952Z/report.json`; SHA256 `336830404fb348fddeff4f4dbbe96c8558af75f4d4b8d950e0444e7e5db79e37` |
| Unique-name copied EXE | SHA256 independently matched report | `e7e2967cce12167c2f912e90b56c0f5d0d374a47b664da19ee3a662fdf6eba4c` |
| Unique-name pre-execution review | Current four source identities matched report | `PRE_EXECUTION_REVIEW.json`, SHA256 `4e52b706b2ea1b30d641bb8bc737ed5ec57e8c8ccf676ea9d7f00af7381c42ea` |

The NULL-name source is retained in `probe-null-name.before.rs`, SHA256 `c01759d9455538980f50a2a862c65bc6bb8c43d87a92e6d7a84d8af46a88c422`. The unique-name source and remaining current inputs are:

| File | SHA256 |
| --- | --- |
| `poc/native-egui/examples/clipboard_probe.rs` | `5f5d4a0d17ffcf6b345761ec0ae5b7771689edefd410d0ad394f147abbebccf9` |
| `poc/native-egui/Cargo.toml` | `84744780e3dc83688a58cf385436e22769679efe67aa6c30a25ea51ec61af350` |
| `poc/native-egui/Cargo.lock` | `b62d2d0993d34cc363317faa89d0cc56fcdd3610f3ffef57b16ae3e910a35038` |
| `tests/native_oracle/run_clipboard_checks.py` | `1c4a98cb99b0ba6326ae44a07e5355e2ef31080cc1fd8aa1b2b1759daacfdbe0` |

The last three files have identical identities in both reviewed attempts. The retained `runner-null-name.before.py` also matches the current runner byte hash. The source change between attempts was limited to constructing/passing the unique UTF-16 name and its comments; isolation flags, rights, assertions and remaining control flow were unchanged.

## What was and was not executed

This conclusion combines the revalidated exact artifacts/logs with static review of the failure branch, not a separate Windows API trace:

1. Each probe initially obtained borrowed original process-window-station and thread-desktop handles and called **`GetClipboardSequenceNumber` on the original station as auxiliary metadata**. No clipboard content was read. It would be inaccurate to describe these attempts as making no clipboard API call at all.
2. The next creation call returned a NULL station handle with OS183 or OS5. The code immediately returns an error on that condition.
3. The `Isolated` guard is constructed **after** successful station creation. Consequently neither observed failure constructed that guard or entered its `Drop`/restore path.
4. No successful owned station/desktop handle was obtained. `SetProcessWindowStation`, `CreateDesktopW`, `SetThreadDesktop`, station visibility/current-context assertions and clipboard `OpenClipboard`/`CountClipboardFormats`/`CloseClipboard` were not reached.
5. Neither source contains a clipboard-content `GetClipboardData`, `EmptyClipboard` or `SetClipboardData` call. The isolation-ready/finish markers are absent. No original clipboard Open/Get/Empty/Set occurred in the reviewed control flow, and no after-restore sequence comparison was executed.

Therefore the evidence supports **early failure before context switching or content access**, not measured successful isolation, empty clipboard, context restoration or owned-handle cleanup. The initial sequence query cannot prove original-content integrity or absence of concurrent user changes. No such broader claim is made.

## Authorization and error discipline

Both attempted versions used current-token rights, API default security attributes, `CWF_CREATE_ONLY`, and a separate test process. Static review found no elevation, security-descriptor modification, privilege adjustment, display switch, input injection, existing-object fallback or interactive clipboard fallback. NULL-name OS183 safely refused reuse. Named creation OS5 was treated as terminal; no attempt to obtain new rights followed.

Microsoft documents requirements/restrictions for named station creation; the OS5 observation establishes denial for this attempted API in the current environment. The reviewer did not query token membership or determine a more specific denial cause, and does not claim that all Windows environments lack this capability.

The earlier approval artifacts were narrowly scoped permission to attempt their exact isolation-only versions, not evidence of successful capability or permission for content tests. **The subsequent stop instruction and two-failure workflow state supersede further execution based on those historical approvals.** No third attempt or full probe is approved by this report. A future change of direction must be separately authorized/reviewed within the project's workflow; this report does not recommend changing permissions to bypass the failure.

The pre-execution review had also found and corrected two evidence/safety-gate defects before either approved execution: CountClipboardFormats zero/error ambiguity, and the need to recheck reviewed sources after build/copy but before launch. Their correction is source-reviewed; the count API and later lifecycle path were not reached at runtime here.

## Locks, preserved product code and workflow state

The reviewer independently compared all native lock package records against `target/p03-g-evidence/native-lock.before`: the package key set, versions, checksums and dependency records are unchanged except the native package's one dev dependency edge to existing `windows-sys 0.61.2`. No new registry package or upgrade occurred. Root lock is byte-identical to `root-lock.before`, SHA256 `574836ca69c075b9844928fcc320d89ce2742001ae4f0596b67558b6ff1f5266`.

Comparison with P03-F's 32-input map found only the expected native `Cargo.toml`/`Cargo.lock` changes; all previously reviewed functional Rust sources, native runner and frozen fixture inputs remained byte-identical. This preserves the historical P03-F artifact's scope; it does not relabel that earlier artifact as a complete current P03-G verification.

At review, genuine plan step 1 records **`fails=2`, `status=needs_replan`**, with both failed commands. Steps 2-7 remain `todo` with no evidence. Full isolated clipboard data/format/busy/alpha checks, UI observability, current complete root/native/oracle regressions, new product-EXE validation and final product gates were **not performed for P03-G**. The reviewer did not run them. Stop/archive documentation is a valid outcome; it must not convert these unstarted steps into passed acceptance items.

## Independent read-only verification

Actual reviewer command:

`$LOCAL_USER/.cache/codex-runtimes/codex-primary-runtime/dependencies/python/python.exe target/p03-g-evidence/reviewer_failure_verify.py`

Final command result: exit 0. This script only reads files, computes hashes, parses JSON/TOML/Python syntax, checks exact lock/source/report relationships and writes its own evidence JSON. It does not import/run the probe runner, launch an EXE, build code or call station/clipboard APIs.

Machine-readable result: `target/p03-g-evidence/reviewer-failure-verified.json`, SHA256 **`bcba8e0f2f245a1026572724455ac19e44b8776607da476d1052216ab2bbafda`**. It records both copied-EXE/report/pre-review/log identities, current source identities, lock conclusions and the observed workflow state.

Remaining product gaps are unchanged: real clipboard import/export, Paint/second-application pixel/alpha readback, real foreground Ctrl+V message routing, screenshots/DPI/Narrator/human tasks and full G06/product acceptance remain unverified. Further independent local UI work may proceed under its own scope; this stopped isolation branch supplies no positive integration result.

Primary API references previously consulted for this review sequence: [CreateWindowStationW](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-createwindowstationw), [SetProcessWindowStation](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setprocesswindowstation), [SetThreadDesktop](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setthreaddesktop), [CountClipboardFormats](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-countclipboardformats), and [GetClipboardSequenceNumber](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getclipboardsequencenumber). These define behavior; the two observed failures remain the actual capability evidence.
