# P03-J Native 边界 R1 独立复审

结论：**BLOCKED_ON_CLOSE_LATCH_RESET**。R1 的最终队列读取修复与原 Close key 清理已通过定点代码检查和实际独立 focused 测试；新增诊断取锁、截图请求身份/实际工作时段、V1 构建路径及跨时钟标签修正也已静态核对。但发现确认 Close 后的 terminal latch 会被后续真实 Close 事件重置，重新开放交付守卫，须 R2 反例与修复后再审。原 `NATIVE_BOUNDARY_INDEPENDENT_REVIEW.md` 保留不变，不能将 R1 测试绿色记成整个边界通过。

本轮只读产品/诊断源码，执行允许的 headless focused 测试并创建新报告与原始日志；未改源码/锁/fixture、未运行 UI、GPU 试验、真实 clipboard/station、性能负载或对外操作。测试前后选定10项输入逐字节一致。父代理等待这些测试结束后才开始 R2；本报告身份仅绑定 R1 冻结版本。

## R1 已解决的部分与新阻断

- `ReviewTask::poll` 分出同一生产路径私有 `finish_poll`，确认 is_finished 后 join；若还没有 received，再 try_recv 一次最终队列。此时 sender 已退出，不会等待。新增决定性测试先证实队列 Empty，再放行 worker 生成/发送并等待退出，直接继续生产 finish_poll；实际得到 success 且只消费一次。已覆盖原审查指定的 Empty→send/exit 间隙，没有靠 sleep 猜调度，也没有提前释放活动 worker 槽。
- resolve_pending/Close 现在按 sealed key 取消，清 ReadyReview/视图，运行中 slot 仍保留；start、Copy/Save 及迟到 consumer 在 close_confirmed 为真时拒绝。新增测试实际覆盖 running slot、AwaitingDraw 和已完成成品上的 late Copy。
- **剩余阻断**位于 R1 `app.rs:112–114`：handle_close_request 收到 close_requested 后，若 close_confirmed 已真便执行 `self.close_confirmed=false; return`。这是旧的一次性放行语义，和 R1 新增 terminal 守卫冲突。已完成 PNG 仍保留于 Editor，确认 Close 后若又收到带 ViewportEvent::Close 的回调，该入口会解除 start/Copy/Save/consumer 的新守卫；同帧或之后回调可再次交付。R1 新测试的 late frame 没携带此 Close 事件，未覆盖该路径。父代理确认需要 R2：保持 closing terminal latch，不因重复关闭事件重置；新增真实 RawInput viewport Close 事件加 late Copy/Save 的 headless 反例，保留取消确认/历史语义。这里是确定的源码状态路径，本次未运行 OS UI 来声称平台销毁行为。

## 独立实际执行与 TDD 日志核查

固定 stable MSVC cargo，显式同目录 RUSTC/RUSTDOC，逐条运行：

`cargo test --locked --manifest-path poc/native-egui/Cargo.toml --bin shareready-native-egui-poc FILTER -- --test-threads=1`

| FILTER | 实际执行 | 命令墙钟 |
| --- | --- | --- |
| async_ | 9 passed / 0 failed / 0 ignored，54 filtered | 22.679秒 |
| review_task::tests | 4 passed / 0 failed / 0 ignored，59 filtered | 3.949秒 |
| display::tests::prepared | 2 passed / 0 failed / 0 ignored，61 filtered | 1.162秒 |

共15项实际执行、均退出0，包含两个 R1 新具名测试：`final_result_sent_after_empty_receive_is_not_lost_when_worker_exits`、`async_confirmed_close_revokes_running_and_awaiting_results_before_late_frames`。完整命令/环境/测试名/前后源码与 stdout/stderr 的 SHA 在 `target/p03-j-evidence/reviewer-native-r1-focused-20261018T131701906948Z.json`，SHA256 `bb185857ca2bef9ed428f6df250beb75a0596f3a06a5c8b5a9a7d4d90904c414`；同前缀 `-1/-2/-3.stdout.txt` 和 `.stderr.txt` 为原始输出。

