# ShareReady｜S03 真实竞品浏览器试验 · 首轮结果

> **2026-10-15 更新 / 以补充诊断为准：** 后续真实 Chrome 检查发现 Redaction Tools 的手动遮挡效果与操作顺序有关：T01 采用“姓名→头像→通知”顺序导出的实际 PNG，四个预设目标区域中心均已遮挡且保留文本未改动；T02 的连续遮挡仍因当前自动化事件兼容性问题未完成。详见 [SR-RES-007 操作顺序与事件诊断](STAGE01_S03_POINTER_ORDER_DIAGNOSTIC.md)。首轮 2/4 与 2/3 数字仅描述**初始自动化输出**，绝非竞品的固有漏检率。\n\n
> **SR-RES-006 · 2026-10-14（PDT）/ 2026-10-15（UTC）**  
> **阶段：S03 IN_PROGRESS / 真实自动化试验已取得 / 公平对比尚未完成。**  
> 这是第三方产品的真实 Chrome 浏览器操作与生成文件核查，**不是参与者可用性研究**，也不是 ShareReady 自身的安全性测试。

## 1. 实际测试环境与来源

- 测试目标：<https://redacted.app/>、<https://redactiontools.com/redact-screenshot>，应用产品版本**未公开具体内部 build**；不能把浏览器版本误当网站版本。
- 操作系统：GitHub-hosted Ubuntu / Linux 6.17.0-1022-azure x86_64；Chrome 154.0.8037.97，headless，1280×900，en-US。
- AI 设置：不启用可选 AI；Redaction Tools 保留默认规则识别（实际自动发现虚构电话）。
- 输入：仓库 [fixtures/manifest.json](fixtures/manifest.json) 中的 `T01_CHAT.png`（4 个应遮挡区域、2 个保留区域）与 `T02_ORDER.png`（3 个应遮挡区域、1 个保留区域），全部虚构。
- 所有手动框都来自已知的真值坐标（oracle-assisted），**不衡量真人发现敏感信息的能力**。
- 页面初始访问：[run #38680949676](https://github.com/2023violet/ShareReady/actions/runs/38680949676)；4 次真实图片导入：[run #38681111604](https://github.com/2023violet/ShareReady/actions/runs/38681111604)。
- 导入—拖拽—实际下载—像素诊断：[run #38681392251](https://github.com/2023violet/ShareReady/actions/runs/38681392251)；真正 PNG/截屏/JSON [artifact #11948102022](https://github.com/2023violet/ShareReady/actions/runs/38681392251/artifacts/11948102022)，ZIP SHA256 `61a6793430cc528aab528f2456efbdf6b5ff28ece8bd7dc26f1b66555fc9b1c5`。
- 独立从导出文件重新绘制视觉检查表：[run #38681589811](https://github.com/2023violet/ShareReady/actions/runs/38681589811)。此处“独立”指另外运行的文件分析步骤，**不是独立评审人员**。
- Redaction Tools 定向复测（每次拖拽重新计算 canvas rect）：[run #38681695615](https://github.com/2023violet/ShareReady/actions/runs/38681695615)，[artifact #11948975741](https://github.com/2023violet/ShareReady/actions/runs/38681695615/artifacts/11948975741)，ZIP SHA256 `77553f50150527734f9bf2f756fc592f5e1161d2ac5030c8e010540644b132db`。

## 2. 实际下载的输出与诊断结果

| 竞品 | 合成任务 | 实际下载 | 目标区域中心发生改变 | 保留区域保持原像素 | 自动化耗时* |
|---|---|---|---|---|---|
| Redacted | T01 | PNG 成功 | 4 / 4；中心 RGB 均为 (0,0,0) | 2 / 2 | 8.148s |
| Redacted | T02 | PNG 成功 | 3 / 3；中心 RGB 均为 (0,0,0) | 1 / 1 | 7.486s |
| Redaction Tools（AI OFF） | T01 | PNG 成功 | 2 / 4；头像和规则检出的电话改变 | 2 / 2 | 6.811s |
| Redaction Tools（AI OFF） | T02 | PNG 成功 | 2 / 3；收件人和规则检出的电话改变 | 1 / 1 | 6.514s |

*耗时是 Selenium 类 **Playwright 脚本测量**，从调用文件输入到下载完成，包含固定等待与人工真值已知的坐标操作。**不是**用户任务时间，更不能用于计算“产品 A 比 B 快 X%”。

检查包含：下载文件可重新打开、源/结果分辨率一致、指定区域中心像素比较、保留区域像素检查与元数据的初步检查。**没有覆盖每一个源像素、Alpha、边界取整、EXIF 块结构、浏览器剪贴板重贴验证**，不构成完整安全审计。

## 3. Redaction Tools 不完整遮挡：归因仍未解决

- T01 页面最初显示 `1 hidden`，自动遮挡的是演示电话；首个手动拖拽头像后显示 `2 hidden`。随后针对姓名和通知的脚本拖拽未使隐藏计数增加，实际导出中这两项原像素仍在。
- T02 情况类似：规则识别电话，首个手动补框遮住收件人；随后针对地址的补框没有产生可检测改变。
- 针对“页面重排使鼠标坐标失效”假说，第二轮在**每次拖拽前**重新计算 canvas bounding box。T01 / T02 的 canvas 均维持 `x=133.75, y=225, width=1012.5, height=675`，且第二次之后 `2 hidden` 仍未增加。复测导出结果重复了缺失遮挡。
- **目前不能判断**是工具存在一个手动遮挡会话中的交互约束、Pointer 事件处理不同、自动化脚本缺少某步状态切换，还是工具本身在此环境的真实问题。
- 因为根因未分离，**不得写“Redaction Tools 漏检率 50%/33%”**。那会将“默认自动规则只检出电话”和“脚本后续拖拽不生效”混在一起，且排除了其可选 AI 与真人补救路径。
- 下一步应在真实桌面手动使用同一输入，验证是否能连续增加至少 3 个手动框，再以可观察录屏 / 文件为证据；如果真人正常，修复 Playwright 指针操作并重新测量；如果真人也复现，再作为受环境限制的可重复缺陷记录。

## 4. 网络隐私声明的观察边界

- Redacted：在本次 T01/T02 导入及导出会话中，自动化脚本没有观察到 POST 网络请求。**这不是对所有请求及隐私安全性的证明**。
- Redaction Tools：本次操作中观察到请求方法 `POST`，目标域名 `api.openpanel.dev`，在截图导入前后页面请求记录中出现。**未读取、分析其载荷内容**；不能推断已上传图片、用户资料或隐私文本，也不能以此单独推断其隐私声明真假。
- 如果未来将“无任何网络请求/无遥测”列为差异化指标，必须先冻结网络条件，再核查真实 request body、源站与第三方资源依赖，并征求必要的安全审查意见。

## 5. 研究结论与下一步 Gate

**已取得：** 两个真实竞品 + T01/T02 的导入成功、真实下载文件、输出像素和保留区域差分、重现性问题及 Chrome 环境。原 S03 至少两个竞争工具的“真实操作”前置条件取得了部分证据。

**尚缺：** Redaction Tools 交互根因隔离；至少一轮人工操作结果复核；可比的任务完成定义和重复试次；复制到剪贴板的最终数据核查；实验输出逐像素安全性核查；真实受试者。前述项目若缺失，不可声称已完成公平基线。

**当前判决：`S03_IN_PROGRESS / BROWSER_FILES_MEASURED / COMPARISON_INCOMPLETE / HUMAN_SESSIONS=0 / A1_OPEN`。** 禁止基于本表开始 Rust 应用编码、合并或发布。独立评审留待 S08。

**研究数据：** [STAGE01_S03_SCRIPTED_TRIALS.csv](STAGE01_S03_SCRIPTED_TRIALS.csv)；不要将它混入原只记录官方文档能力的 `COMPETITOR_EVIDENCE_MATRIX.csv`。
