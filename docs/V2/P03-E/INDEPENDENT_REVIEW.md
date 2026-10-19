> 独立子代理审查记录的本地路径脱敏副本；原始记录哈希见 RESULTS.json。

# P03-E M06-A independent local review

Date: 2026-10-17. Reviewer: separate `save_review` agent, not the implementer. Authority: `docs/governance/AUTONOMOUS_LOCAL_DEVELOPMENT.md`. Product source, manifests, locks and frozen fixtures were read-only during this review; reviewer scripts, logs and synthetic outputs were written under ignored `target/` only. This is a scoped engineering review, not an external security audit or release approval.

Verdict: **NO BLOCKING FINDINGS IN THE SCOPED LOCAL M06-A CHANGE**. The bounded new-file delivery behavior is supported by independent command runs and artifact checks on this machine. Full M06, G06/G07, human usability and wider Windows/filesystem compatibility remain unverified. No product fix is requested by this review.

## Exact reviewed identity

- Branch/HEAD: `main`, `94f7f419aee91faf6f846349d40a897d0d72d7d0`, with preserved uncommitted work. HEAD alone does not identify this implementation.
- Reviewed build identity: `1b7f89be3d72f95b598c0d737ec39eb28d4bf365bb6470f7f1d1535e244dd645`.
- Exact copied EXE SHA256: `10ba7a7de24fa714b3355be8206c17c62dc186f6d67c7e5c814366c5d7fca3d5`.
- Implementer harness report: `target/p03-c-evidence/native-20261017T141409357447Z/report.json`, SHA256 `38b45d761fb50edbe65b3a105468e0e9702aff524658c72254c1300a1672c65f`.
- Independent verification JSON including the CI addendum below: `target/p03-e-evidence/reviewer-verified.json`, SHA256 `3a9cbbf43584671891829e8c3e1675cd9cd1aa8759bdbd4964b804faf355fd23`.
- Its `source_files` map records all 22 individually rehashed source, manifest, lock and T04 fixture inputs, checked against the harness and checked again after the independent EXE run.

Key current source SHA256 values:

| File | SHA256 |
| --- | --- |
| `crates/shareready-platform-windows/src/windows.rs` | `6dc37cc3ba7474bc73cecd7639867403fa7654f369d5fd9f57ef815fc3e85230` |
| `crates/shareready-platform-windows/src/lib.rs` | `57a6901c743f16c97f1f72d4a111f25c31cd45260415b1cc6c249a982b14bce4` |
| `poc/native-egui/src/app.rs` | `3553f33824f738bfeb6b7323c651367ecd63e539f66c2c37d54cbec19e2073ad` |
| `poc/native-egui/src/main.rs` | `e15e020eb62a51b62d92568c9d709b1bce519aeff48c2f1a8792f5a3cea75c5a` |
| `tests/native_oracle/run_native_checks.py` | `99513051727bbbec01d7910a2a4ba72f4fa92a53f659c3804587328b7544bfaf` |
| `Cargo.lock` | `574836ca69c075b9844928fcc320d89ce2742001ae4f0596b67558b6ff1f5266` |
| `poc/native-egui/Cargo.lock` | `7daa5b2c8d4f7d18d9c42931d224d171fd396824ff2a8d6c880f12f3be9bd0a1` |

## Independent executions

Rust commands used `$LOCAL_USER/.rustup/toolchains/stable-x86_64-pc-windows-msvc/bin/cargo.exe`, with explicit sibling `RUSTC=rustc.exe` and `RUSTDOC=rustdoc.exe`. Python used `$LOCAL_USER/.cache/codex-runtimes/codex-primary-runtime/dependencies/python/python.exe`.

| Actual reviewer command | Result | Evidence |
| --- | --- | --- |
| `cargo test --workspace --locked` | exit 0, 43 tests passed across workspace; doc tests exit 0 | `reviewer-workspace.log` |
| `cargo test --manifest-path poc/native-egui/Cargo.toml --locked` | exit 0, 31 tests passed | `reviewer-native.log` |
| `cargo test -p shareready-platform-windows --locked` | exit 0, 9 tests passed | `reviewer-platform.log` |
| `python -m unittest discover -s tests/native_oracle -v` | exit 0, 19 tests passed | `reviewer-oracle.log` |
| `python target/p03-e-evidence/reviewer_verify.py` | exit 0; identities, all five harness log pairs, two artifact oracles, trace and locks verified; exact copied EXE executed independently | `reviewer-verified.json`, `reviewer-selftest.stdout.txt` |
| `git diff --check` | exit 0 | inspected command output; only existing Windows LF/CRLF notices |

