# Background review Implementation Plan

> **For agentic workers:** Use superpowers:executing-plans to implement task by task. Root implements; an independent subagent reviews. Continuous local authorization replaces repeated human gate replies; no commit, branch, push or external action is authorized by this plan.

**Goal:** 复核编码期间主线程持续处理事件，忙碌/取消诚实可见，迟到结果不取得交付权。

**Architecture:** Editor发出不可伪造的复核工作和私有结果；单worker做Rust编码与CPU显示准备，主线程只try_recv、复核身份和上传纹理。同步Editor ABI复用相同领域逻辑。

**Tech Stack:** 既有Rust/egui/eframe/image，std thread＋容量1 channel，无新依赖或锁升级。

**Spec:** [DESIGN.md](DESIGN.md)

执行记录：Task1–3与独立代码/运行证据审查已完成，详见 [RESULTS.json](RESULTS.json)。下列清单是已执行计划的历史记录；流程验证、文档同步与不删除归档以 `target/workflows/p03-j` 的genuine状态为准，不能据计划勾选替代证据。

## Global Constraints

- main only；预期HEAD94f7f419aee91faf6f846349d40a897d0d72d7d0，保护I源码/锁/总方案/PDF/V1/金样基线，不清理其他脏工作。
- Rust处理与导出；源25MiB/24MP、生成物128MiB/完整PNG校验和当前凭证不放宽。
- 仅固定合成取证，不使用系统clipboard/私人图片或权限设置，不提交/推送/发布/删文件。
- genuine high Gate A/B、反例、精确产物、独立审查、有限结论和不删除归档。

## Review Focus

- 两个Editor数值ticket相同：不同source Arc成功/失败key均不能接受或清掉对方pending（Task1）。
- send已完成但旧worker尾部被barrier阻塞：不清slot/不加线程/队列，安全取消并最终可重试（Task2）。
- 等待期间编辑/换图/关闭：迟到成功和失败都不改新状态、不触发交付（Task1/2）。
- 解码/启动失败、panic/断连、cap变化：不留下永久busy/伪复核，当前可恢复（Task2）。
- 同帧已排Copy、入口finish后第二pass、discard后无图/隐藏canvas：入口捕获等待状态，按cumulative frame持锁跨全部pass拒绝交付；绘图标记绑定key/PNG/display/cap，私有envelope不重配同尺寸数据（Task2/3）。

## Task1：封闭领域复核工作

Files: `crates/shareready-editor/src/lib.rs`（必要时同crate私有review模块），其单测；不改Core编码。

Interfaces: 消费Editor Arc源/Region/Ticket；产出prepare_review、ReviewWork::key/run、ReviewKey::ticket、ReviewResult::bytes/dimensions、review_is_current/cancel_review、accept_review及StaleReview错误。key字段私有、可Clone；work/result不可Clone，同步begin复用且run失败key清理。

- [x] 写当前/跨Editor同数字的成功及失败key/改版/换图/取消后新pending/无交付、同步run失败可重试等分段测试；执行 `cargo test --locked -p shareready-editor` 固定RED。
- [x] 最小实现prepare/run/accept，保持已有begin/bytes_for/finish语义和全部原18项测试，运行同命令GREEN。
- [x] 检查新API文档不能把一般PNG当可信生成物；记录源码和实际命令，不提交。

## Task2：单worker与CPU显示准备

Files: `poc/native-egui/src/display.rs`、`app.rs`；仅为单任务边界需要时新增私有模块，`main.rs`声明相应模块。测试使用可控channel/barrier。

Interfaces: 消费Task1封闭key/result；产出PreparedDisplay/from_prepared、私有且不可重配的PreparedReview envelope、单slot/JoinHandle、start/poll/cancel及awaiting-draw。只在key当前、最后绘图pass的累计frame已推进且无modal时finish(true)。

- [x] 写send后barrier占槽、取消/编辑/换图/迟到错误/断连/失败/重试/cap变化、错误envelope以及接收同帧Copy/discard-final无图/隐藏canvas/modal测试，普通Native全workspace固定RED。
- [x] 提取Display纯CPU准备，保留同步入口；背景标准库线程生成PNG与PreparedDisplay，try_recv处理，容量1且取消后仍占worker槽直到结束。
- [x] 增加忙碌/取消、16ms后续重绘、按key失败清理和完成绘图帧守卫；不join运行线程，is_finished收尾计时；运行普通及feature Native测试GREEN。

