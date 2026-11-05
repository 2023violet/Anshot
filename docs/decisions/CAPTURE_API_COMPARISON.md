# 截屏接口比较：GDI / DXGI Desktop Duplication / Windows.Graphics.Capture

日期 2026-11-05 · 状态：**提议，待用户决定**（不改变产品；产品仍用 GDI）

## 为什么写这份文档
- 安截从 B2（提交 5c6d736，2026-10-19）起用 GDI 冻结屏幕：`BitBlt(SRCCOPY | CAPTUREBLT)` 加 `GetDIBits`，见 `crates/anshot-platform-windows/src/screen.rs`。
- 当时没有比较过别的接口。`01_PRD.md` 写了“屏幕采集覆盖层是难点，Gate 03 spike 必须覆盖”，但 Gate 03 的 spike 只比较了三个界面框架，没有测截屏。这一条被跳过了，是代理（实现方）的疏漏。
- 当时选 GDI 的实际原因是实现简单、满足当时的需求（Windows 10、单显示器、像素精确）：项目已有 `windows-sys`，一次同步调用就能冻结整个虚拟桌面。这不是对比后的长期平台决策。以上原因是事后按当时代码条件整理的，当时没有书面记录。
- 2026-11-05 用户报告 Windows 11 上截图时先黑屏几秒、整屏变暗偏红。本文件补上比较，并记录对黑屏的模拟排查。

## 方法（MEASURED，本机，代理执行）
- **环境：** Windows 10 19045；Intel UHD + GTX 1650 双显卡，屏幕由 Intel 驱动；1920×1080，120 Hz，SDR（DXGI colour space 0 = sRGB，8 bit，270 nits）。
- **截屏比较：** `spike/capture-api/`（Rust，`windows =0.62.2`，不进产品）。`run.ps1` 先铺一张置顶的合成测试图，再用三种接口各截主显示器 5 轮。每轮都新建再销毁，测的是冷启动。只输出耗时和像素差异统计，像素不落盘。
- **黑屏排查：** `spike/capture-api/overlay_probe.ps1` 启动 0.5.0 发布版 exe，按快捷键截图。每隔约 4 ms 用 GDI 缩小采样屏幕，只记亮度统计；同时记录截图层何时获得焦点、显示设置是否变化、Windows 的通知状态，以及 Esc / 右键能否退出。背景是合成的灰色窗口。

## 截屏接口结果
| | GDI（现用） | DXGI Desktop Duplication | Windows.Graphics.Capture |
|---|---|---|---|
| 冷启动截一帧（含创建与读回） | 34–44 ms | 65–84 ms | 107–121 ms（进程内第一次 425 ms） |
| 与 GDI 的像素差异（SDR，合成图） | — | 0 | 0 |
| 第一帧 | 即时 | **每次第一帧都是空的**（`LastPresentTime = 0`，约 98.6% 像素不同），要丢掉再取下一帧 | 正常 |
| 屏幕边框 | 无 | 无 | 在 Windows 10 22H2（Build 19045）实测环境下，截图时屏幕边缘出现**黄色边框**（实测到黄色像素）；`IsBorderRequired` 接口不可用，返回 `E_NOINTERFACE`。该属性按官方文档要求 Build 20348 起（Windows Server 2022、Windows 11）才提供（DOCUMENTED）；Windows 11 上无边框捕获所需的授权，以及未打包桌面程序是否适用，均未实测（NOT_VERIFIED） |
| HDR | 只能 8 位；HDR 屏上会失真（DOCUMENTED，未实测） | `DuplicateOutput1` 可取 FP16（DOCUMENTED，未实测） | 可取 FP16（DOCUMENTED，未实测） |
| 多显卡 / 多显示器 | 一次调用拿整个虚拟桌面 | 按显卡和输出分别复制，要找对显卡；本机输出在 Intel 上 | 按显示器分别建采集对象 |
| 代码量 | 小，同步 | 中，要处理访问丢失和空帧 | 大，WinRT + D3D11，异步取帧 |

