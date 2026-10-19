# P03-K — current-code final static candidate review

Verdict: STATIC_CANDIDATE_NO_NEW_BLOCKER_RUNTIME_REVIEW_PENDING

This is the requested static candidate checkpoint while the genuine full Step3 command is still running. It is **not** final batch acceptance. I did not run Cargo, tests, GUI, clipboard/station APIs, benchmarks, image decodes/pixel oracles or background tasks in this turn. Only reads, textual diff, SHA256/JSON identity checks and new ignored static-review evidence were performed. The parent's interim callback counts are not used as verified results here.

## Current identity and review continuity

Verified branch `main`, HEAD `94f7f419aee91faf6f846349d40a897d0d72d7d0`. The current source map has **84 inputs**. Compared with the exact independently tested Step2 80-input map, only these seven paths differ:

- Added: `poc/native-egui/examples/source_window.rs`, `poc/native-egui/examples/support/source_driver.rs`, `tests/native_oracle/run_source_checks.py`, `tests/performance/test_source_method.py`.
- Changed: `poc/native-egui/src/app.rs`, `poc/native-egui/src/source_task.rs`, `poc/native-egui/examples/support/performance_driver.rs`.

There are no removed inputs. All eight inputs captured by the last independent method recheck remain byte-identical. Core, Editor, display/review-task implementation, platform code, manifests/locks, fixtures, existing J driver and remaining mapped inputs have not drifted since the Step2 boundary review. The four `PROTECTED` artifacts also match their fixed hashes. This is a current-file/source-map comparison, not a claim that the entire dirty tree equals Git HEAD.

Full current source map and comparison results are in `FINAL_STATIC_SOURCE_MAP.json`, SHA256 `a2bc583a8286531a3afe8cab49b9c50c7af7ed8a198a73570f49872a3b8c5af4`. Applying the runner's exact sorted-JSON source-map formula yields candidate identity `aa277ed50e862194273a8e796e56c19a5c09f5941df8d9a6daa0018600fcff0e`; this formula calculation does **not** prove any EXE was built or executed with that identity.

| Current reviewed file | SHA256 |
|---|---|
| Editor `lib.rs` | `c903c26c9c2e61968043a6fce79b12e2719cdca58201b0323c5232d9c685616e` |
| Native `src/app.rs` | `a2332463ea0e0fe7b611c7e1b38d06701ffc60a6491f5635c268b6afe7fc4f04` |
| Native `src/source_task.rs` | `81d8762ac5e45b2467cfaf772167b772ef9083eb7a7630f081a3db745dd6ed0f` |
| `examples/source_window.rs` | `71d07694c5a81637ff4edcae6da931b8ce414f8e785f83dc84ec766f44b92d89` |
| `examples/support/source_driver.rs` | `335af4c7b806045818de793642f7c3d73669a58db234c774ed66873ff5e530a2` |
| `examples/support/performance_driver.rs` | `441128a6d88ef1dc10b1a0037e91fe0bddd5d4cd7865b77a99faf7e177ed850e` |
| `tests/native_oracle/run_source_checks.py` | `155666e00ad562450ad9235f2fe21da838297151c6ef551233e67c98f773a318` |
| `tests/performance/test_source_method.py` | `ea3f0ffb186d522c7cc4c43177c7dd87de6a400c9d8386f224fda4f340f3f84a` |

Native example paths above are relative to `poc/native-egui/`. Earlier domain/Native/method reviews retain their own exact identities and actual executions; they are not rewritten as tests of later code.

## Static conclusions

The domain candidate remains private, non-Clone and fully Core-validated at construction, with immutable source bytes/dimensions. Acceptance consumes the candidate without a second UI decode. Checked document-id growth precedes mutation; successful replacement revokes old history, gesture, output and review/delivery authority. Existing synchronous wrappers and J source-Arc review identity remain intact. This is unchanged from the independently executed domain review.

The ordinary source worker still owns a capacity-one channel and one real thread handle. Received does not mean finished; canceled/stale work retains the slot until thread exit, then the final drain preserves send-after-Empty results. Current key cleanup is conditional on ownership. Source and review/AwaitingDraw starts reject overlap before external source reads. File I/O/decode/display preparation use the worker; bounded clipboard byte acquisition and the OS picker remain explicitly synchronous. Cancel/error do not replace the old document or discard its gesture. Candidate arrival checks current edits, and both receive and final modal-confirm consumption check cap. Terminal Close and Copy/Save entry/callback guards remain in place.

I compared current App/source-task diffs against the preserved Step2 source snapshot. The new branches add feature-only phase telemetry and make an absent current key an explicit stale-return path; they do not grant an alternate acceptance or delivery route. Numeric diagnostic request IDs supplement the private Arc key and are not authorization. Events are buffered only when feature and active synthetic context are present. Captured read/decode/preparation Instants can be emitted after the work; frame/pass context must not be inferred from emission order as a timestamp for the original phase. No new dependency, unsafe block or thread pool was introduced in these mapped differences.

