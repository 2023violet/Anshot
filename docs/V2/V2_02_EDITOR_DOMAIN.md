# SR-V2-02｜Rust Editor 领域模型与真实图片闭环

状态：ENGINEERING_INTEGRATED / RUST_CHROME_WINDOWS_CI_PASS（功能源码提交 cba5af81992ed49dabf8202fc4c52fb81a535d77，Actions Run #39004861114，两个作业 SUCCESS）。文档更新提交需重新进行其自身 CI，但未修改任何产品实现。不是人工 Windows / 安全认证。

## 一、功能与架构

Rust Editor 新增：
- 从真实 PNG/JPEG 源字节创建 Editor，使用 Core 先解码与归一 EXIF，错误输入替换保持旧文档完整；仅持有尺寸的文档不能生成虚假成品。
- 带单调 ID 的 Mask、原图半开像素框、Selection、移动、八方向 Resize、删除、清空和数值式键盘步进。移动保持区域宽高，Resize 不允许翻转或产生零像素框。
- 独立 GestureTicket：先 preview，再 commit 一条 Undo，或者 cancel；触发手势瞬间使旧成品和 pending Copy/Save 失效。
- Snapshot 管理 mask + Selection；Undo/Redo 恢复编辑状态但生成新的 revision，不复活旧 PNG；新的编辑清除 redo；历史容量限制 128、mask 数限制 1024。
- Viewport 由真实已渲染图框的浮点坐标映射来源图像像素，不假定 DPR 与 CSS zoom 相同；禁止 NaN/Inf/无效尺寸。鼠标拖拽区域采用向外取整与边界钳位。
- Review 调用 Rust Core 的真正图片源字节与当前 mask，编码新 PNG；只有匹配当前 DocumentId、Revision、OperationId 的 ticket 可以读取/确认这些 bytes。
- Save / Copy 必须基于已生成的当前 PNG。OS 实际复制/保存结果只能由未来受信任的原生宿主报告，Editor 不把一次尝试自动标记成功。

## 二、安全边界

无有效遮挡不得导出；保留 V1.3 25MiB/24MP 限制、EXIF 方向和无源元数据的 PNG 重新编码；旧版本的异步结果拒绝进入新状态。

系统剪贴板可能在用户换图前就已被 OS 写入。这种外部副作用无法自动撤回。Editor 负责拒绝旧回调冒充新图片复制成功，未来 GUI 仍要诚实说明剪贴板实际行为。

当前 Review=true 表示 Rust 已编码并验证 PNG，并不等于用户肉眼核对了全部未遮挡隐私。GUI 须提供真实成品复核流程。源图仅存于本地 Rust 进程内存，并不构成内存清除/全面安全认证。

## 三、验收

R01 新 Editor 全部单元测试与 Core 跨包集成测试；
R02 8 手柄、源图边缘、负向拖拽、i64 边界、1px 最小框；
R03 手势 preview 不入历史，commit 恰好一次、cancel 不提交；
R04 选择状态/撤销重做/清除 redo / 单调 Revision；
R05 旧 Review / Copy / Save / Gesture 在换图/修改后失效；
R06 Rust 真正 PNG 与 Core 当前遮挡结果一致；
R07 V2-00 五张冻结 RGBA 金样本严格匹配；
R08 旧版 Chrome、WASM ABI、Windows WebView2 native smoke 保持通过；
R09 独立审查最终 main SHA、唯一分支、无无关删除。

上述 R01-R09 不可用文档存在代替测试证据。最终 GUI/24MP GPU 性能/跨屏 DPI/Windows Narrator/真实剪贴板回贴仍属 V2-03 及后续人工验收，不能谎称已通过。

## 四、下一步

SR-V2-03：用 egui/eframe 首先做可运行的 Windows 原生图像画布和平台功能 POC，按照既定 G01–G10 测试；如果关键要求不能通过，比较 iced。此阶段不删除 V1 JS/C# 运行链路，不发布 Rust-only 产品。

## 五、真实机器证据

- [Rust/Chrome/Windows 双成功 Run #39004861114](https://github.com/2023violet/ShareReady/actions/runs/39004861114)，测试代码 SHA cba5af81992ed49dabf8202fc4c52fb81a535d77。
- Core 单测 5、旧 Codec 集成 9、Editor 单测 17、跨 Core 集成 2，合计 33 项通过。其余二进制/文档测试为 0 的 crate 不计入通过数量。
- V2-00 全图像素五个 golden 严格匹配，输出 SR_V2_00_FROZEN_GOLDEN_VERIFIED。
- Linux 合成资源与依赖盘点成功；该结果不是全面漏洞安全审核。
- 真实 Chrome 产品恢复/像素回归通过；Windows CI 运行原 WebView2 桌面 smoke 并生成未签名 x64 便携包，不能替代 Windows 物理鼠标、Explorer、DPI、Narrator 和跨应用剪贴板验收。
