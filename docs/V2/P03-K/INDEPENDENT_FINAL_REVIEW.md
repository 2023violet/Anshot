# P03-K — independent final local review

Verdict: ACCEPT_WITH_LIMITS

The current bounded background-source batch is accepted for its documented local synthetic/code boundaries. No unresolved blocker was found in the exact R2 code and evidence reviewed below. This is not full-product, physical-screen, human-usability, accessibility, cross-application clipboard, security-audit, package or release acceptance. Product status remains **IN_PROGRESS**. The reviewer did not implement the product changes.

## Exact accepted identity

- Branch `main`; HEAD `94f7f419aee91faf6f846349d40a897d0d72d7d0`.
- **84 compiled/test/fixture inputs**, all independently matched to current bytes, preserved source snapshots and the actual reports. Current code remained unchanged during the independent tests and runtime audit.
- Source release build: `114491f9a9a202b39f5e0a1c38db3e21fde738cb138bd0015c095a135c7be014`.
- Source EXE SHA256: `72b11e173dfe1822a64b68d2a5af73bd746dbc4150fdd732b530a91a5a400292`.
- Main non-pilot report: `target/p03-k-evidence/window-20261019T005902416947Z/report.json`, SHA256 `e87afd1eb18c3eec492a7d82e74951bdd3a47b5059ea6809e6d4fb6c906b8be5`.

The nested current J/fix/ordinary/large/performance reports and their separate EXEs were independently hashed and linked from actual main-report references. Ordinary eight-state evidence is a featureless **debug** EXE with its own source/build identity, not the source release EXE. The complete per-report identities and checks are retained in `independent-runtime-data.json`; historical directory names do not mean old results were reused.

## Independent work actually executed

Using explicit fixed `RUSTC` and `RUSTDOC` **1.98.1 (48a229cea 2026-09-01)** and the fixed cargo binary, I ran **41 successful named test executions**, comprising 22 ordinary and 19 `performance-evidence` cases, or **23 distinct named tests** across both configurations. These include all four source-worker tests, all ten intake tests, all three accept tests in both configurations, the ordinary terminal-Close/late-review/canceled-close defenses, and real-source visual/repeated-import waiting controls. No failed, ignored or measured selected tests were substituted. Commands used `--locked --manifest-path poc/native-egui/Cargo.toml --target-dir poc/native-egui/target --bin shareready-native-egui-poc` with recorded filters and feature flags. They did not launch a native window or access the system clipboard.

Actual command arrays, counts/names, full stdout/stderr hashes and all 84 before/after source hashes are in `independent-current-native-20261019T011059106634Z.json`, SHA256 `f107abece9c40297ca941f8f7d206e1274d839dd74e6687a1587041d7ab61339`. All 24 stream files were subsequently rehashed successfully. Earlier independent domain execution separately ran 30 lib + 2 workflow + 2 compile-fail cases against the unchanged current Editor hash; it is not relabeled as another new domain-suite run.

I then executed a read-only Python artifact audit, exit 0, 24.488 seconds:

- **17 complete image-oracle recomputations**: six source old/first/retry PNGs, four J first/retry PNGs, one large generated PNG, three performance grid/wide/tiled samples and three ordinary saved/review products.
- **11 complete controlled-Copy RGBA comparisons**: all six source, all four J, and the large case. Every byte agrees with its current PNG and independent mask composition. First/retry pixels differ as required. JPEG uses the explicitly allowed Rust-normalized source with independent composition; this does not independently validate JPEG decoding. Untrusted-source and generated-PNG ceilings remain separate.
- **10 independent Pillow RGB crop comparisons** of complete inset visible 100% regions: four source, four J and two wide/native-or-forced-512 performance images. Busy Fit remains preview-only.
- **423 directed file hashes**, all actual command stdout/stderr identities, five distinct EXEs, all case-file identities, six source-job and six J-job phase reconstructions, and **13 complete memory sample streams**.
- All eight performance cases/40 screenshot hashes, **30 distinct interactive frames per case** (240 total), and 20 distinct actual reimport-trigger/confirmed-reimport/controlled-copy rounds were checked. The repeated-import controller genuinely waits for completion and acceptance.

