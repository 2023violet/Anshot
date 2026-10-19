# P03-J：有界后台复核设计

状态：设计候选，尚未实现；仅使用已批准 V2 连续本地授权，不代表人类逐 token 或本设计审阅。生产影响按 high 档执行，genuine Gate A/B 和独立审查仍必需。作者工作流 main 已于 2026-10-18 再查为 `1eb9624ee2037e4a2b253ee8120fa274dad1ef21`、v4.1.0 candidate，无版本升级。

## 问题与范围

[P03-I](../P03-I/README.md) 当前设备二十轮复核 controller 中位 1778.871ms、最大 1904.7852ms，主线程同步执行 Core 编码及再次显示解码。目标是在复核期间保持事件处理，并显示诚实的忙碌与取消反馈；不以该批冻结 100ms 物理反馈预算。保留当前 PNG 真值、版本凭证、源 25MiB/24MP、生成物 128MiB 与完整 PNG 校验。

这是 Editor 与 Native 之间的任务接口调整，按接口级设计处理。仅改后台复核，不把导入、OS 复制/保存、文件对话框或其他操作同时异步化；不加运行时/依赖、线程池、缓存或自动重试，不改 V1、金样、锁、主方案/PDF，不提交/推送/发布/删文件或改权限。物理双屏、Narrator、真实 OS clipboard 和完整产品门槛继续开放。

## 方案比较

1. **选择：单个标准库后台任务＋封闭的 Rust 复核结果。** 原始压缩源已是 Arc，可共享不可变输入；只给当前任务一个结果槽，事件线程用 try_recv，不等待。主要风险是迟到结果和准备显示像素的耗时，需要明确身份和实际忙碌图/时间证据。
2. 先显示“正在生成”再在下一帧同步编码，可让第一条消息出现，但之后约两秒仍不能处理事件，不满足本批目标。
3. 通用异步运行时/工作池或全操作队列更适合大量任务；本产品当前单图单复核，增加依赖、取消和并行内存复杂度，没有本批依据。

## 领域接口与身份

在 `shareready-editor` 中增加不可 Clone、字段私有的 `ReviewWork` / `ReviewResult`，以及字段私有、可 Clone 的 `ReviewKey`（仅共享不可变源身份和当前ticket，不共享Editor）：

- `Editor::prepare_review(&mut self) -> Result<ReviewWork, EditError>`：检查无手势、有效遮挡和已有操作；分配当前 Review Ticket，立即撤销旧成品和交付凭证；捕获 source Arc、精确尺寸和区域值，只预留操作，不编码。
- `ReviewWork::key(&self) -> ReviewKey`；`ReviewKey::ticket(&self) -> Ticket`；`ReviewWork::run(self) -> Result<ReviewResult, EditError>`：用 Core 生成 PNG，结果内部保留 key、精确尺寸及生成字节，外部不能构造任意成品。UI/worker在move工作前持有同一sealed key，错误、panic/断连、启动失败也不丢失所属源身份。
- `ReviewResult::bytes(&self) -> &[u8]`、`dimensions(&self) -> (u32,u32)`：只读，供显示准备，不代表 Editor 已复核。
- `Editor::review_is_current(&self, key:&ReviewKey) -> bool`：pending ticket、document/revision/operation/kind和当前源 Arc 身份全部匹配。`cancel_review(&mut self,key:&ReviewKey) -> bool` 只撤销该当前工作；错误/断连清理必须用key，不能只拿数值ticket执行finish或更新status。
- `Editor::accept_review(&mut self, result: ReviewResult) -> Result<Ticket, EditError>`：仅在key当前时安装 pending PNG，否则返回新增 `EditError::StaleReview` 且不改新状态。返回ticket仍需UI完成当前PNG的绘图帧后finish；尚未完成前禁止Copy/Save。
- 保留同步 `begin(Review)` ABI，内部复用 prepare/run/accept；run/accept失败须用所属key清理自己预留的pending，不能永久OperationPending。E1与现有领域调用无需改成线程。Copy/Save边界保持。

源 Arc 身份检查必须使用所有权身份（例如 ptr_eq），不以像素哈希或只有数值 stamp 代替：两个新 Editor 都可能从 document1/revision1/operation1 开始。测试必须包含“数字相同、不同 Editor/源 Arc”的拒绝。编辑、撤销、换图、开始手势和取消后，旧 result 一律不能安装；拒绝旧 result 也不能清掉较新 pending 或当前状态。

## 后台工作与 UI

Native 最多保留一个worker/receiver，即使取消接收后也等旧worker自行结束，再允许下一次复核；没有等待队列。标准库thread Builder与容量1 channel足够。**收到消息与线程结束是两件事**：slot保留JoinHandle与已收消息，只有is_finished为真才join收尾并释放slot；绝不join仍运行的线程。锁定Rust文档说明is_finished后join预期快速，但不承诺绝对零等待，收尾耗时需记录。测试在send之后用barrier继续阻住线程，证明收到结果仍不产生第二worker。

worker只做Core和CPU显示准备，不持有Editor/UI可变引用，不访问对话框、OS clipboard、保存路径或trace。Native只允许私有 `PreparedReview` success envelope，由同一次ReviewResult及其bytes生成PreparedDisplay，不能将PNG/display作为独立参数重配或仅比较尺寸。消息发送后请求重绘；busy每16ms请求一次后续重绘，使panic/断连没有正常send时也可发现。UI每帧try_recv，失败清理与status更新也必须review_is_current(key)，不得污染新Editor。启动/编码/断连失败可恢复，不以原图兜底。

