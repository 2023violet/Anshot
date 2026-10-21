# 安全说明 / Security

## 报告漏洞 / Reporting a vulnerability

请**不要**在公开 issue 中描述可利用的细节，也**不要附带真实的私人截图**。请使用 GitHub 的 [私密漏洞报告](https://github.com/2023violet/Anshot/security/advisories/new)（Security → Report a vulnerability），说明版本号、复现步骤（使用合成图片）和影响。这是个人维护的学习 / 作品集项目，没有承诺的响应时限。

Please do **not** post exploitable details in a public issue, and never attach real private screenshots. Use GitHub's [private vulnerability reporting](https://github.com/2023violet/Anshot/security/advisories/new) with the version, reproduction steps (synthetic images only) and impact. This is a personally maintained learning / portfolio project with no guaranteed response time.

## 适用范围 / Scope

最关心的问题 / Most relevant:
- 遮挡区域在导出（复制、保存、历史）中泄露原图像素，或导出带有原图元数据。/ Covered areas leaking original pixels into any export, or exports carrying source metadata.
- 打开、拖入、粘贴恶意图片导致崩溃、卡死、越界或过量内存。/ Crashes, hangs or excessive memory when opening, dropping or pasting a crafted image.
- 覆盖了已有文件；程序发起网络连接。/ Overwriting an existing file; any network connection made by the app.

## 已做与未做 / What has and has not been done

已做：本机恶意样本语料与变异测试（[docs/SECURITY_TESTING.md](docs/SECURITY_TESTING.md)）、`cargo deny`、独立**代理**安全审查（非专业审计）。
未做：第三方专业审计、覆盖率引导模糊测试、代码签名。
Done: hostile corpus and mutation tests, `cargo deny`, independent **agent** security reviews (not a professional audit). Not done: third-party audit, coverage-guided fuzzing, code signing.

## 测试用环境变量 / Test-only environment variables

发布版仍会读取 `ANSHOT_SETTINGS_DIR`、`ANSHOT_HISTORY_DIR`、`ANSHOT_RUN_KEY`（自动化测试用来隔离设置、历史与开机启动项）。它们只能由你自己的用户环境设置，不跨越任何权限边界；若被设置，设置 / 历史可能存到别处（例如同步盘）。正常使用不要设置它们。
Release builds still read `ANSHOT_SETTINGS_DIR`, `ANSHOT_HISTORY_DIR` and `ANSHOT_RUN_KEY` (used by the automated tests to isolate settings, history and the start-at-login entry). Only your own user environment can set them and they cross no privilege boundary, but if set, settings or history may be stored elsewhere (e.g. a synced folder). Do not set them in normal use.

只有最新发布版本会获得修复。/ Only the latest release receives fixes.
