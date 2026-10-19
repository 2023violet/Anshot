# P03-K source diagnostic method — independent R1 recheck

Verdict: METHOD_FIX_ACCEPT_WITH_LIMITS_RUNTIME_PENDING

The CPU-interval method blocker from `SOURCE_DIAGNOSTIC_PRE_REVIEW.md` is resolved for the exact Python revisions below. This is a method/static recheck, not acceptance of the current EXE, real-window evidence, Task3, K or the complete product. Earlier failed evidence and the original blocked review remain preserved.

I actually reran, using the bundled interpreter:

```text
C:/Users/16429/.cache/codex-runtimes/codex-primary-runtime/dependencies/python/python.exe -B -m unittest discover -s tests/performance -p test_source_method.py -v
```

**All 15 named method tests passed**, exit 0, `Ran 15 tests in 0.005s`. These are handwritten synthetic records and assertions, not measured UI/worker/GPU evidence. The four added cases reject file-read-only callback coverage, reject cancellation returning before read completion, allow the valid busy editing view that still retains an old reviewed PNG, and reject GPU receipt document/revision/reviewed/view mismatches (four mismatch subcases in that last test).

I also replayed my two original independent counterexamples, unchanged from the blocked review. Both previously returned success with counts `[2, 2, 2]`; now:

| Original independent counterexample | Actual current result |
|---|---|
| Both counted callbacks before first request's `read_complete` | Rejected: `actual CPU interval lacks two distinct complete instrumented callbacks` |
| Canceled request's `cancel_return` before `read_complete` | Rejected: `cancel did not return during actual CPU work` |

Exact command, full counterexample rows, all eight before/after input hashes and outcomes are retained in `independent-source-method-20261019T004718845675Z.json`, SHA256 `108510a3aac7dc41f03f14cd4505b70194184f38bac7848c3bdbee2ccc08c621`. Its adjacent stdout is empty (`e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`); stderr contains all 15 named results (`50f40845dbe7f6bf59e8a9972b40cba91c1a9e72decd26b840d5d1ed42b456d7`). The independent runner remains `run_source_pre_review_methods.py`, SHA256 `2570e0ebd77ce93158f0e423a89b079498c88dbb97214c61fc231efee2a28d7f`.

The inspected `source_jobs` now uses `read_complete..cpu_prepared` for counted complete callback intervals and both cancel/cancel-return endpoints. `source_shot` uses that same post-read boundary for the busy GPU request. Worker-start/read timings remain available without being used to fill the CPU quota. Final-drain chronology still permits receive observation after the first finished observation and requires installation after both have settled. Actual captured intervals remain wall duration around decode/preparation code, not OS per-thread CPU utilization.

The screenshot fix correctly separates `reviewed` (the old encoded artifact remains current) from `review_view` (the canvas displays that artifact). A busy edit view may legitimately have `reviewed=true, review_view=false`. Request/trace shot must agree on document, revision and both booleans; accepted review screenshots additionally require the corresponding accepted source document. The pixel checker is now given `shot['review_view']`, so the editing preview is not interpreted as an opaque reviewed canvas. The runner also explicitly requires exactly one later `screenshot_received` for each named request with the same request key and nondecreasing time/frame. Those receipt-list predicates were statically inspected; they are not separately demonstrated by the 15-unit-test count. Rust's unchanged per-request snapshot check remains another guard; final review must still inspect real request/receipt/trace streams and images.

I read the retained implementer method logs: `source-method-r1-red.txt` ends with `Ran 15 tests` and `FAILED (failures=6, errors=1)`; SHA256 `eef5e3bc7d29a1faf18901b704aa5855e3cc9bcade101302ddf3d6ab09bcd870`. `source-method-r1-green.txt` records all 15 passing, SHA256 `106c35b82b892dbe409f820094c07258649e4fb93d870d8cd61e59428f34c47c`. I did not rerun historical broken code. Today's independent execution above confirms current behavior without upgrading the historical failing pilot or relying only on implementer GREEN.

Reviewed current identities:

| File | SHA256 |
|---|---|
| `tests/native_oracle/run_source_checks.py` | `155666e00ad562450ad9235f2fe21da838297151c6ef551233e67c98f773a318` |
| `tests/performance/test_source_method.py` | `ea3f0ffb186d522c7cc4c43177c7dd87de6a400c9d8386f224fda4f340f3f84a` |
| `poc/native-egui/examples/support/source_driver.rs` | `335af4c7b806045818de793642f7c3d73669a58db234c774ed66873ff5e530a2` |
| `poc/native-egui/src/source_task.rs` | `81d8762ac5e45b2467cfaf772167b772ef9083eb7a7630f081a3db745dd6ed0f` |
| `poc/native-egui/src/app.rs` | `a2332463ea0e0fe7b611c7e1b38d06701ffc60a6491f5635c268b6afe7fc4f04` |

All eight inputs captured by the independent runner stayed identical throughout its execution. No additional blocking defect was found in this bounded method revision. No source, lock, fixture, workflow or old report was modified. No Cargo, GUI, clipboard/station API, benchmark, additional product process or heavy image oracle was run; only new ignored independent evidence was written. I did not inspect the first pilot's images/streams in this recheck, so its existence is not credited as current-version validation. Current exact-EXE runtime, GPU/full-pixel outputs, full regressions, physical/human/clipboard/Win11/8GB evidence and product completion remain pending.