The executed script is `independent_runtime_audit.py`, SHA256 `c5b8975988c31f88d175981d49432fcf84174065be562b2e4977af366fdd2cce`; its detailed result is `independent-runtime-data.json`, SHA256 `4c2b3ab325c73d561abdbd36b036b9b7ac877c424cc991750b16e6956ffc48a8`. It reused the unchanged prior independent image/memory helpers, SHA256 `2df6206f92c46f29ee74bd3f083a839f494838a562f6110ab47f481347db5c28`; exact bytes are preserved here as `independent-runtime-helpers.snapshot.py`, with provenance in `independent-runtime-helper-identity.json`. This is analysis of existing artifacts, not another Rust build, GUI run or benchmark.

## Runtime and visual findings

I independently reconstructed complete post-read callback coverage, request/receipt association, three source-task slots, cancellation return, stale discard and retry ordering for both sources. Actual counts and code-section wall intervals are:

| Source / task | Eligible distinct callbacks | Decode + display preparation, ms | Callback P95 / maximum, ms |
|---|---:|---:|---:|
| grid / first | 63 | 563.833 | 0.511 / 39.180 |
| grid / cancelled | 63 | 520.856 | 0.432 / 8.345 |
| grid / retry | 61 | 510.294 | 0.322 / 0.364 |
| noise / first | 31 | 306.588 | 0.344 / 36.850 |
| noise / cancelled | 36 | 296.542 | 0.589 / 11.704 |
| noise / retry | 35 | 289.838 | 0.346 / 0.371 |

Coverage and cancellation endpoints lie in `read_complete..cpu_prepared`; file I/O, discarded/repeated frames and later waiting do not fill the quota. All observed callback spans, including controlled artifact work, have larger maxima: **347.730 ms grid / 271.491 ms noise**. These remain visible in the metrics. Instrumentation ends at its final captured sample before buffered row formatting/return; it excludes subsequent integration and GPU/physical presentation. These are captured wall intervals around code, not OS per-thread CPU utilization or an assertion that every callback meets a product latency budget.

Cancellation returns in about **0.098 ms grid / 5.878 ms noise**, but observed stale-result discard follows cancellation by about **508.665 / 291.446 ms**. This revokes result ownership; it does not stop CPU work. The raw flow and bound driver checks preserve source/stamp/PNG before subsequent editing and protect the new edit/status after the late result. Retry occurs only after settled slot drainage. Independent current tests cover canceled-slot retention, late success/failure, key-owned cleanup and final drain. J raw coverage independently recomputes to **220/222/220 grid**, **136/142/140 noise**, all above its 30-frame requirement, with canceled results not promoted and successful promotion after a matching drawn frame.

The source cases' OS lifetime working-set peaks were **592,932,864 bytes grid / 521,760,768 bytes noise**. I recomputed sample maxima and OS-peak series from all 13 raw memory streams and checked exit/guard/query-error state. The 20-repeat performance case peaks at **692,441,088 bytes**; sampled maximum working set is **691,474,432 bytes**. Sampling may miss peaks; OS process working set excludes separate GPU memory. Retained artifacts, pending allocations, GC/allocator behavior and timing prevent a steady-state/no-leak claim. The 2 GiB/120-second guards are experiment protection, not a frozen product budget.

I actually viewed **all six current source GPU images**. Both busy images retain the old T04 editing canvas, Chinese opening/cancel indication and edit handles. First/retry show the accepted grid/noise source, opaque encoded review masks without edit handles, changed retry region coordinates, and the corrected status: controlled synthetic output verified, **system clipboard not used**. I also viewed current ordinary error/discard images; the other six ordinary states are byte-identical to the six previously actually inspected J images. Chinese recovery actions remain present; English diagnostic detail is not claimed to be fully localized. Wide-native and forced-512 central stage-4 images were separately viewed and independently compared pixel-by-pixel; original-pixel detail and tiling evidence are consistent. I did not individually re-view all 40 performance images or all six J images; their hashes, raw linkage and selected exact pixel regions were checked. These are actual GPU readbacks, not physical-screen/human tests. Diagnostic trace mode intentionally disables ordinary OS delivery buttons.

## Current code and complete-run scope