The synthetic driver uses the ordinary File-worker entry with exact fixed grid/noise source files. It starts on the old bundled source, waits for current review completion, checks cancel before making subsequent edits, waits for stale-tail drainage, retries and generates new whole-image PNG and controlled Copy outputs. It does not add artificial delay to obtain busy frames. The nested egui-lock defect previously identified is repaired by taking pixels-per-point outside `data_mut`. This is static confirmation, not a deadlock stress test.

Method checks now restrict CPU-coverage/cancel/busy-request intervals to post-read decode/preparation (`read_complete..cpu_prepared`), reject duplicate/missing phases and invalid phase/frame time values, and exclude repeated/discarded/outside-interval callbacks. The reviewer previously reran 15 pure-Python cases and both original counterexamples against these exact method hashes; no further method change has occurred. That evidence remains a method test, not proof the live worker meets its quota.

GPU requests and receipts separately bind reviewed-artifact state and currently displayed review view, request key, document/revision and a unique later callback; Rust also compares the visual snapshot before accepting a screenshot. Busy Fit is preview-only. Native first/retry pixels, generated PNG structure/content and controlled Copy RGBA must be checked from actual final artifacts. Save and `gpu-*` output names are distinct. Driver input stripping and fixed trace validation preserve the synthetic-only scope; no real clipboard consumer or private input path is exercised.

The old visual flow waits for actual source completion before capturing error/discard states. Repeat benchmarking now records a request-only trigger, waits for slot completion and a genuine pending candidate, confirms it, checks exact source/document change, and only then continues review. The large-artifact source/paste rejection helper drives real App frames before checking unchanged old output and stale-consumer rejection. Existing final pixel/format/credential assertions remain. The full source runner still calls the existing J/full regression chain and applies current suite-count checks rather than substituting the small method suite or a grid pilot for complete regression.

No new static blocker was found. This is a bounded code assessment, not an exhaustive security certification or concurrency proof.

## Runtime review still required after all active measurements terminate

1. Bind the final **non-pilot** report, copied release EXE, build ID, all 84 source inputs, HEAD/branch, protected artifacts and command/stdout/stderr identities. Check nested J/fix/ordinary/large/window/golden reports against their actual artifacts and code identities. Preserve all earlier genuine failures and distinguish transition receipts from complete raw logs.
2. Independently rerun the selected current Native source/accept/Close/late-result tests and current feature-specific visual/repeat controls only after root confirms no measurement contention. Parse actual named results/counts. Earlier independent 21 tests used the earlier Step2 App/source-task hashes and must not be relabeled as current execution.
3. Independently reconstruct all three source jobs for **both grid and noise**: real File read and decode/preparation phases, at least two eligible distinct complete instrumented callbacks for each required successful task, cancellation and return after read/before CPU preparation end, actual stale discard without install/accept, settled slot release before retry, and exact key association. Recompute maxima/distributions from raw data rather than copying the summary. Distinguish measured instrumented span from the callback formatting/return tail and physical latency.
4. Inspect actual busy/first/retry images for both fixtures, including the Chinese busy/recovery state, editing versus opaque review view, current document/mask placement and moved retry. Validate request/receipt/snapshot/metric linkage. Re-run appropriate whole-image PNG oracles and full controlled-Copy RGBA comparisons, explicitly using the normalized-source option only for the large synthetic JPEG contract. Verify first/retry outputs differ for the intended edit. Busy Fit must not be counted as original-pixel proof.
5. Check cancellation's preserved source/old PNG before editing, later edit's stamp/status survival and slot drainage against actual terminal flow. Verify the accepted retry's fresh review ticket, actual encoded PNG, positive clipped draw and later promotion remain consistent with the unchanged J boundary; no screenshot or telemetry marker alone may confer delivery authority.
6. Inspect current root, normal/feature Native and bridge, 19 PNG-oracle controls, J grid/noise, ordinary eight-state GPU, existing eight performance cases/20 repeated imports, large PNG and five frozen-golden results. Confirm new imports are awaited rather than skipped, and keep request-only, install, full completion, App span, GPU readback and process-wide memory metrics separate. Do not infer causal speedup or a frozen G04 budget from this change.
7. Reconcile final documentation, artifact manifests/checksums and scope claims after raw runtime review. Treat later identity-only receipts as identity reconciliation, not benchmark/test reruns. Preserve human, physical screen/DPI/Narrator, real cross-application clipboard, Win11/8GB, release/security/package and full-product gaps.

No final live-report numbers are certified here. The genuine Step3 command is still in progress and runtime/independent current-code execution remain pending. Product status remains IN_PROGRESS.
