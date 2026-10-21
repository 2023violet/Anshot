# 安截 Anshot

**中文** · [English](README.en.md)

**Windows 截图 · 标注 · 遮挡 · 分享——遮住的地方，导出里真的没有了。**

> 版本 0.2.0（首个公开版本，**未做代码签名**）。所有像素处理与导出都在本机由 Rust 完成；程序不联网、无账号、无遥测。
> “安截 / Anshot”是 2026-10 起使用的名称，只做过公开检索，**未做商标清查**（见 [命名调研](docs/NAMING_RESEARCH.md)）。原临时代号为 ShareReady，仓库地址暂未改名。

按 **Ctrl+Shift+A** 截图，加上箭头、文字、序号，用实心块或**安全马赛克**遮住不该公开的内容，再一键复制或保存。遮挡区域在导出时被替换成**不含原图任何信息**的像素，导出的是全新的 PNG（不带原图元数据）；画布显示什么，导出就是什么。

![编辑器：箭头、椭圆、直线、画笔与自定义颜色（合成示例图）](tests/desktop/evidence/release/19-new-shapes.png)

## 下载与安装

在 [Releases](https://github.com/2023violet/ShareReady/releases) 下载：

| 文件 | 说明 |
|---|---|
| `Anshot-0.2.0-win-x64-setup.exe` | 安装版：只为当前用户安装到 `%LOCALAPPDATA%\Programs\Anshot`，**不需要管理员权限**；开始菜单快捷方式；可在“应用和功能”中卸载 |
| `Anshot-0.2.0-win-x64.zip` | 便携版：解压后运行 `Anshot.exe` |
| `SHA256SUMS.txt` / `release-metadata.json` | 校验值与构建信息（提交号、工具链、“未签名”声明） |

**关于 Windows 提示“已保护你的电脑”**：本程序没有购买代码签名证书，首次运行时 SmartScreen 可能拦截。确认文件来自本仓库 Releases 且校验值一致后，点“更多信息 → 仍要运行”。校验方法：

```powershell
Get-FileHash .\Anshot-0.2.0-win-x64-setup.exe -Algorithm SHA256   # 与 SHA256SUMS.txt 对照
```

升级：直接运行新版安装程序覆盖安装（会自动关闭正在运行的旧版）。关于 → “检查更新”只是用浏览器打开 Releases 页，程序自身不联网。
卸载：设置 → 应用 → 安截 Anshot；设置与截图历史默认保留，卸载时可勾选一并删除。开机启动项会随卸载删除。

系统要求：Windows 10 / 11 x64。

## 功能

- **截图**：全局快捷键或托盘；每个显示器一个冻结画面；拖选区域（可拖边角调整、拖内部移动）、单击吸附窗口、Enter 截当前屏、放大镜。选区旁工具条可直接 **复制 / 保存 / 钉图 / 长截图 / 编辑**。
- **长截图**：选好区域后点“长截图”，慢慢滚动内容，程序在本机逐帧拼接（自动识别固定的页头页脚；滚太快会提示）。
- **钉图**：把截图钉在桌面最上层，拖动、滚轮缩放、调透明度，右键复制 / 保存。
- **标注**：箭头、直线、框、椭圆、画笔、多行文字（支持中文输入法）、自动递增序号、高亮；6 种预设色 + 自定义颜色（HSV / HEX，记住最近使用）；3 档粗细、5 档字号。按住 Shift 画正方形 / 正圆 / 45° 线。
- **遮挡**：实心（黑 / 白）或**安全马赛克**（颜色只由位置决定，与原图无关）。不提供可被还原的模糊或像素化。
- **编辑**：选择、移动、八向调整、撤销 / 重做、裁剪（可拖手柄调整）、缩放与平移、Tab 循环选择、方向键微调。
- **交付**：Ctrl+C 复制；Ctrl+S 另存为新 PNG，**永不覆盖已有文件**（同名自动改为“名称 (2).png”）。
- **截图历史**（Ctrl+H）：保存编辑器**复制或保存过的图片**——与你分享出去的那张完全一致（遮了就是遮后的，没遮就是原样）。默认开启、保留 50 张，可设 20 / 200 张或关闭，可一键清空；存在本机 `%LOCALAPPDATA%\Anshot\history`、**未加密**，卸载时默认保留。冻结的整屏画面、原图文件和编辑状态从不落盘；钉图的复制不记录。
- **设置**（Ctrl+,）：截图快捷键（4 种可选，被占用时会提示）、复制后是否收起编辑窗、历史张数、默认保存位置、开机启动（登录后在托盘待命）。

快捷键：V 选择 · A 箭头 · L 直线 · R 框 · E 椭圆 · P 画笔 · T 文字 · N 序号 · H 高亮 · B 遮挡 · M 安全马赛克 · C 裁剪 · 1–6 预设色 · 7 自定义色 · Del 删除 · Ctrl+Z / Y 撤销 / 重做 · Ctrl+T 钉图 · Ctrl+H 历史 · Ctrl+0 / 1 适应 / 100% · Ctrl+, 设置。

## 隐私与安全承诺

| 承诺 | 怎么保证 | 状态 |
|---|---|---|
| 遮挡区域不含原图信息 | Rust 渲染时直接替换像素；安全马赛克颜色只由坐标决定 | 单元测试 + 端到端逐像素读回（MEASURED） |
| 导出不带原图元数据 | 重新编码的 PNG，交付前强制回读校验只含 IHDR/IDAT/IEND | MEASURED |
| 不联网 | 无网络库；运行期采样网络端点为 0 | MEASURED（本机） |
| 不覆盖文件 | `CREATE_NEW` 写入，失败不留半成品 | 单元测试 + 真实另存对话框 e2e |
| 历史只存交付过的图片 | 只记录编辑器复制 / 保存成功的同一份 PNG 字节（可能含未遮挡内容）；整屏截图与原图从不落盘 | 单元测试 + e2e |

已做：恶意 / 畸形图片语料与 10 万次级变异测试（发现并修复了 JPEG 解码库的一个卡死与一个 panic，见 [安全测试](docs/SECURITY_TESTING.md)）、`cargo deny` 依赖审计、两轮独立**代理**安全审查。
**未做**：第三方专业安全审计、覆盖率引导的模糊测试、代码签名。请自行检查未遮挡的内容——本程序不会自动识别隐私信息。漏洞报告见 [SECURITY.md](SECURITY.md)。

## 从源码构建

需要 Windows 10/11 x64、Rust 1.85+（[rustup](https://rustup.rs)）。

```powershell
cargo test --workspace --locked                                   # 核心库
cargo test --locked --manifest-path apps/desktop/Cargo.toml       # 桌面应用
cargo build --release --locked --manifest-path apps/desktop/Cargo.toml
pwsh -File scripts/package.ps1      # 生成 dist/ 下的便携 zip、安装程序（需 NSIS 3）、校验值
```

架构（Rust 可信边界）：`crates/shareready-core` 解码 / 遮挡 / PNG 编码与回读校验 · `crates/shareready-render` 场景合成（导出与画布预览共用）· `crates/shareready-editor` 对象、撤销历史、版本令牌 · `crates/shareready-platform-windows` 屏幕采集、快捷键、剪贴板、不覆盖保存、开机启动 · `apps/desktop` Slint 1.18.1 界面。界面只发送意图、显示 Rust 渲染的结果。（内部 crate 仍沿用旧代号命名。）

## 测试与已知限制

机器验证均由开发代理用真实系统键鼠完成：单元测试、冻结金样本、30 余项端到端用例（另一进程读回剪贴板逐像素比对）、拼音输入、性能、安装 / 升级 / 卸载流程。详见 [验收报告](ui-design/07_UI_QA_REPORT.md)。

已知限制：
- 没有真人可用性测试（按项目决定以代理实机操作代替，二者不等同）。
- 只在单显示器、100% 缩放、Windows 10 上实测；多显示器 / 混合 DPI、125%/150% 系统缩放、Windows 11、讲述人朗读均**未验证**。
- 未签名；无自动更新（手动检查）。
- 长截图依赖内容按整像素滚动；带动画或视差滚动的页面可能拼接失败（会提示“没接上”）；超过 25 MB 的长截图会被截短并提示。
- 升级或卸载时若程序正在运行，安装程序会先询问再关闭它（未保存的编辑与钉图会丢失；静默安装 `/S` 不询问）。

## 许可

代码 [MIT](LICENSE)。界面使用 [Slint](https://slint.dev)（Royalty-free 2.0，“关于”中显示 AboutSlint）；字体 Noto Sans CJK SC（[OFL-1.1](assets/fonts/OFL-1.1.txt)）；第三方清单 [THIRD_PARTY_LICENSES.md](apps/desktop/THIRD_PARTY_LICENSES.md)。

## 文档

设计与验收（中文）：[PRD](ui-design/01_PRD.md) · [UX](ui-design/02_UX_ARCHITECTURE.md) · [视觉与框架选型](ui-design/03_VISUAL_DIRECTIONS.md) · [设计系统](ui-design/04_DESIGN.md) · [高保真](ui-design/05_HIGH_FIDELITY/README.md) · [实现记录](ui-design/06_IMPLEMENTATION.md) · [验收报告](ui-design/07_UI_QA_REPORT.md) · [代理产品评审](ui-design/07_AGENT_PRODUCT_REVIEW.md) · [更新日志](CHANGELOG.md)。
历史：V1.3 网页版与 egui 样机作为冻结参考保留在 `web-app/`、`desktop-windows/`、`e1-spike/`、`poc/`（tag `pre-redesign-2026-10-19`）。

**请勿在 issue 中附带真实的私人截图。**
