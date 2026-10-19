# P03-K Native source intake — independent static pre-review

Verdict: BLOCKED_PENDING_CAP_RECHECK_AND_REGRESSION

This is a Task2 pre-review, not final acceptance. Reviewer is independent of the implementer. Product sources were read only. No Cargo command, UI, benchmark, OS clipboard/station API, privilege change, or external action was run in this review. Only this new ignored evidence file was written.

## Blocking finding

**Revalidate the source display cap when consuming a confirmed replacement.** In the reviewed `app.rs:27`, the UI-level `PreparedSource` stores candidate/texture/label but no cap. `source_task.rs:55–58` checks the current key and `DisplayImage::from_prepared` checks cap when polling a completed worker. However, `app.rs:112–114` may retain that already-uploaded source in `PendingAction::Replace` while the user considers discarding edits. `resolve_pending` then calls `replace_prepared` (`app.rs:122–130,147`) without another cap check. A cap change during that modal therefore still commits the old-cap candidate; its retained thumbnail may also exceed the new limit. This violates the documented acceptance-time cap boundary. This is a static control-flow finding; I did not execute a reproducer.

Minimal repair: preserve the prepared cap in the sealed UI candidate and check it against `display::texture_limit(ctx)` immediately before `replace_candidate`, including the confirmation path. On mismatch, drop the candidate and preserve the existing Editor/source/history/output; do not compensate by clearing the old document or silently accepting. Add a decisive test: finish a valid intake with existing edits, observe `PendingAction::Replace`, change the cap, confirm discard, and prove old stamp/source/masks/history/current PNG remain unchanged and retry remains possible. Keep the existing receive-time mismatch test as a separate boundary. The implementer acknowledged this finding and plans RED/GREEN; no later fix is accepted by this report.

## Other code observations

- The source worker owns its `SourceInput`, validates via `SourceCandidate::decode`, and builds `PreparedDisplay::source` from that exact candidate's immutable bytes. The worker envelope fields and constructor are private; consumers cannot combine an arbitrary candidate with an unrelated prepared display. `SourceKey` uses a private live `Arc<()>` allocation and `Arc::ptr_eq`, rather than reusable numeric identities.
- The channel capacity is one. A received result does not release the worker slot. `poll` waits for `JoinHandle::is_finished`, joins only after that observation, and performs a final `try_recv` if its first receive had not obtained a result. Canceled work retains `source_task` until completion. Thread spawn failure happens before assigning a new key/task. Panic and disconnect produce errors; stale completion is not installed and stale errors do not overwrite status when the key is absent.
- `start_source` and `start_review` mutually reject the other task, including the review AwaitingDraw state. `load_source` performs no file read before this check; the worker does the bounded file read. `paste_with` checks terminal close, trace mode, modal, source/review busy **before** invoking the supplied OS reader. Toolbar file selection is also disabled while busy. The dialog and bounded clipboard byte acquisition remain synchronous by design.
- Copy/Save entry and callback paths reject source-busy, modal, review-busy, terminal-close and delivery-wait states before the external write/dialog boundary. Current output tickets are checked before consuming callbacks. Accepting a valid candidate clears old Editor delivery/history through the already-reviewed domain operation; a failed worker does not invoke replacement or cancel a gesture. User editing during intake is allowed, and `offer_prepared` checks current edits/gesture at arrival, not merely the state at request time.
- A pending modal defers worker polling/application. Canceling the source removes reception authority but keeps the slot. The clean-document Close branch revokes the source key and latches `close_confirmed`; confirmed Close also revokes review authority. `keyboard_with` now rejects after close, and `show` suppresses ordinary input and disables the UI. This code-level assessment is not physical window-close evidence.
- The implementer separately identified `source_key.take().filter(matches)` as insufficiently key-owned cleanup: a deliberately mismatched live key would also be removed. Current normal single-slot entry paths prevent installing a second task/key while the first exists, so I did not establish an ordinary reachable cross-task failure. A different-current-key regression and conditional cleanup are appropriate hardening before final review; this observation is not credited as my independent discovery.

## Tests and evidence actually inspected

I independently read the raw stdout and JSON and recomputed stdout/stderr SHA256 for these implementer runs; I did **not** execute them myself:

- `source-worker-green-r1`: exit 0, four named worker tests passed (result-before-exit retention, final drain, foreign key/cap rejection, panic/disconnect/invalid source).
- `source-app-green`: exit 0, nine named intake app tests passed (pending start, mutual exclusion/delivery, clean Close, reader guard, cancel preservation, edits at arrival, errors/gesture/stale status, modal deferral, receive-time cap change).
- `source-close-keyboard-red`: exit 101, the real repeated Close plus keyboard-N counterexample failed with revision changing 0 to 1. The reviewed source includes the subsequent keyboard terminal guard. Its presence alone does not replace a passing rerun.
- `native-full-first`: exit 101; **65 passed, 12 failed**, including old synchronous replacement/paste assumptions and the oversized-source helper's former immediate-result expectation. This is a real failing whole-regression state, not acceptance. Waiting for real source completion and preserving original assertions is necessary; specific failures must be resolved and rerun rather than classified as harmless wholesale. Report SHA256: `2e6b61cc4c8a0aad9e36022e2028cb1a2acbe42acd94ac46f1ac64493ccec5a5`.

The worker final-drain test currently establishes successful completion with no previously cached result; unlike J's deterministic split-boundary test, it does not explicitly demonstrate an initial `try_recv == Empty` followed by controlled send/exit. The code contains the required final drain, but strengthening that test would make the exact regression decisive. No new failure is claimed here.

## Exact reviewed identity

Branch `main`; HEAD `94f7f419aee91faf6f846349d40a897d0d72d7d0`. These product hashes were observed during this review, with the same app/source/display/review hashes on the later read:

| File | SHA256 |
|---|---|
| `poc/native-egui/src/app.rs` | `292cb6edf7a2b9b8183dd45ce61db8a3e253fad4397939adb99d78fbe1d0f13a` |
| `poc/native-egui/src/source_task.rs` | `d1aa5933433cf908eeee7e82b036e71e2ec230e1f586e71d135a03a4369f4857` |
| `poc/native-egui/src/display.rs` | `21338c4f16825855c8f723e4c04b0eea57b3bdc568456e4a77bec535bc7ea063` |
| `poc/native-egui/src/review_task.rs` | `b1bf49bac80dfe51ee1d2a40b6388d65e8490d22676e16f9d319a4e52f0b2f7c` |
| `poc/native-egui/src/input.rs` | `cf505a1ccc39086feb1e7ac0b9038e95c9037708c08922616b5dd3037d5ba4fa` |
| `crates/shareready-editor/src/lib.rs` | `c903c26c9c2e61968043a6fce79b12e2719cdca58201b0323c5232d9c685616e` |
| `docs/V2/P03-K/DESIGN.md` | `cb06d2869aa476036ee34195b5358755e8c2231e473d6cd6b461dca62dcd31bc` |
| `docs/V2/P03-K/IMPLEMENTATION_PLAN.md` | `3bf27e2dc65c7348e2ed84283e0ae162b4c66870d0d2955679304bd43bf36dd8` |
| `Cargo.lock` | `574836ca69c075b9844928fcc320d89ce2742001ae4f0596b67558b6ff1f5266` |
| `poc/native-egui/Cargo.lock` | `b62d2d0993d34cc363317faa89d0cc56fcdd3610f3ffef57b16ae3e910a35038` |

## Limits and next acceptance boundary

This review used local text search/reads, Git HEAD/status reads, and Python standard-library SHA256/JSON checks only. No independent Native test was run while the implementer used the shared build target. The prior independent domain result remains separately scoped to Task1. Source input remains 25 MiB / 24 MP; DIB conversion still has the existing 25 MiB normalized-PNG ceiling (`core/src/clipboard.rs:56`), so high-entropy DIBs may be rejected without making this a new format expansion. Generated PNG uses its separate unchanged boundary.

The source diagnostic driver/new runner, real GPU busy/accepted images, actual CPU interval/callback coverage, cancellation/retry full-pixel output, complete ordinary/feature/J regressions and current exact EXE evidence are not yet accepted. Do not reuse this static pre-review to claim those passed. Physical display, human usability/accessibility, real cross-application clipboard, Win11/8GB and full-product completion remain unverified.

Closeout identity note: while writing this report, a final read observed `app.rs` had advanced to `23343000b827edef47cd96c9971abc7c8bfb8520eca14af5697f2d37e0e8c1a0`; `source_task.rs` remained as listed. That later App revision is outside this report's reviewed identity and is not accepted here. The finding above describes the earlier exact App hash in the identity table; repairs require their own recheck.