将复核结果的CPU显示准备最小分离为 `PreparedDisplay`（私有字段：完整ColorImage、缩略图和捕获的图片有效cap）。`prepare_generated(bytes,dimensions,cap)` 在worker解码/转换/生成缩略图；`DisplayImage::from_prepared(ctx,prepared)` 只上传。cap精确区分device、图片有效上限（诊断可能512）、thumbnail=min(图片cap,2048)；接收时图片有效cap不同则拒绝并提示重新复核，不在UI悄悄重解码。保留decode/decode_generated同步入口。这里的“只上传”限定结果接收；现有非Fit可见裁片仍在UI复制/上传、导入和OS交付仍同步，本批不承诺所有回调小于100ms。NEAREST/Fit合同不变。

任务启动同帧显示“正在生成复核图”，Copy/Save禁用，提供“取消生成”。取消仅表示不再接受结果，不能声称线程立刻停止；旧worker还运行时提示“已取消，后台任务正在结束”。编辑/缩放与事件循环保持可用，状态不与PendingAction混用。换图/关闭确认模态存在时延后结果应用：取消确认后可继续原当前工作，丢弃则key失效；窗口关闭不join运行线程或触发交付。每份待接收结果最多128MiB PNG和当前24MP显示数据，保留2GiB自有子进程保护线。

结果接收/上传后进入 `awaiting-draw`，不得在同一帧accept后立即finish。按锁定egui0.36.2 `Context::run_dyn`，cumulative_frame_nr在全部pass结束后递增。每次App::ui重置本pass绘图标记，只在本success envelope的当前PNG纹理实际进入正面积canvas clip、无pending modal时记录key/frame/本显示实例与cap；多pass覆盖标记，当前累计frame不推进就不能finish。下一frame核对最后标记与当前key、PNG/display实例和cap仍一致、无模态，才finish(true)。编辑/取消/换图或上传失败立即清标记。此协议仅证明上一完整egui run的最后App pass排入了当前PNG图元，不证明GPU/物理呈现。

在App::ui入口捕获本帧开始时是否busy/awaiting，交付拒绝按cumulative frame持锁，跨该frame全部pass保持；不得仅用一次ui调用的局部bool。即便首pass入口完成finish，后续pass的Copy/Save仍消费拒绝，按钮也不能提前交付，下一安全frame才开放。接收结果同帧与busy期间输入不保存、不重放到完成后。测试包括poll成功+同帧Copy、首passfinish后第二passCopy、首pass有图后discard而finalpass无图、隐藏canvas及编辑/模态。

## 验收与证据

1. 风险A1先验：prepare只预留并立即返回；run/accept/finish分段；跨Editor同数字、错操作/改版/换图/取消/新操作均拒绝，错误key也不能清掉另一Editor的pending。无结果时bytes_for为空/交付不可用，同步begin失败后可恢复。
2. 可控barrier在worker发送结果之后继续阻塞，证明单slot仍被占用；取消/编辑可处理，不靠睡眠。失败/断连/启动失败及cap变化可恢复，旧成功和失败不改新状态。成功envelope不能重配PNG/display；当前PNG经历完成绘图帧之前及discard/mode/隐藏canvas不得晋权。
3. 实际release、固定6000×4000合成PNG和9MP合成JPEG运行普通新路径：至少30不同累计UI帧须落在真实worker工作开始/完成之间，不能将多pass、已完成结果等待、截图回读或人为延时补入；不足则保留真实数量并重规划，不填充。busy/最终截图绑定请求时的key公开编号（不记录指针）、阶段、frame与PNG身份，回调拒绝错配。记录触发/返回、worker开始/编码完成/显示准备完成、send/receive、is_finished/join收尾、绘图/晋权、取消/丢弃理由、cap、样本/产物/EXE/source_map及WS/OSpeak；单位明确，分开worker总时长、UI回调、轮询、上传和GPU读回。测取消后换图的峰值/尾部。后台不等于总耗时加速，不冒称物理响应。
4. 更新原有视觉/性能诊断，仅等待真实异步完成，不重复启动、不跳过PNG状态、不放宽像素容差；基于墙钟保护待完成任务，不以不受控重绘帧计数误判卡死。实际 PNG 全图、19检查器、普通八图、五冻结金样、Core/Editor/Native/桥接回归与当前大成品/受控Copy保持。需要改共享Core时才扩展WASM重建，否则保持I的Core/WASM身份并如实说明未复跑。
5. 最终精确源码/普通EXE/诊断EXE/流/图绑定，独立审查、文档同步及不删除归档。忙碌反馈未实际观测或迟到结果边界失败不能算本批完成。

## 止损与未决假设

A1：封闭结果加当前source Arc/ticket能在不 Clone Editor的前提下安全接受后台产物，先用领域反例验证。A2：CPU显示准备分离后UI只做有界上传，实际24MP忙碌帧可观察；仍是HYPOTHESIS，不预先声称100ms或全部24MP合格。A3：一个worker的内存与取消尾部可接受，需同源测量与失败恢复；若不成立先记录、缩小或重规划方案，不扩大队列/预算。

遇到错误源/旧结果被接受、未显示真实PNG却可交付、worker无限堆积、超2GiB或120秒无进度，只停止本批自有子进程并保留证据；同一步两失败重新规划。不得杀用户进程、提权或用真实用户图片取证。


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
