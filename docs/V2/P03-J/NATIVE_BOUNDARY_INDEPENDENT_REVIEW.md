# P03-J Native 边界独立初审

结论：**BLOCKED_PENDING_FIX_AND_RECHECK**。本次冻结候选的既有 focused 单元测试全部通过，但发现两项 Native 状态边界阻断，以及新增诊断的嵌套 Context 取锁问题。实现者已接收并准备决定性 RED、修复和 genuine 重规划；尚未在本报告中认定修复完成。此文件保留本轮原版本审查事实，后续版本需另做精确身份复审。

本次读取 DESIGN、IMPLEMENTATION_PLAN、A1 独立报告、Editor、app/review_task/display，以及 async_window、async_driver、run_async_checks 和 smoke/visual/performance 等待适配。仅执行获准的 headless focused 单测并写新审查证据，没有修改产品、锁、fixture，没有启动 UI、性能子进程、真实剪贴板/station 或对外操作。main / HEAD 为 `94f7f419aee91faf6f846349d40a897d0d72d7d0`。

## 阻断

1. **poll 的 Empty→send/exit 竞争可能丢失真实成功结果。** 审阅版本 `review_task.rs:128–139` 先 try_recv，随后检查 is_finished、join，最后仅从 received 返回。如果第一次读取 Empty，而 worker 在它之后 send 并退出，is_finished 会变 true，队列里虽有成功结果，received 仍为空，当前代码误报“未收到复核结果”。这是源码中可构造的明确调度时序，本次未运行强制该窗口的测试；现有 barrier 测试只覆盖 send 之后线程仍活着，未覆盖此间隙。最小修复是在确认线程结束并 join 后，received 仍空时再非阻塞读取已结束 sender 的队列；增加 Empty 之后才完成 send/exit 的决定性反例。该问题不授予错误产物交付权，但会偶发丢结果并要求用户重复复核。

2. **确认 Close 丢弃没有使当前 review key 失效。** 审阅版本 `app.rs` 的 resolve_pending(true)/Close 分支只设 close_confirmed 并发送 Close，不取消当前 key 或清 AwaitingDraw。若关闭命令后还有一次 App 回调，pending 已为空，旧工作仍可接收甚至晋权，违反“确认丢弃使 key 失效”。不能依赖 OS 总会立即销毁窗口。应以相同 sealed key 取消接收并清显示守卫，保留尚在运行的 worker 槽而不 join；测试确认 Close 后继续 headless 回调，分别覆盖 running slot 与 AwaitingDraw。现有 modal 测试覆盖 Replace 的确认丢弃和 Close 的取消确认，缺这个分支。

3. **新增 async diagnostic 的 center_review 嵌套取 Context 锁。** `async_driver.rs` 在 ctx.data_mut closure 内调用 ctx.pixels_per_point。锁定 egui 0.36.2 的 context.rs:1033 经 write 持有独占 Context 锁，pixels_per_point:2288 经 input 再读取同一 Context；Context read/write 实现在 759/764。应在 closure 外先读取 ppp，再只插入计算好的值。根节点先定位该问题，我独立检查了调用点和锁定依赖路径；未亲自运行 UI 复现。它属于测试专用 driver，不能据此断言普通 UI 相同路径死锁。

根节点执行的旧版 pilot 已诚实失败：`target/p03-j-evidence/window-20261018T130611960381Z/report.json`，SHA256 `84c8bc42a057153588dbf5613eebe6d54aa5e1409a35a800ee8762c789b99e40`；build `61c84f802afffbded8d8f6eaf014edf52c4086e08ba06056f4875b424989452b`，EXE `8bc8b49a089692b57a40f57287c47169168d8eb1b39754dccd453150b72f462e`。我读取该报告：ok=false、grid process/monitor failed、125.217 秒终态、no_stdout_progress_guard 约120秒、query_errors 空。该报告证明实验未完成和保护回收，不单独证明死锁调用栈；根因判断来自代码取锁关系。未把首轮已有输出当整批通过。

## 独立实际单测

在 pilot 终态后，使用固定 stable MSVC cargo，显式设置同目录 RUSTC/RUSTDOC，逐条实际执行：

`cargo test --locked --manifest-path poc/native-egui/Cargo.toml --bin shareready-native-egui-poc FILTER -- --test-threads=1`

| FILTER | 实际结果 | 命令墙钟 |
| --- | --- | --- |
| async_ | 8 passed，0 failed/ignored，53 filtered | 26.721秒 |
| review_task::tests | 3 passed，0 failed/ignored，58 filtered | 2.987秒 |
| display::tests::prepared | 2 passed，0 failed/ignored，59 filtered | 1.285秒 |

具名记录验证了取消后 send barrier 持槽、当前失败/panic 恢复与旧失败忽略、启动失败清理 helper、cap 改变拒绝和重试、隐藏 canvas、discard 后最终 pass 无图、等待 frame 跨 pass 拒绝 Copy/Save、modal 延后/Replace 丢弃，以及跨 Editor 同数字 envelope 拒绝、原像素和缩略图准备。启动失败测试调用受控清理 helper，没有实际耗尽 OS 线程资源；Copy/Save 帧测试调用 cfg(test) consumer，没有系统交付。headless Context/shape/texture delta 不等于 GPU 或人类验收。