Static review confirms the private non-Clone Core-validated candidate, consume-without-UI-redecode acceptance, atomic domain-id overflow, old-source/review/history revocation, sealed source/display/cap envelope, Arc-key authority, final channel drain and slot retention until thread exit. Source and review/AwaitingDraw remain mutually exclusive. Current cap is checked again at final discard confirmation. OS-reader and Copy/Save entry/callback guards precede external consumers; confirmed Close remains terminal across repeated Close and keyboard input. No new dependency or unsafe bypass was introduced. This is a bounded assessment, not an exhaustive security proof.

I read and hashed the actual implementation full-run raw outputs: root **72**, ordinary Native **82 + bridge 4**, feature Native **89 + bridge 4**, with zero failed/ignored/filtered cases. Current critical source/Close tests occur as named successes. The ordinary harness's 19 PNG-checker tests, full eight-state image records, existing large artifact and complete J/performance chains remain intact. Five actual frozen golden results exactly match the frozen contract's five RGBA hashes; reported V1 producer bytes and retained snapshots match. Those full Rust/GUI/golden executions were performed by root and inspected here; they were **not independently rerun wholesale**. K does not include a fresh browser/WASM acceptance run; unchanged Core/V1 browser inputs retain the separate historical I evidence only.

## Findings repaired and evidence delivery

The record preserves failures rather than selecting only GREEN: the independent final-confirm-cap defect and parent key-cleanup/clipboard-context findings have real RED/GREEN and current test coverage; the first complete ordinary 65/12 failure retained its original assertions after real async waiting adaptation. The independent nested egui-lock finding was repaired before any GUI execution. The independent file-I/O-as-CPU method finding has 15 passing method tests and both original false-acceptance counterexamples now reject. The first pilot failed its reviewed/view check. A subsequent machine-green full run had misleading system-clipboard status, found by root's visual review and missed by my earlier static review; its exact source/run is preserved, and R2 reran the corrected complete artifact. None of these historical GREEN/failed artifacts is substituted for current identity.

I additionally found that the draft delivery checker could omit an entire documented screenshot and its metadata together. The repair now derives exactly six source plus four ordinary copies from actual raw report paths and verifies their complete names/path/hash set. The original false-acceptance RED is retained. I independently replayed the current receipt validators against the retained fixtures: **3 positive controls accepted / 14 negative controls rejected**, including whole-root/file omission, empty/duplicate/malformed checksums, complete visual omission and redirected visual source. All ten current visual copies match the exact required raw source paths/bytes. Metrics/table distributions were recomputed and matched. The candidate **1,453-file** raw manifest was checked against its complete declared roots/explicit inventory and actual bytes at that draft point; later independent outputs/final review still require regeneration into the final inventory.

Method evidence is `independent-delivery-method-data.json`, SHA256 `c5eab27259e7a449384f8b79feb56a5aad89c748977f1099cb4f81475bfe4344`; current checker SHA256 `630fe9c794d0e97f4fe36afc98c16c95c4f0e8f3911cf512ddb6025d962fff7f`; inspected control report SHA256 `3fe1d28006617db2d20754d3bb9c06b2f5ece8ca2a580562448523d2970a6bd7`.

`build_docs.py` preserves exact sources/producers/through-Step3 workflow; `refresh_docs.py` refuses silently changed prior independent copies and rebuilds complete inventories; `check_delivery.py` distinguishes source/report/manifest identity reconciliation from executing new tests or measurements. These are appropriate for docs-only closeout. This review does **not** claim that a final document checksum containing this not-yet-written report has already passed. After copying this report and syncing entry/status documents, regenerate manifests and perform a separate exact final checksum/identity recheck. No new performance run should be claimed from that reconciliation.

No product source, lock, fixture, workflow or earlier report was changed by the reviewer. Only new ignored independent scripts, synthetic test outputs, logs, snapshots and this report were written. No private input or real OS clipboard was read or written, and no native GUI, privilege change, commit/push or external publication was performed by the reviewer. Real dialogs/drop/clipboard and cross-app output, physical DPI/screens, Narrator/IME, human tasks, Win11/8GB, G04 budgets, security audit, package and release remain open. These limits are conditions of this local **ACCEPT_WITH_LIMITS**, not missing items silently marked complete.