另外：用 DXGI 的 `IDXGIOutput6::GetDesc1` 能读出当前显示器是否在 HDR 模式（colour space 12 = PQ / BT.2020）。产品可以据此提示或换路径。

## 黑屏模拟排查结果
| 场景 | 截图层获得焦点 | 黑帧 | 显示设置变化 | Esc / 右键退出 |
|---|---|---|---|---|
| 托盘待命，OpenGL 渲染 | 75–137 ms | 0 | 0 | 都正常 |
| 编辑器打开时 | 133–165 ms | 0 | 0 | 正常 |
| 软件渲染（`SLINT_BACKEND=winit-software`） | 75–118 ms | 0 | 0 | 正常 |
| CPU 满负荷 | 82–113 ms | 0 | 0 | 正常 |
| 截图层比屏幕高 1 px | 72–107 ms | 0 | 0 | 正常 |

结论：
- **整屏变暗、偏暖：确定是安截的截图遮罩。** 遮罩是 `Theme.scrim` `#26251e`，约 55% 不透明，未选区时盖住整屏。用户第五张照片正是截图状态。
- **黑几秒、要重启：本机复现不出来**（共 21 轮以上，0 黑帧）。截图层在任何组合下都会被 Windows 当成“全屏程序”（`SHQueryUserNotificationState` = 2），但所有全屏截图工具都一样，不能单凭这点定罪。
- **“托盘入口跳过等待”不是原因。** 代码确实如此（`main.rs` 的 `begin_capture`），但实测托盘入口和编辑器入口一样快、一样正常。
- **测量盲区：** GDI 采样看到的是 DWM 合成后的画面。显示器本身黑屏（例如显示模式或 HDR 切换）采样不到。显示设置变化的监听没有用真实的变化验证过。
- **剩余嫌疑**（HYPOTHESIS，只能在用户的 Windows 11 电脑上验证）：HDR 下的 GDI 抓屏；Windows 11 显卡驱动对全屏 OpenGL 窗口的呈现方式；高分辨率或缩放下的首帧。判断步骤见下。

## 在 Windows 11 电脑上的判断步骤
1. 记下 HDR 开关、分辨率、缩放、刷新率和显卡型号。
2. 复现一次。黑屏后先等 10 秒，再依次试 Esc、右键、任务管理器结束 Anshot，记下哪一步恢复。
3. 退出安截，用 `set SLINT_BACKEND=winit-software && "%LOCALAPPDATA%\Programs\Anshot\Anshot.exe"` 启动后再试。
4. HDR 开着的话，关掉 HDR 再试第 2 步。

**怎么判断：**
- 第 3 步好了：是 OpenGL 渲染与驱动的问题。
- 第 4 步好了：与 HDR 有关。这时换接口（DXGI `DuplicateOutput1` 或 Windows 11 上去掉边框的 WGC，加色调映射）才有意义。
- 只有结束进程能恢复：截图层在那台电脑上卡住了。

## 提议（待用户决定）
- 在 Windows 11 结果出来之前，不更换截屏接口。在 SDR 上，三者截到的像素完全相同，GDI 最快、最简单。
- **WGC 不宜直接替换 GDI。** 在实测的 Windows 10 22H2（19045）上每次截图都会出现黄色边框且无法关闭，冷启动也最慢。若采用，只在提供 `IsBorderRequired` 的系统（Windows 11）上、并实测确认能去掉边框后使用。
- **DXGI 适合作为 HDR 路径**，但要丢掉空的第一帧、找对显卡、处理访问丢失。
- 不论最后选哪个，都先加 HDR 检测（`IDXGIOutput6::GetDesc1`），至少在 HDR 屏上如实提示颜色可能不同。

## 局限
- 只有一台 Windows 10 SDR 单显示器的电脑。Windows 11、HDR、多显示器、混合缩放、4K 都没测（NOT_VERIFIED）。
- 耗时是这台机器的冷启动单帧数据，不代表其他机器。
- 由代理执行，不是真人测试，也不是专业审计。