完整命令、环境、具名测试、原始 stdout/stderr SHA 与运行前后10项输入身份在 `target/p03-j-evidence/reviewer-native-focused-20261018T131036457263Z.json`，SHA256 `718b3db10de7ae9fbcd3d4cc22e43c04467e811b3f54b933bd01bfc55024759f`；同名前缀 `-1/-2/-3.stdout.txt` 与 `.stderr.txt` 为完整原始输出。测试期间选定源码和两锁逐字节不变，三个命令均退出0。**这13项绿色未覆盖前述两个新发现，不构成审查通过。** 未独立重跑全 Native/feature/bridge；实现者 feature 原日志的 66+4 汇总已读取，不能冒称我的执行。早前 A1 的26+2独立执行保留原报告，不在此重复计算。

## 已检查的结构与后续取证约束

- 私有 PreparedReview 只能由同一次 ReviewResult/bytes 构造显示准备，安装前检查当前 key 与有效图片 cap；失败按 sealed key 清理，旧消息不更新新状态。CPU 生成 PNG、严格生成物解码、ColorImage 和缩略图在 worker；UI 接收上传并 accept。非 Fit 可见裁片仍在 UI 复制/上传，不能描述成所有 UI 无重活。
- 单 slot 保留 received 与 JoinHandle，is_finished 后才 join；取消不提前释放槽，无排队或运行中 join。ReadyReview 按 key/纹理实例/cap 与正面积 clip 记录每 pass 标记，在累计 frame 推进后晋权；每 pass 清除旧标记，入口等待帧拒绝锁跨同 frame 的后续 pass 保持。该协议证明 egui 图元排入完整帧，不证明 GPU/OS 呈现。
- 新 driver 只使用嵌入 grid24MP/noise9MP，Trace::create_async 按完整固定字节允许，普通导入/clipboard 在合成 trace 下拒绝；时序日志为 feature-only、缓冲后落盘。source_map 模式覆盖新 review_task、async example/support 与 runner。未见人为 sleep 或跳过真实 PNG/凭证的通过路径。
- smoke/visual/performance 已改为等待真实复核、保持 PNG 状态和墙钟保护；performance 的 trigger timing 已标 `ASYNC_TRIGGER_ONLY_NOT_COMPLETION`。本次只静态检查，未执行这些 GPU/窗口回归；Task3 全部产物仍待。

runner 还有两项应在最终取证前收紧：①v1-build 未显式 target-dir，后续 verifier 却固定读取根 target/release；继承 CARGO_TARGET_DIR 时可能验证旧 CLI，应让构建与消费路径一致并绑定两个真实 EXE。当前没有证明该环境变量实际存在，不声称本次 pilot 已假绿；pilot 本就不跑 V1。②busy 截图除了 reviewed=false，还应核请求 ms 落在同一 task 的 worker_start..cpu_prepared，避免把完成后等待 poll 的图叫真实 worker 忙碌图；30 帧验证目前已严格使用真实工作时间区间。identity 的 origin_unix_ns 实际采样于 identity 写入时，应改名或在 start 处捕获真正 wall origin，避免和 RSS 的绝对时间错误对齐。

## 精确审阅身份与限制

完整17项只读快照为 `target/p03-j-evidence/reviewer-native-boundary-source-20261018T130934182378Z.json`，SHA256 `b2b095547b080c4c3a67f603cd20d11d9eebff99a758f16b0e600ea83d201606`。主要产品身份：

| 文件 | SHA256 |
| --- | --- |
| crates/shareready-editor/src/lib.rs | 3e79cce40918bbdd1cc45b82fca83f2cd03cd3fb16d085ec5895c4772bfb49ac |
| poc/native-egui/src/app.rs | b9f2e7ffc8e36e79d8616b67773680139f88a670a6bbe810c7e36aea54efa70c |
| poc/native-egui/src/review_task.rs | 6a4243c457386b61cb329441c7d099506aed58d93d62934a6e3a66b0733c28d0 |
| poc/native-egui/src/display.rs | d3dd3b5760c7c50cbdf64ecb078e54ad37461680c86d8a6bd363cead1a541ef2 |
| poc/native-egui/examples/support/async_driver.rs | 7150eb798e849c44a76dac4c295b59a3ec06c361ad3bb2b34d7967c4a3ff8e5e |
| tests/native_oracle/run_async_checks.py | 9474b447c8dd1b7793e43df860d99885835bf3678d660ad4b8800ea0d1cecbe1 |

此后实现者增加决定性反例/修复时，这些旧身份及日志只作失败版本历史证据；不能套用于后续版本。未运行新 release UI、未查看本批 GPU 截图或独立 PNG 全图 oracle，没有物理输入、DPI、Narrator、真实跨应用剪贴板、Win11/8GB、完整安全或产品验收。当前预算与 G04/完整产品门槛保持开放。
