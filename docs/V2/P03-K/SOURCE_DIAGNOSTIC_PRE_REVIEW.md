# P03-K Task3 — independent diagnostic pre-review

Verdict: BLOCKED_CPU_INTERVAL_METHOD_PENDING_RECHECK

Only static code inspection and the explicitly permitted pure-Python method tests were performed. No Cargo, GUI, worker/window benchmark, real OS clipboard/station API, privilege or external operation was run by this reviewer. Only new ignored reviewer evidence was written. This is not acceptance of a pilot, Task3, K or the full product.

## Findings

**Blocking method counterexample: file I/O can supply the claimed CPU coverage and cancellation interval.** `source_task.rs` records `worker_start` before `SourceInput::File` calls the bounded `read_source`; `read_complete` is captured after that read, and `cpu_prepared` after both decodes and display preparation. `source_jobs` nevertheless bounds both counted callbacks and the cancellation check by `worker_start..cpu_prepared`. Two callbacks wholly before `read_complete`, or a cancellation return before `read_complete`, are accepted as CPU-period evidence. This demonstrates worker responsiveness during potentially blocked file I/O, not necessarily during decode/display CPU preparation. It contradicts a claim of coverage specifically during CPU work.

I reproduced both false acceptances with tiny handwritten Python records, without running the product or any benchmark:

- First request: move `read_complete` to 1400 ms and `decoded` to 1500 ms while the two callback intervals remain 1100–1103 and 1300–1303 ms. Both are before CPU decode starts; the checker still accepts and reports `[2, 2, 2]` busy-frame counts.
- Canceled request: move `read_complete` to 2500 ms and `decoded` to 2600 ms while cancellation remains 2400–2410 ms. The checker still accepts cancellation returning before CPU decode starts.

Minimal fix: retain worker-start/read timing for end-to-end reporting, but require counted CPU callbacks, the busy GPU request if labeled CPU-period evidence, and cancel/cancel-return to lie in `read_complete..cpu_prepared` (or add a dedicated decode-start capture). Add both counterexamples as actual rejecting method tests before repairing the validator. If coverage is insufficient in a real run, preserve failure and replan; do not add delay or use I/O frames to fill the CPU quota. The root acknowledged the finding and will wait for the already-running pilot to terminate before changing its frozen inputs; a structural old-pilot success must not be upgraded to this criterion passing.

**Nested egui lock found and statically repaired before GUI execution.** The initially read `source_driver::center` called `ctx.pixels_per_point()` from inside a `ctx.data_mut` closure, repeating the earlier J diagnostic deadlock pattern. I reported this immediately. The current reviewed source takes `ppp` outside the closure and uses that local value inside. Root states no GUI had run before the repair. I inspected the repaired code but did not run a GUI reproducer or certify runtime absence of a deadlock. The issue was diagnostic-only, not an ordinary product path.

## Actual independent Python execution

Command, using the bundled Python interpreter:

```text
C:/Users/16429/.cache/codex-runtimes/codex-primary-runtime/dependencies/python/python.exe -B -m unittest discover -s tests/performance -p test_source_method.py -v
```

All **11 existing handwritten method tests passed**, exit 0, `Ran 11 tests in 0.006s`. The two additional independent records above were then passed to the current `source_jobs` and both were accepted, preserving the method counterexample rather than treating the green existing suite as sufficient.

