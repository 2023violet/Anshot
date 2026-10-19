# P03-K Step2 — independent Native boundary review

Verdict: ACCEPT_WITH_LIMITS_STATIC_HEADLESS_BOUNDARY

The earlier independent modal-cap blocker is resolved in the exact code below. No remaining blocking issue was found in the reviewed source-intake ownership/state boundary. This accepts the bounded Step2 static/headless scope only. It does not accept the new real-window source driver, GPU output, worker timing, complete K batch, or full product. Earlier BLOCKED and R1 static reports remain preserved.

## Independent execution

I actually ran six sequential ordinary Native test commands using explicit `RUSTC` and `RUSTDOC` from `C:/Users/16429/.rustup/toolchains/stable-x86_64-pc-windows-msvc/bin`. Both report version `1.98.1 (48a229cea 2026-09-01)`; that directory was prepended to PATH. No GUI, real clipboard/station operation or benchmark was invoked.

Each command used this exact argument prefix, followed by the filter in the table:

```text
C:/Users/16429/.rustup/toolchains/stable-x86_64-pc-windows-msvc/bin/cargo.exe test --locked --manifest-path poc/native-egui/Cargo.toml --target-dir poc/native-egui/target --bin shareready-native-egui-poc <filter>
```

| Filter | Actual passing tests | Exit |
|---|---:|---:|
| `source_task::tests` | 4 | 0 |
| `source_intake_` | 10 | 0 |
| `source_accept_` | 3 | 0 |
| `async_confirmed_close_` | 2 | 0 |
| `async_modal_defers_result_and_discard_rejects_old_messages` | 1 | 0 |
| `close_cancel_retains_history_and_explicit_discard_emits_one_close` | 1 | 0 |

All **21 distinct named tests** actually ran and passed, with no ignored tests or failures. The parser checked actual named successes and summary counts, not merely exit status. Full per-command stdout/stderr and SHA256 are retained in `target/p03-k-evidence/independent-native-boundary-20261019T003153930850Z.json` and its twelve adjacent stream files. I subsequently rehashed all twelve streams against the JSON successfully.

- Independent execution JSON SHA256: `a423b21897841d067dc97c0993ff7eb9b6709397d913b9fad40823886a2213a6`.
- Runner `run_reviewer_native_boundary.py` SHA256: `f1afb40ec0d58b129942d08ace7c002e1b994ecc90e98d10584b791f0a651aa6`.
- All **80 product inputs** in the current feature-run source map were hashed before and after these tests and remained equal. They also exactly matched `native-feature-r1.json.source_after`. This includes the new source-task module; it is not merely a Git HEAD comparison.

## Code and behavior assessed

The worker owns the bounded input and privately binds its `SourceCandidate`, same-source CPU `PreparedDisplay`, original cap, label and live Arc request key. Candidate and display cannot be re-paired by the consumer. A result received from the capacity-one channel remains in its slot until the actual worker finishes. The final drain test now deterministically observes Empty, releases the worker to send and exit, and invokes production `finish_poll`, preserving its queued success. No live worker is joined before `is_finished`.

Source/review start guards reject overlap, including canceled work still finishing and review AwaitingDraw. File reads take place in the worker; clipboard acquisition remains a synchronous bounded OS boundary behind close/modal/trace/busy checks. Current errors preserve the old Editor/output/gesture; canceled and late results cannot replace it. Cleanup clears a key only if it belongs to the completed task. Clipboard origin preserves the appropriate recovery message without becoming an identity credential.

Candidate arrival checks the then-current undo/redo/gesture state and uses the existing discard modal. CPU preparation does not change the old document. The receive-time cap check and **final consumption-time cap check after confirmation** both precede Editor replacement. The independently executed `source_accept_rechecks_cap_after_discard_confirmation` proves that cap change at this later boundary retains the old stamp, masks, PNG and undo history. The old-error/different-current-key and paste-error-context tests also actually passed in this exact current build.

Copy/Save entry and callback paths reject source-busy before any OS consumer; the independent prequeued-callback test exercises a previously issued Copy ticket and Save ticket while intake is live, proves the Copy closure is not invoked, verifies no Save file is created and the existing PNG survives. The test uses a new ignored synthetic destination, not a user file.

The clean-close intake test exercises repeated real `ViewportEvent::Close` plus keyboard N and rejects post-close input/readers. The independently rerun J cases preserve the terminal Close latch, reject late running/AwaitingDraw review results, retain canceled-close history, and defer results across modal state. The stale-review replacement defense deliberately injects a Core-validated test candidate; it does not bypass ordinary source/review mutual exclusion in production. Headless `App.show` frames drive actual worker completion; they do not mark jobs complete directly or constitute GPU presentation.

## Counterexamples and complete regression evidence

