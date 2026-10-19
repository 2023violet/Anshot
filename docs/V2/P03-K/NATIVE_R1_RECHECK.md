# P03-K Native R1 — independent static recheck

Verdict: R1_STATIC_FIXES_CONFIRMED_PENDING_INDEPENDENT_TESTS_AND_FULL_REGRESSION

This report supersedes the earlier cap finding only for the exact revisions below. It does not accept Task2, Task3, the complete K batch or the full product. The earlier `NATIVE_PRE_REVIEW.md` is retained. Independent reviewer performed source/JSON/log reads, a textual comparison with the preserved J source, and SHA256 checks only. No Cargo/test process, GUI, real clipboard/station API or benchmark was run; no product, lock, fixture or prior report was edited.

The modal-cap defect is repaired in the reviewed code: worker `PreparedSource` retains the same normalized cap used for CPU display preparation, passes it through its private install operation to the UI-level candidate, and `replace_prepared(ctx, source)` checks that cap against the current context **before** `Editor::replace_candidate`. Both direct and `PendingAction::Replace` acceptance use this method. A mismatch drops the new candidate and updates the error message without replacing Editor, source texture, history or output. The new test finishes a valid candidate into an existing-edits confirmation, changes the cap, confirms discard, then asserts unchanged stamp, masks and output, undo availability and removal of the pending modal. The implementation has no mutation of the old document on this branch. An explicit retry assertion would improve the test but is not a new static blocker: there is no remaining source slot or pending action and the normal start checks permit retry.

`poll_source` now clones the matching current key without taking it unconditionally, and clears `source_key` only when it matched the completed task. The deliberately different-current-key error test checks that its live key and status survive. `SourceTask` records the input's clipboard origin before moving the input to the worker; only a current matching task's failure uses that origin to preserve the paste-specific recovery message. This flag neither supplies source identity nor changes decoding limits. Stale errors still cannot update status.

The final-drain regression is now decisive for the intended race: a channel-gated worker cannot send until the test first observes `try_recv() == Empty`; the test then releases it, observes actual thread completion with no cached result, invokes the production `finish_poll`, and verifies the valid source bytes. Production receive/finished/final-drain ordering and capacity-one slot retention remain intact.

The adapted older tests now wait by driving actual `App.show` frames through `finish_source` until the source slot is empty, with a ten-second failure bound; they do not directly mark the candidate successful. I inspected the replacement/history, paste error/format/alpha, gesture, source-limit and callback assertions: synchronous return assumptions were replaced by completion waits, while the relevant terminal-state/pixel/format/ticket assertions remain. Oversized source is now expected to start asynchronously and then be rejected with the old document preserved, rather than incorrectly expecting synchronous `offer_source` failure. The J stale-review defense test explicitly injects an already Core-validated candidate to exercise rejection of a late review after replacement; it is correctly distinguished from the ordinary mutually exclusive source/review entry path. That injection is test-only and does not prove normal concurrent intake is allowed.

I read both actual three-case raw runs and independently matched stdout/stderr SHA256 to their JSON receipts:

| Receipt | Actual observed result | JSON SHA256 |
|---|---|---|
| `r1-accept-boundaries-red.json` | exit 101; 0 passed / 3 failed, including real document-id change on cap mismatch | `2aa46365158ff47e39d785eebea1a3c0f6b51aa8b7d4c8d09ba5884a696d3899` |
| `r1-accept-boundaries-green.json` | exit 0; same three named tests passed | `515a6650f2ff958a0fe7decb60e5c106b61b5fdb6a0b251eacc18a21c2172306` |

Both receipts record identical source-before/source-after maps within their respective runs. The three-case GREEN records App SHA `bf62db63c3941d88ce9398d1fee3ea8abd9a675e95118549e97ae3a45a63a215`, before the subsequent older-test waiting adaptations; its source-task SHA matches the reviewed source-task file. It is therefore evidence for that exact earlier run, not a test certificate for every byte of the later reviewed App below. These were implementer executions, not independent reviewer executions. The earlier whole-regression 65-pass/12-fail result remains an actual failure. At the point checked for this report, `native-full-r1.json` had not been generated; no whole-suite, feature or GPU success is inferred.

Exact reviewed identity, on main / HEAD `94f7f419aee91faf6f846349d40a897d0d72d7d0`:

| File | SHA256 |
|---|---|
| `poc/native-egui/src/app.rs` | `7c328862a221945c76892c68444f723c44a7feeca823ccd3724563344276b23e` |
| `poc/native-egui/src/source_task.rs` | `eb00f886615136e9b846de998b45a30ebd31097bb74679847d9a5ed9dccc94f2` |
| `poc/native-egui/src/display.rs` | `21338c4f16825855c8f723e4c04b0eea57b3bdc568456e4a77bec535bc7ea063` |
| `poc/native-egui/src/review_task.rs` | `b1bf49bac80dfe51ee1d2a40b6388d65e8490d22676e16f9d319a4e52f0b2f7c` |

No additional source-level blocker was found in these R1 changes. Independent focused execution after source freeze, passing normal/feature regressions, and exact-artifact diagnostic/UI evidence remain required. Headless egui drawing would still not be GPU or physical presentation evidence. Real clipboard/cross-app behavior, human interaction/accessibility, Win11/8GB and full-product acceptance remain unverified.