Reviewer log SHA256:

- workspace: `2f465b7e658cba161e9f3a9576107fa883121f2563d76f1e91108d4342ed62aa`
- native: `f862dc740351cd6e3119403a2c2bc98e6df35a38009600b1a09290dee7472a2f`
- platform: `1e128c3c6e163c9c9d69be056f97dce62a069a965e5cfeac27582a84fc565d9c`
- oracle: `ce688fcca357b6575e79c5411342862a4ec073da97f3b91301f97c70b2b5bc01`

The workspace emits existing e1-spike output-filename collision warnings, with no command failures. This review does not reinterpret those warnings as new save-adapter failures.

## Acceptance comparison and implementation review

1. **Create, exclusive ownership, complete commit:** `OpenOptions::create_new(true)` with explicit `GENERIC_WRITE|DELETE`, `share_mode(0)` and `FILE_FLAG_DELETE_ON_CLOSE` precedes writing. The public function runs `write_all`, then `sync_all`, then clears on-close using the same live borrowed File handle; it closes that handle before success. Actual tests confirmed committed complete bytes/readability, no second read/write or rename during ownership, and one complete winner among concurrent creators.
2. **Rollback without path cleanup:** both FFI calls borrow a valid File handle; no raw ownership transfer occurs. The generated C-layout structure, exact length, stack lifetime and synchronous call establish the small unsafe boundary. All error returns retain OS on-close cleanup. Partial-write, sync-error and commit-error injection tests removed only the new output; same-path retry succeeded. A child process calls `process::exit(23)` with a live uncommitted handle: actual exit 23 and absence afterwards were independently rerun. There is no adapter `remove_file`/DeleteFile-by-path cleanup.
3. **Existing/source target and dangerous namespaces:** existing-file and source-file tests retained original bytes, while target collision and GUI location failure allowed retry. ADS colon components are rejected before open; device/generic verbatim namespaces and reserved DOS names are rejected by validation. Device-name tests call only the validation function. The actual ADS attempt uses an owned synthetic base file. Existing symlink/hardlink targets remain subject to `create_new` refusal; full reparse-point/mount attacks were not experimentally tested.
4. **GUI ticket/state discipline:** `save_choice` rejects wrong-kind and stale tickets before filesystem work; a pending discard modal blocks delivery and completes a valid pending Save as failed. Cancel/error preserves encoded PNG, revision and history; status indicates failure/cancel, retries succeed. Correct-kind success is recorded only after the adapter returns and Editor accepts the same ticket. These are machine/headless tests of controller paths, not a human/system-dialog observation.
5. **Rust boundary/dependencies/artifact:** GUI `main.rs` and Editor retain their existing `#![forbid(unsafe_code)]`. Core source and manifest hashes match P03-D, with no newly introduced unsafe code. **Core did not already contain `forbid(unsafe_code)`; do not claim that it does.** All new Win32 unsafe code is in the platform crate; pixel transformation/PNG encoding remain in existing Rust Core/Editor. An independent structural lock comparison preserved every old package record/checksum/dependency list except the explicit new native adapter edge. Root adds only platform 0.1.0, windows-sys 0.61.2 and windows-link 0.2.1; native adds only the platform package and edge. The windows-sys version already present in the native lock is reused.

The reviewer reran the copied exact EXE with `--self-test --self-test-output target/p03-e-evidence/reviewer-selftest-1792246603827461800`. It emitted `SR_V2_03_SAVE_NEW_HANDLE_BOUND_PASS`, produced output via the real new adapter, and returned exit 0. Independent full-image RGBA/PNG-structure verification matched the harness output. The two verified PNGs contain only IHDR, IDAT, IEND:

- self-test output PNG SHA256 `d19b1540029ff9578db3a89d11316eae39f6bbed02cb32da9cc5ce5ba69cd7a2`, RGBA SHA256 `b983aa4b8039d92d57d76370af66579b3538f9ce3348ab635545b44e6db96865`.
- actual-window trace review PNG SHA256 `dd420118b7784e0a41ae11739a07089f6da640a51d88435de91d5badf889ce31`, RGBA SHA256 `6be3f5e6fcdd58affaf8ad199ecef518aa35fd18f0c5fd3133cfaa860c77fed4`.