I read and rehashed both stdout/stderr for the genuine R1 three-case RED/GREEN receipts. The RED genuinely ran all three cases and failed (0/3), including the cap mismatch committing document 2 instead of retaining document 1; GREEN passed the same three (3/0). Their JSON identities remain `2aa46365158ff47e39d785eebea1a3c0f6b51aa8b7d4c8d09ba5884a696d3899` and `515a6650f2ff958a0fe7decb60e5c106b61b5fdb6a0b251eacc18a21c2172306`. These historical runs used their own recorded code identities; today's independent execution supplies the current-code confirmation. The actual keyboard-after-Close RED (0/1) and earlier whole-regression 65/12 failure remain retained, not reclassified as passes.

The current implementer feature run `native-feature-r1` actually reports **87 app + 4 bridge passed**, no failed/ignored/filtered tests, exit 0. I read the raw summaries, recomputed both stream hashes, checked before/after source-map equality and matched all current 80 inputs. Its JSON SHA256 is `0c937510e48ae5fa3bc82d36085264c0df4ce32704796ec11009829749f29c53`. This is inspected implementer execution, not an independent feature-suite rerun.

The ordinary genuine Step2 receipt records the current workspace command exiting 0 and Step2 done. The earlier retained `native-full-r1` raw is **80 app + 4 bridge**, preceding the additional prequeued-callback test; it is not relabeled 81. Initial inspection found that `native-green.json` was only a workflow transition receipt and its warning tail omitted the application summary. The implementer then executed and retained a fresh complete ordinary run, `native-full-frozen`, without changing product inputs. I subsequently read its actual stdout: **81 app + 4 bridge passed**, with no failed/ignored/filtered tests, exit 0, 47.453 seconds for the command. All 21 cases independently selected above also occur as named successes in that whole-run raw output. This later log resolves the evidence-retention gap; it remains an implementer whole-suite execution, not an independent reviewer whole-suite rerun.

`native-full-frozen.json` SHA256 is `9bd1ea7397aa4a2b9bda0e0183186a97416d6f7d09cfc0e3bd9fc15f14c435f5`; I independently matched its stdout SHA `a1fb26a530f9cbec26666464243dcde5a4f18c2040f830496d2b7bf0a06f4bf4` and stderr SHA `faa5524b18507b92cf5d43a82090f0e0f010ae8d9eae7d23334755b35aa5fd22`. Its source-before and source-after maps equal each other, today's independent run's 80-input map, and the current files. Both ordinary and feature full configurations therefore include the current boundary repairs and additional prequeued-delivery case.

I also independently hashed every copied file in `target/p03-k-evidence/native-boundary-sources-20261019T003406488116Z/files`: all **80** exactly match the independent test/current source map. Its `report.json` SHA256 is `90a24ad983a0490e4354e17caae3662c29520ef91ee960a792e42689a83e0152`. This snapshot preserves the exact reviewed source after later diagnostic development; checking it is source identity evidence, not another test, EXE rebuild or GPU run.

## Exact code identity and limits

Main / HEAD `94f7f419aee91faf6f846349d40a897d0d72d7d0`.

| File | SHA256 |
|---|---|
| `poc/native-egui/src/app.rs` | `b9ad046b058bdefea6f498a010ce2085a181c48726ab994093bb72b4e4464ddf` |
| `poc/native-egui/src/source_task.rs` | `eb00f886615136e9b846de998b45a30ebd31097bb74679847d9a5ed9dccc94f2` |
| `poc/native-egui/src/display.rs` | `21338c4f16825855c8f723e4c04b0eea57b3bdc568456e4a77bec535bc7ea063` |
| `poc/native-egui/src/review_task.rs` | `b1bf49bac80dfe51ee1d2a40b6388d65e8490d22676e16f9d319a4e52f0b2f7c` |
| `crates/shareready-editor/src/lib.rs` | `c903c26c9c2e61968043a6fce79b12e2719cdca58201b0323c5232d9c685616e` |
| Root `Cargo.lock` | `574836ca69c075b9844928fcc320d89ce2742001ae4f0596b67558b6ff1f5266` |
| Native `Cargo.lock` | `b62d2d0993d34cc363317faa89d0cc56fcdd3610f3ffef57b16ae3e910a35038` |

No product source, lock, fixture, workflow or older review was modified by the reviewer. This review wrote only new ignored reviewer scripts/logs/JSON and this report. Existing ordinary example dead-code warnings are not hidden as errors or independently fixed. Input/PNG limits and the separate earlier Task1 domain conclusion remain unchanged.

New real GPU source-busy/accepted screenshots, actual worker timing/callback coverage, cancellation/retry full-image products and complete Task3 machine regression have **not** been performed or accepted by this report. Physical screen presentation, real OS clipboard/cross-application use, human task/accessibility testing, Win11/8GB and full-product completion remain unverified. The project remains in progress.
