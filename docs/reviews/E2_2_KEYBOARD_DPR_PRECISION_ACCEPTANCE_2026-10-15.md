# ShareReady E2.2｜键盘精修与真实缩放坐标回归验收

> **2026-10-15 · E2_2_ENGINEERING_PASS / SYNTHETIC_FIXTURES_ONLY / E1E_INDEPENDENT_REVIEW_PENDING**
>
> 本文件由实施方整理 GitHub Actions 实际运行结果，**不构成独立安全/无障碍审核签署、PR 合并批准或真实私人图像处理许可**。

## 1. 精确基线

- 源码仓库：`2023violet/ShareReady`
- 分支：`feature/e2-workbench-integration`，Draft [PR #12](https://github.com/2023violet/ShareReady/pull/12)，以 E1 源分支为 base。
- 实际 CI 运行所核验的源码 commit：`01f49607a6e8ab1747963f28eec1b4ba07dd9543`。
- Rust 核心未更改；本轮只调整 `e2-workbench/app.js`、`e2-workbench/index.html`、新增 `e2-workbench/tests/verify_e2_precision_browser.py`，并在两条已存在的 E2 CI workflow 中增加验证。
- 经过人类确认的浅色 Precision Workbench UI 方向保持不变，仍只接受四个 SHA256 冻结合成 fixture。

## 2. E2.2 缺陷、修复与验收

| ID | 目标 | 实施情况 | 机器证据 |
|---|---|---|---|
| E2.2-01 | 选区通过方向键按原图整数像素移动 | 每次按键移动 1px、裁剪到图片边界，并入 Undo/Redo | Chromium 1440 / 1024 / 800 |
| E2.2-02 | Shift+方向键调整选区边界，防止收缩为 0 面积 | 源像素右/下边界至少保留 1px | 同上 |
| E2.2-03 | 保证选中区域操作后焦点可继续使用 | Render 重新创建 DOM 后归还焦点，隐藏 Inspector 时改为聚焦实际可见的遮挡块 | RED [Run #38765534478](https://github.com/2023violet/ShareReady/actions/runs/38765534478) → GREEN [Run #38765702642](https://github.com/2023violet/ShareReady/actions/runs/38765702642) |
| E2.2-04 | 页面缩放与设备 DPR 不破坏源像素选区映射 | Pointer 映射依据实际 `getBoundingClientRect().width / sourceW` 和高度比例，不再只除以内部 app zoom | DPR 2；CSS 页面 zoom 125% / 150% / 100% |
| E2.2-05 | 真正导出的新 PNG 与源像素选区一致 | 三种视口上的导出 PNG 相同 SHA256，逐点检查遮挡矩形四个角及保留像素 | 每次均为 `dfdcd24b6bbdd16e5b8c562f514b63ce602dbe3080696131631b0bc26699554e` |
| E2.2-06 | 不破坏已有 E2 正向流程 | T01/T02/T04/T05 真实 Rust/WASM 全新 PNG、回退路径、旧产物失效和阻止非白名单导入 | 全源码 CI 通过 |

**RED/修复：** 首轮 commit `5c9f0f7beda9b0934f67dc785c8db71a75ecaf4b` 的快速 CI，在 1440/1024 上通过，但 800px 下 Shift+ArrowDown 不生效：隐藏 Inspector 的 DOM 节点获得焦点后导致键盘事件不再归属选区。针对该缺陷，将焦点恢复目标改成实际可见的遮挡区域，修复提交 `01f49607a6e8ab1747963f28eec1b4ba07dd9543`。

## 3. 真实 GitHub Actions 全量验证

| 工作流 | SHA | 结果 | 证据 |
|---|---|---|---|
| [加速浏览器回归 #38765702642](https://github.com/2023violet/ShareReady/actions/runs/38765702642) | `01f496...` | **SUCCESS**：四 fixture、键盘、DPR2、CSS zoom、PNG 及禁止远程请求检查通过 | [Artifact #11990280297](https://github.com/2023violet/ShareReady/actions/runs/38765702642/artifacts/11990280297)，ZIP sha256 `16386d2a11c878afebc05e610680fb53ef9734a898cf56ee4e6060135d8d2602` |
| [Rust/WASM 从源码重建 #38765702714](https://github.com/2023violet/ShareReady/actions/runs/38765702714) | `01f496...` | **SUCCESS**：Rust 3 unit、6 security integration；重新编译 wasm-bindgen；四 fixture 实际 Chromium 编辑/导出及三个高 DPI CSS zoom 场景全部通过 | [Artifact #11990120471](https://github.com/2023violet/ShareReady/actions/runs/38765702714/artifacts/11990120471)，ZIP sha256 `8b43164e2bf7783de242f13e3ff1bbb111219f32335e9708df4fbe326e2bb451` |

三组截图场景：1440×900/DPR2/CSS zoom125%、1024×768/DPR2/CSS zoom150%、800×820/DPR2/CSS zoom100%。这属于 **CSS 页面缩放模拟**，不能直接等同于各浏览器 GUI 缩放设置的独立兼容性认证。测试程序检查了真实 PNG 编码的像素、Undo/Redo、网络请求仅到 localhost/blob，以及剪贴板权限拒绝下的下载回退。

## 4. 待完成的明确范围

- **E1e 独立安全审查仍无正式 review（0）**。E2 CI PASS 不能改变此事实。
- 无法保证真实私人截图已经安全；产品仅允许经哈希校验的合成输入。不能删除这条白名单来冒充正式交付。
- 未实现完整屏幕阅读器兼容、focus trap 全面审计、全浏览器（Edge/Firefox/Safari）Clipboard round-trip、真机设备 DPR 测试与移动端触控体验。
- 原 Stage 01 A1 真实用户需求未验证；S05a 招募仍 BLOCKED。
- **NO MAIN WRITE / NO MERGE / NO RELEASE / NO PRIVATE IMAGES**。

## 5. 下批建议

先安排 E1e 与 E2 的非实施者独立审查，再以此为依据决定是否允许对非合成输入开放；同时可以继续隔离环境中的 UX 质量改进，如屏幕阅读器焦点、键盘创建遮挡、浏览器系统级缩放与 Clipboard paste-back，始终维持 Draft PR 和不发布边界。