## Limits that remain open

- Measured local behavior is Windows 10 build 19045 on D: NTFS, consistent with the supplied read-only system-query JSON. Other Windows builds, FAT/exFAT/ReFS/SMB/cloud folders, reparse points, removable media and unsupported on-close commit behavior were not run. The code returns errors rather than falling back to overwrite/direct writes when the commit API fails.
- The sync/commit fault tests inject Rust errors at defined stages; they do not cause physical disk-full, hardware I/O failure or a real `SetFileInformationByHandle` failure. This supports control-flow cleanup on an error, not proof of all filesystem implementations. Process exit is a real OS test, but forced termination, power failure and durability under power loss were not tested.
- While writing, a directory entry can be visible even though its file cannot be read/written/replaced by ordinary conflicting opens. No directory-entry atomic visibility claim is made.
- The DX12 lifecycle run and its trace/log/PNG identities were independently revalidated from disk. The reviewer did not independently operate a physical window, take screenshots, operate the Windows Save dialog, test Explorer dragging, mixed DPI, Narrator or ordinary human tasks. Machine evidence cannot close those gates.
- This review covers new-file delivery only. Existing-file overwrite, clipboard/cross-application alpha, M04, remaining M06 work, M07/M08/M09, performance and full product gates remain open.
- Gate A/B confirmation is by the agent under the recorded user-approved policy, not a human review of the tokens. Workflow closure and documentation archives remain the parent agent's responsibility; this report does not self-confirm any gate or publish anything.

## Reference cross-checks

Read upstream deciding-and-executing SKILL.md (metadata 4.1.0) and stage-4/verification references to compare evidence scope. Win32 semantics were cross-checked against Microsoft's primary documentation: [CreateFileW](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-createfilew) for create/share/delete-on-close and [FILE_DISPOSITION_INFORMATION_EX](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/ntddk/ns-ntddk-_file_disposition_information_ex) for setting/clearing on-close. These references support source review; the observed local tests, not documentation alone, support the measured compatibility claim.

## CI integration addendum

The subsequent `.github/workflows/v2-03-native-poc.yml` change was independently inspected. Its exact SHA256 is `20a58267c745a018481fbcc9b3c455a06229a1c8ece22e097a6c196e2be71139`. Verdict remains **no blocking finding in this bounded change**.

The diff contains exactly five added lines: three `push.paths` entries (`crates/shareready-platform-windows/**`, root `Cargo.toml`, root `Cargo.lock`), and the explicit `cargo test -p shareready-platform-windows --locked` command plus its `$LASTEXITCODE` failure check before the native tests. The path entries ensure adapter/root workspace or lock changes trigger the existing Windows workflow; the explicit root-package command is necessary because the separate native manifest test does not itself execute the platform crate's unit tests. Failure propagates by throwing in the existing PowerShell step. YAML list indentation/scalar structure is consistent with the existing entries. No permissions, upload, renderer, action-pin or other workflow changes were present.

Actual addendum command: `python target/p03-e-evidence/reviewer_ci_verify.py`, exit 0. The script independently constructs the expected five-line addition against `git show HEAD:.github/workflows/v2-03-native-poc.yml` and requires exact normalized-text equality, rechecks all 22 previously reviewed source input hashes, and invokes PowerShell's AST parser on the full modified build/test run block. Parser exit 0, zero parse errors; output is `reviewer-ci-parse.stdout.txt` / `.stderr.txt`. All 22 source inputs, original build identity and EXE hash remain unchanged. The previously independently executed platform command passed 9 tests; the full suites were not needlessly repeated for this workflow-only addition.

Remote CI is **NOT_RUN**: no push, workflow dispatch, publish or external operation was performed. A complete YAML-parser run is **NOT_RUN** because the provided Python runtime has no PyYAML (`ModuleNotFoundError`); no package was installed. The exact additive YAML structure and PowerShell syntax were checked as described, without claiming a complete workflow runner/YAML validation. The CI file is recorded separately under `ci_addendum` in the updated independent verification JSON and does not alter the original 22-input EXE build identity.