- Record with exact command, all synthetic counterexample rows, outcomes and eight reviewed-input before/after hashes: `independent-source-method-20261019T004455720841Z.json`, SHA256 `30dc70ec9c1f7621f3f69cf89fbc42404e7312710ca7917e7fc74210d17c5091`.
- Adjacent stdout is empty, SHA256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`; stderr contains the full 11 named test results, SHA256 `553528a1be391adf83e95647cc0f76ab70309f7b3b6736704c1995564b07c50a`.
- Independent runner `run_source_pre_review_methods.py`, SHA256 `2570e0ebd77ce93158f0e423a89b079498c88dbb97214c61fc231efee2a28d7f`.
- Those eight product/method inputs remained byte-identical throughout this Python execution. The reviewer did not edit them.

## Other assessed boundaries

The source example accepts only `NEW_DIR grid|noise`. It creates a frozen synthetic trace, writes the exact embedded fixture, starts the ordinary App on its bundled old source, and submits a real `SourceInput::File` path through the ordinary source worker. The exact disk fixture is later compared with its embedded source. Private source/clipboard import is disabled by the trace/driver; input events are stripped except screenshot callbacks. I found no artificial sleep, busy wait, padding work, new dependency, private-image read or real clipboard consumer in this driver. Initial normalized-source generation occurs before the measured window flow; it is setup, not background-intake completion.

The feature-only request number is diagnostic metadata; product authorization still uses the private Arc key. Phase Instants are captured around read/decode/display and emitted later as buffered events. `received` may legitimately be later than the first finished observation because of final drain; the validator uses settled `max(received, finished_observed)` before installation. Ordinary modes do not emit these telemetry rows. Instant elapsed values are wall duration around code and may include scheduling/locking; they are not OS per-thread CPU consumption measurements.

Cancellation checks key revocation and unchanged source/stamp/PNG before making a real subsequent edit. After the canceled tail is drained it checks that no pending candidate, old output or status pollution replaced that edit, then retries. Retry changes a mask position and produces a new reviewed artifact. These are checks in the driver, not reviewer-observed runtime outcomes yet.

Save uses ordinary `save_choice` and new-file semantics; controlled Copy uses ordinary `copy_choice` and the generated-PNG pixel helper to write a full RGBA buffer without OS clipboard access. The runner checks old/first/retry whole-image PNG oracles and full Copy RGBA equality and requires first/retry RGBA to differ. The noise fixture must remain below the source ceiling while its generated PNG exceeds 25 MiB but stays within 128 MiB; Rust-normalized JPEG source plus the independent compositor is honestly distinct from independent JPEG decoding. GPU filenames use `gpu-*`, avoiding collisions with actual Save PNGs.

Screenshot requests bind the source request number, document/revision, reviewed/view/busy state and captured App visual snapshot; callbacks require the expected viewport/name and unchanged snapshot before writing. The phase driver serializes requests and waits for current review completion. Runner checks request/key and accepted-document association plus a unique nondiscarded metric frame. The busy Fit image is only downsampled preview evidence (`native_pixels` explicitly returns without a native-pixel assertion for Fit); first/retry 100% crops and full output oracles carry the pixel checks. No screenshot here proves physical display or human input/latency. A later final review must independently verify raw callback/request/received streams and images, not merely an `ok` flag.

Performance repeat adaptation waits for actual source-slot release and a valid pending candidate, resolves the discard confirmation, verifies exact source and document increment, then continues its existing review/output stages. Trigger timing is explicitly request-only. Headless large-source rejection now waits for real App frames before asserting the old stamp/PNG and no pending candidate; stale consumer checks remain. Ordinary visual capture waits until no source task remains before requesting the error/discard state screenshot. These adaptations preserve completion assertions rather than marking work done directly.

Full source execution invokes the existing J/full runner, retains current root/ordinary/feature count checks and the existing ordinary/large/performance/golden paths. Pilot deliberately excludes noise and those regression commands. Static inspection does not establish any of those commands have now passed. The `source_map` globs include the new example, support module, runner and method tests. Preserve the exact release EXE and source identity before/after execution, as the runner intends.

## Reviewed identity

The hashes below refer to the post-center-lock-fix inputs used by the independent Python execution, while the CPU-interval validator is still the old form:

| File | SHA256 |
|---|---|
| `examples/source_window.rs` | `71d07694c5a81637ff4edcae6da931b8ce414f8e785f83dc84ec766f44b92d89` |
| `examples/support/source_driver.rs` | `335af4c7b806045818de793642f7c3d73669a58db234c774ed66873ff5e530a2` |
| `src/source_task.rs` | `81d8762ac5e45b2467cfaf772167b772ef9083eb7a7630f081a3db745dd6ed0f` |
| `src/app.rs` | `a2332463ea0e0fe7b611c7e1b38d06701ffc60a6491f5635c268b6afe7fc4f04` |
| `examples/support/performance_driver.rs` | `441128a6d88ef1dc10b1a0037e91fe0bddd5d4cd7865b77a99faf7e177ed850e` |
| `tests/native_oracle/run_source_checks.py` | `f921bd9bf4f64d196cb32b762033d909277f9026651f5a7e90705a1a310c12e2` |
| `tests/performance/test_source_method.py` | `d89279e2d98a618b40eaf2234cfaab8c4a8cf1ff46e0fd1c5e8a8d1160a97547` |

The first five paths are relative to `poc/native-egui/`. Source-map implementation remains SHA `c09009df59ac166f4bb003230fbb27d648cfb0cc196e9f008b62817280d5f797`. Step2's prior exact 80-input static/headless acceptance is separate and does not automatically accept these subsequent diagnostic changes.

One optional evidence improvement is an explicit wall-clock/monotonic-origin anchor if phase-aligned RSS analysis is later claimed; the current source log supports relative window-flow timings and process-wide memory, not per-phase RSS attribution by itself. Likewise retain inter-job settled/trigger ordering and all failure histories when reviewing the real stream. Neither suggestion authorizes changing the acceptance quota or product limits.

The real pilot was not reviewed here and may still be running. New exact-artifact tests, real GPU frames, full outputs and repaired-method runtime evidence remain pending. Physical screen, human usability/accessibility, cross-app clipboard, Win11/8GB, G04 budget and full-product completion remain unverified.
