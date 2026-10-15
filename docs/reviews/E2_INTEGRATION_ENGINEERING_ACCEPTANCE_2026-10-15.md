# ShareReady E2｜真实 Rust/WASM 工作台工程验收报告

**2026-10-15 · E2_INTEGRATION_ENGINEERING_PASS / NOT_INDEPENDENT_SECURITY_REVIEWED / SYNTHETIC_ONLY**

## 一、交付范围

用户认可 V1 浅色 Precision Workbench 方向并要求推进下一阶段开发。独立 E2 分支 feature/e2-workbench-integration 从 E1 修复后提交 7a234b957c177156389fcb3b2e668a58aa357c0c 派生，独立 Draft PR #12（base：E1 分支），不修改 main、不合并 PR、不发布。

本批代码不再是纯 UI 模拟：E2 工作台直接调用真实 E1 Rust/WASM redact_web，生成全新 PNG bytes，随后使用浏览器 Image.decode 重新打开成品再提供复制与下载。包含：仅四份 SHA256 合成测试图的选择/拖放/粘贴、矩形新增/选择/移动/右下角 resize、区域列表、Undo/Redo、Fit/Zoom/Pan、输出失效状态、用户可辨认的错误文案与独立成品复核。

## 二、修复及精确代码基线

- E2 初次真实 CI #38761830660 在 T01/T02 均通过，但 T04 图片因人为白名单哈希少一个字符被拒绝；旧运行 FAIL 的范围仅为 E2 导入白名单校验。
- 纠错 commit 20dfc4f9932371166e6b8b76bd18c1fe576794ac 将 T04 SHA256 与冻结 fixture 清单完全一致，四样本首次通过完整重新构建。
- 回归增强 commit 3c2a858d04cb17aa0924eed121a72abdf703b3e4 增加真实鼠标移动、resize、撤销恢复与 zoom-fit 复验（4 份 fixture）。
- 本轮最近一次**真实从源码重建 CI**绑定上述 3c2a... 精确提交。[Run #38763089705](https://github.com/2023violet/ShareReady/actions/runs/38763089705) SUCCESS。
- 提供本地编译与运行说明 e2-workbench/README.md 的后续提交仅变动说明文档；该文档不影响通过 CI 的应用程序、测试或 Rust 源代码。

## 三、真实机器测试（不是文字演绎）

| 检查 | 结果 | 证据 |
|---|---|---|
| JS 模块静态语法、Python 脚本语法 | PASS | 全量 Run #38763089705 |
| E1 原生 Rust 单元 + codec 安全集成负向测试 | 3 单元、6 集成 PASS | 同一 Run Cargo --locked --offline |
| Rust wasm32 构建及 wasm-bindgen 模块 | PASS | 同一 Run 源码重建 |
| T01_CHAT：4 框真实鼠标操作、PNG 下载像素验证 | PASS | PNG sha256 23ca9baafb02af1298b5f3c91538c6dffafe8e1f3fe5bbc60f0b2cfede6af65a |
| T02_ORDER：3 框真实鼠标操作、PNG 下载像素验证 | PASS | PNG sha256 4f7713902a26c001efa2a3af063b4fa9559036be73c196dee48e37cbe7b7b548 |
| T04_EDGES：4 个边缘遮挡、PNG 下载像素验证 | PASS | PNG sha256 dabbada524174652d48118ecbb20273fc700183b982b776808c37266bd72085f |
| T05_JPEG_EXIF：2 框、编码/元数据及跨 decoder 保留像素检查 | PASS | PNG sha256 8bb43042c073f50d9e2dd305e4893d8a10118584efffde2616732d04fc6655da |
| 选区移动/右下角 resize/Undo 真实几何恢复，Zoom→Fit | PASS（四样本） | 浏览器真实 pointer events 和 window.e2Debug 快照比较 |
| 空遮挡复核禁止 / 非白名单输入拒绝 | PASS | 真实页面内文件控件与主按钮行为 |
| 编辑后旧 PNG output revision 无效、原成品 bytes 清除 | PASS | 回编辑删区后 revision 与 outputBytes 断言 |
| 剪贴板 Permissions-Policy 拒绝后的明确错误与下载回退 | PASS | 实际 Chrome 权限策略，未伪报复制成功 |
| 真实浏览器网络请求 | PASS，均 localhost/blob，未观察到远程输出传输 | Playwright request handler |
| 本轮独立第二审查人签署 | **PENDING** | E1 PR #10 reviews=0，截至报告时 |

**全量原始机器工件：** [GitHub Actions Run #38763089705](https://github.com/2023violet/ShareReady/actions/runs/38763089705)，[Artifact #11986283397](https://github.com/2023violet/ShareReady/actions/runs/38763089705/artifacts/11986283397)，ZIP sha256 16555fc07cd8a51805b5d4745d0f51b9f61d991de1a4566c2b13d3371063f946，预计 2026-11-14 UTC 失效。Artifact 记录四份实际 PNG、截图、e2-results.json、当轮 WASM JS 与二进制。

**加速验收交叉检查：** [Run #38763089756](https://github.com/2023violet/ShareReady/actions/runs/38763089756) PASS（使用先前 E1 已核验的 WASM 二进制，先检查 E1 源码与锁文件 Git Blob ）。

## 四、剩余工程/安全风险（不应从 PASS 中推导）

1. E1e R01–R10 的独立审查仍 TODO，尤其畸形图片、恶意 EXIF/ICC/chunk 与许可证/依赖审查，并未因此被批准。
2. E2 对所有真实图片仍默认禁止：冻结合成样本 SHA256 白名单有意保留。未发布处理私人图片的功能。
3. 浏览器 clipboard write 在部分环境缺乏真实独立 paste-back 证据；仅验证拒绝+回退，不宣称跨 Chrome/Edge/Firefox/Safari 完整支持。
4. 编辑状态的中心视图功能已跑通；键盘仅能选择工具/删除/历史，尚未完成全键盘区域移动/调整、触摸、多指缩放、低分辨率/高 DPI 的独立可用性评估。
5. Stage 01 的真人用户价值 A1 仍 OPEN，S05a 招募仍 BLOCKED_BY_RECRUITMENT。技术通过不能等同于市场需求成立。
6. 未通过独立安全审查前，E2 验收限定为合成 fixtures 的工程集成，不代表产物能安全处理真正个人数据。

## 五、下一开发批次 E2.2 / E3 建议

- E2.2：键盘可访问的选区移动/尺寸调整、坐标微调、DPR2/缩放和输入状态竞态的压力测试、Inspector 与画布移动反馈验证。
- 安全门禁：引入明确独立代码审核（至少 E1e R03 及 R01–R10 最终评估），解决 Clipboard paste-back 证据缺口。
- E3：只有在上述安全门禁和真实图片输入威胁模型完成后，再考虑逐步移除 SHA256 白名单，补齐 PNG/JPEG 一般用户输入与真实隐私保护安全测试。保留独立分支→Draft PR→CI→Review→用户批准合并。

**最终判定：E2_FIRST_INTEGRATION_ENGINEERING_PASS；SECURITY_REVIEW_PENDING；NO_MERGE / NO_RELEASE / NO_REAL_PRIVATE_IMAGES。**
