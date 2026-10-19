# Background source intake Implementation Plan

> **For agentic workers:** Use superpowers:executing-plans to implement task by task. Root implements; independent subagent reviews. Continuous local authorization supplies agent gate confirmation, not human per-token review. No commit, push, branch, external or destructive action.

**Goal:** 大图导入的CPU准备在后台，取消/失败/关闭不丢当前编辑。

**Architecture:** Editor封闭预验证来源候选，Native单worker准备同源显示。来源与复核互斥，UI接受候选前检查身份/cap和当前丢弃确认；OS读取/对话框和GPU仍按明确限制记录。

**Tech Stack:** 既有Rust/Core/Editor/egui、std thread+sync_channel(1)，锁与依赖不变。

**Spec:** [DESIGN.md](DESIGN.md)

执行记录：Task1–3及独立代码/运行审查已完成，见 [RESULTS.json](RESULTS.json)。下列勾选保留原计划的执行历史；genuine验证、文档同步和不删除归档状态以 `target/workflows/p03-k` 为准，不能用勾选替代证据。

## Global Constraints

- main / expected HEAD94f7f419aee91faf6f846349d40a897d0d72d7d0，不改锁/V1/冻结goldens/总方案/PDF；保留J精确历史，不清理他人脏工作。
- Rust decode/export；25MiB源/24MP、DIB既有限额、生成物128MiB不放宽。固定合成输入；不读真实OS clipboard或用户图片。
- high genuine A/B、实际RED/GREEN、当前source/EXE/raw身份、独立审查。自有试验2GiB/120秒无进度、两失败重规划；不凑帧、不删除、不提交/发布。

## Review Focus

- 解码成功但接受时document_id溢出：旧source/stamp/masks/history/output全部不变（Task1）。
- 导入期间用户新增编辑/手势：候选到达按当时状态确认，错误/取消不得偷偷撤销手势（Task2）。
- source与review正在收尾或AwaitingDraw：另一入口不得读取外部来源或spawn，Copy/Save不得穿过busy（Task2）。
- 首次Empty之后send/exit与send后barrier：最终drain保结果且槽到线程结束，旧错误不覆盖新状态（Task2）。
- 无编辑但来源在途时的Close、modal期间完成、实际重复Close：不复活导入或交付，无额外系统动作（Task2/3）。

## Task1：封闭预验证领域来源

Files: modify `crates/shareready-editor/src/lib.rs`，本crate单测。不改Core像素实现或Cargo依赖。

Interfaces: `SourceCandidate::decode(Vec<u8>)->Result<Self,EditError>`、`bytes()->&[u8]`、`dimensions()->(u32,u32)`；不可Clone且字段私有。`Editor::from_candidate(SourceCandidate)->Result<Self,EditError>`、`replace_candidate(SourceCandidate)->Result<(),EditError>`；旧from_source/replace_source保持签名并复用。

- [x] 先加具名测试 `source_candidate_keeps_exact_validated_bytes_and_dimensions`、`source_candidate_replacement_revokes_old_review_and_history`、`source_candidate_overflow_preserves_entire_existing_document`、`source_candidate_invalid_input_preserves_existing_review`，含源bytes、stamp、mask/history/output/旧key和兼容路径断言。运行领域命令固定RED。
- [x] 最小私有候选与消费接口；Core完整decode仅在decode构造，接受不再decode。运行 `cargo test --locked -p shareready-editor` GREEN，原26+2保持。
- [x] 独立审查候选的不可伪造性、失败原子性、旧复核失效和同步ABI，记录精确源码/命令。通过后才Native接入。

## Task2：有界来源worker与普通入口

Files: add `poc/native-egui/src/source_task.rs`；modify `app.rs`、`display.rs`，按实际调用调整合成helper。保持J review_task接口及Core不变。

Interfaces: private `SourceInput::{File(PathBuf),Bytes(Vec<u8>),Clipboard(ClipboardImage)}`、唯一Arc key、私有`PreparedSource`绑定领域candidate/PreparedDisplay/label；`SourceTask::spawn/poll/key/is_finished`，容量1。PreparedDisplay新增source CPU构造，from_prepared沿用cap检查。App start_source/cancel_source/poll_source与review互斥；candidate到达后既有PendingAction::Replace消费领域候选。

- [x] 先写上述Review Focus barrier/RawInput决定性测试RED；加入panic/disconnect/current-vs-stale error/cap变化、无效源不改旧成品、拒绝多文件/trace/忙碌在实际reader之前的断言。
- [x] worker移动有界file read/PNG格式校验/DIB转换/两次Core来源decode与ColorImage/缩略图；UI仅检查上传、当前编辑确认和replace_candidate。取消保留槽；最终drain；Close terminal；16ms重绘。
- [x] 文件/拖放与有界clipboard owned bytes接入，不把OS读取移为未经验证的后台权限路径。旧同步helper等待真实App.show+结果，不删旧断言；普通和feature Native全workspace GREEN。

## Task3：真实导入窗口和全回归

Files: 新source合成diagnostic driver/example及`tests/native_oracle/run_source_checks.py`，现有等待helper仅必要调整；普通模式无遥测。

- [x] 固定grid24MP/noise9MP，调用真实App来源入口；非人工延迟的CPU区间至少2真实callback，记录所有原始帧和最大值、受控取消/编辑/迟到丢弃/重试。
- [x] busy/接受后GPU图绑定key/源；取消保持旧source与PNG，重试有效换图后实际复核/受控Copy/fullPNG oracle。真实窗口只操作自有合成过程；OSclipboard不使用。
- [x] 根/普通/feature/bridge、19检查器、J复核边界、八普通GPU态、9MP大生成物、五冻结goldens；原始工件完整保存，不声称没跑的浏览器/物理试验通过。

## Task4：独立审查与收口

- [x] 独立审查当前完整代码、反例、原始报告/图/事件与量测限制；问题先修复再验。
- [x] RESULTS/README/工件与文档完整清单，root/V2入口当前/历史区分。必要receipt只作身份重核，不替代真实回归。
- 流程收口：genuine verify/coverage/review、复盘、现场静态审计及不删除归档的实际状态见保留的流程记录；完整产品保持active，不purge/commit/push。


## 重规划依据

R1最终cap/异步完成及R2计时/受控文案的事实与选择，统一见 [规格重规划记录](DESIGN.md#r1最终接受能力与异步完成断言) 和真实genuine事件。失败与修前源字节全部保留；本计划不再复制该历史段落。
