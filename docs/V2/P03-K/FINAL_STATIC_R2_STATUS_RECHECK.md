# P03-K R2 — independent diagnostic-status static recheck

Verdict: R2_DIAGNOSTIC_STATUS_FIX_STATICALLY_CONFIRMED_RUNTIME_PENDING

This is a new addendum; previous reviews and old run records are unchanged. The misleading controlled-Copy status was found by root's actual visual inspection, not by this reviewer. My earlier static candidate review did not catch it. A green machine command did not establish that screenshot wording was truthful.

I compared all 84 current inputs with both `FINAL_STATIC_SOURCE_MAP.json` and the preserved pre-R2 source archive. **Only `poc/native-egui/examples/support/source_driver.rs` changes**; no input was removed. Its exact diff is:

```diff
-        if !called{return Err(self.app.status.clone());}Ok(())
+        if !called{return Err(self.app.status.clone());}
+        self.app.status="合成取证：受控输出核验完成，未使用系统剪贴板。".into();Ok(())
```

This sets honest synthetic-evidence status only after the controlled Copy closure has successfully written the RGBA output. Failed/uninvoked consumers still return the prior error. The ordinary `copy_choice`, OS clipboard adapter, domain, worker/poll/key/Close/cap guards, image/oracle/method rules and all other mapped code remain byte-identical. The driver still calls the ordinary Copy boundary with a local file-writing consumer and does not invoke OS clipboard operations. First/retry screenshots should now display the controlled-output wording rather than the ordinary system-clipboard success claim, but I have not inspected a new R2 screenshot or executed the new EXE.

Current source-driver SHA256: `aa6b3bfb5fbc912ec57dfc73e16e1be12b0f8776c275878d64a5bb88ab78b704` (previous `335af4c7b806045818de793642f7c3d73669a58db234c774ed66873ff5e530a2`). The current 84-input map gives formula identity `114491f9a9a202b39f5e0a1c38db3e21fde738cb138bd0015c095a135c7be014` using the runner's sorted-JSON hashing; this calculation alone is not proof of a current build/run.

For historical separation, I read the completed pre-R2 report `window-20261019T004720615835Z/report.json`: it records `ok=true, pilot=false`, source count 84, build `aa277ed50e862194273a8e796e56c19a5c09f5941df8d9a6daa0018600fcff0e`, EXE SHA `ff56ab7ede3f5ae93e930d7786669e512b3303be30352d159847751b65932f63`; report SHA256 `528228a82c3b9d54bf6aa7985e4045861a704e8928ddf63b91126d71b7a94f44`. Its source map equals the previous static candidate. This is a read of historical report identity/status, not an independent verification of all of its runtime artifacts or an acceptance of its inaccurate screenshot wording.

I independently hashed all 84 copied files in `source-full-r1-20261019T005818941823Z/files`; every file matches that previous exact source map. The archive's `report.json` SHA256 is `227630b0d997c9ec8ceebd2bde62771ec5f748e59b82b8f33045a339e6f06155`. It preserves the pre-R2 version for later reproduction rather than silently replacing it with current code. I read its V1 producer metadata but did not rebuild or execute those saved producers in this recheck.

The genuine event log records Step3 reopen, the visual failure bringing its failure count to two, replan without waiver, renewed A/B and return to in-progress. That is consistent with preserving machine GREEN separately from failed visual acceptance and rerunning the changed exact artifact. The new R2 complete command is still running; I do not infer its result from the old run or the small source diff.

No additional blocker was found in this bounded diagnostic correction. No source, lock, fixture, workflow or old report was edited; no Cargo, GUI, clipboard/station API, benchmark, image decode or heavy oracle was run. New R2 exact-EXE/streams/images/full outputs/full regressions and independent current-code execution remain pending. Physical/human/clipboard/Win11/8GB/G04/full-product limits remain unchanged.