## Task3：实际异步窗口与回归

Files: 新显式合成diagnostic example/runner、必要的既有visual/performance driver等待调整；不允许普通模式添加图片/按键遥测。

- [x] 固定实际普通路径、6000×4000及9MP样本，记录worker/触发返回/接收/上传/不同帧；busy与最终PNG GPU图带当前任务身份。
- [x] 至少30不同帧只计真实worker开始至CPU准备完成间；记录完整key公开编号/trigger/worker编码及准备/send-receive/线程收尾/绘图晋权/UI-upload/取消尾部内存与截图请求身份；不足保留实数重规划，不用完成后等待或延时凑数。安全取消/编辑迟到丢弃后新复核全图比较。
- [x] 原生8图/19PNG、主workspace、feature/bridge、五冻结goldens、大成品/受控Copy与原像素路径回归，所有产物绑定最终完整source_map。只在真实需要时重建共享WASM。

## Task4：独立审查与阶段归档

- [x] 独立审查代码、反例、全部真实产物/图/任务身份及计量限制；阻断先修复并复验。
- [x] 写当前RESULTS/README/清单、逐项接受与NOT_VERIFIED；同步root/V2入口，保持I为历史基线。
- 流程收口记录：genuine coverage/verify/review及closing-out现场审计、不删除归档的完成状态，以保留的 `target/workflows/p03-j` 与 `target/workflow-archives` 记录为准。失败与工作目录保留，完整产品保持进行。


## R1 independent-review correction

- A2 remains unproven by unit green. Independent review found an Empty-receive -> worker send/exit -> finished-poll race, and confirmed Close did not invalidate pending/awaiting keys. Reopen Task2; preserve previous evidence and two-failure escalation. Add deterministic boundary counterexamples before fixes. Drain once more only after confirmed thread exit/join; confirmed Close cancels the owned key and clears awaiting view while retaining a live worker slot. No live join, extra worker or output permission.
- The first real grid pilot was stopped by the existing owned-process 120s no-progress guard. Its diagnostic center_review nested Context data_mut and pixels_per_point locks. Read ppp before the data closure. This does not establish a deadlock in ordinary UI.
- Bind the busy GPU request to the same task public key and actual worker_start..cpu_prepared timestamps. Keep 30 distinct UI frames strictly inside that interval; awaiting-result frames cannot count.
- Pin the V1 build to --target-dir target, record CLI/preview hashes before golden execution and recheck afterward. No inherited CARGO_TARGET_DIR may redirect the producer away from its consumers.
- Keep independent review open until corrected source and real pilot/full evidence are reviewed. No success threshold, budget, dependency or authorization change.


## R2 terminal Close correction

Independent R1 recheck passed 15 focused tests but found the old handle_close_request reset close_confirmed on a subsequent actual ViewportEvent::Close. Treat explicit confirmed Close as a terminal latch until destruction, not a one-shot bypass. First add a real RawInput viewport Close event counterexample followed by late Copy/Save/start attempts, then remove the reset and strengthen the legacy explicit-close assertion to require the latch remain set. Preserve cancellation/history assertions and no extra Close command/CancelClose. Retain live worker slot and poll cleanup. This closes the Native callback authority; it does not claim OS destruction timing or human close UX. No dependency, output privacy, deadline, concurrency or acceptance threshold change.


## R3 diagnostic artifact namespace

Second real grid pilot completed the first worker and delivered the current controlled output, but the requested GPU image attempted first.png, already created by the full PNG output consumer. The create-new boundary correctly refused OS80; the runner reported no completion. Separate GPU filenames/states as gpu-busy/gpu-first/gpu-retry while keeping first/retry PNG+RGBA output artifacts. Verify each GPU request name/ticket against the corresponding image record and retain every raw failed pilot. A3 is not established by the partial first-task trace. No overwrite, deletion, fake delay, relaxed sample count or budget. This is diagnostic artifact routing, not a new product export behavior.