我另外读取实现者的完整 R1 RED 日志：61 passed / 2 failed，失败分别是 queued success 被 missing-result error 替代、Close 后 key 仍 current，符合两项原发现；不是缺 API 的编译 RED。stdout SHA `4d0360b3d1b29e21092c1d7cafd29fa98dbd253a5e7650916f877bfb85d8665e`，stderr SHA `0a1dda5a3ef066cac6ea737e3a5bdd9253bec6d17e47183f832fc2b9867728b9`。RED 由实现者运行，未冒称独立执行。genuine `r1-native-frozen-green.json` 记录 step2 done，SHA `34d89f0e440b7b8844d425e38ad01bfc28e7c22209fc4f1553aad5978e652bf0`；另有源码变动时被 genuine 拒收的旧 green/failure 记录仍保留，没有用它替代冻结证据。

## 诊断与时序复审

center_review 现在先在 data_mut closure 外取 ppp，closure 内只插入值，已移除已定位的嵌套 Context 读取。截图请求记录完整 document/revision/operation，runner 验证该身份来自实际任务；busy 请求 ms 必须在该任务 worker_start..cpu_prepared，完成图须在 promoted 之后。snapshot 回调仍拒绝状态变化，未用等待已完成结果、人为 sleep 或降低像素标准凑30帧。

R1 最终 drain 的 received_at **可能晚于** finished_observed_at：例如首读 Empty、随后观察退出/join、最终读取。当前 runner 正确要求 trigger≤worker_start≤encoded≤cpu_prepared≤send_invoked≤received，并单独要求 finished_observed≥send_invoked；不强加 received≤finished_observed。日志缓冲顺序亦可能先打印 received 再打印较早的 finished_observed，应按时间字段与事件名理解，不能把数组顺序当真实总序。join_ms 为实际 join 调用 CPU 墙钟，不是 worker 总尾部时长，也不证明绝对零等待。

V1 build 现在显式 `--target-dir target`，构建后、五金样执行前记录两个固定 EXE SHA，执行后核相同；不再受继承 CARGO_TARGET_DIR 导致消费旧默认目录的原缺口影响。identity 改为 identity_unix_ns 与 origin_unix_ns_estimate，注明相邻 wall/monotonic 样本的近似关联，不声称精确跨时钟同步。这些仅静态复审，尚未由我运行本轮 release runner、goldens 或 GPU 回读。

## R1 精确输入

| 文件 | SHA256 |
| --- | --- |
| crates/shareready-editor/src/lib.rs | 3e79cce40918bbdd1cc45b82fca83f2cd03cd3fb16d085ec5895c4772bfb49ac |
| poc/native-egui/src/app.rs | c3dc126b9c895cf04ec5b29dfe82368a46409108e93a1359b3f4e07a27d5032c |
| poc/native-egui/src/review_task.rs | b1bf49bac80dfe51ee1d2a40b6388d65e8490d22676e16f9d319a4e52f0b2f7c |
| poc/native-egui/src/display.rs | d3dd3b5760c7c50cbdf64ecb078e54ad37461680c86d8a6bd363cead1a541ef2 |
| poc/native-egui/examples/support/async_driver.rs | 69b939e64fdc64290a7b5bd58d785d065cd91775a01aa2da9330680f602f2d96 |
| tests/native_oracle/run_async_checks.py | 3995e2c7a20e256c0139f9423b52830956e81d734e5b457d5ef8a9c2aafa9971 |

另外两锁及 trace/visual 前后 SHA 在 reviewer JSON 中，均未变化；原阻断报告 SHA 仍为 `05ce9d16a6f12bec67ee19c000beae8c5e54651adb47f81f1b2c3b74aaf03c7f`。未覆盖 R2、全 Native/feature/bridge、真实 GPU/PNG 全图、窗口及性能回归、物理输入/DPI、Narrator、真实跨应用剪贴板、Win11/8GB 或完整安全/产品验收。headless 绘图与 cfg(test) consumer 不能升级为这些证据，预算及完整产品门槛保持开放。
