# 产品命名调研（v0.2.0 准备）

> **不是法律意见，也不是商标清查（trademark clearance）。** 本文只是 2026-10-21 由 agent 做的公开网络检索记录，用于给用户挑选候选名。检索面有限（见 §5），"没搜到"只表示在所列来源里没观察到，**不代表**可以注册或不会侵权。正式改名（M5）前应由用户本人在 CNIPA 等官方库做近似查询，必要时咨询专业人士。
>
> 状态：`DOCUMENTED`（网络检索结果）。没有任何候选名被验证为"可用"。本文不改名，代码与文档中的 ShareReady 保持不变。

- 检索日期：2026-10-21；仓库基线 `6972ee740bfa80cded16b23408a1c99e14dbc8b5`
- 产品要点：Windows 桌面截图工具，截图 → 标注 → 遮挡（实心块 / 不含原像素的"安全马赛克"）→ 复制/保存；Rust 处理、纯本地、无网络、MIT 开源。
- 命名目标：暗示"干净 / 安全地分享截图"；短；中英文都好读；最好有中文名；中文用户优先，英文其次。

## 1. 为什么 "ShareReady" 不能作为正式名称

| 证据 | 内容 | 链接 |
|---|---|---|
| 已有同名产品 | **ShareReady**（iPhone 应用，Hawkwise Labs OÜ，© 2026）。自称"分享前处理层"：对人脸/车牌做模糊、马赛克或涂黑，重建像素以去除元数据，本地处理，有 Pro 付费档。**名称完全相同，定位（分享前图片隐私处理）高度重合。** | <https://shareready.app/> |
| 仓库早期已记录 | Stage 01 研究把它列为"同名且同样以分享前图像处理为定位"（C08） | [STAGE01_PUBLIC_EVIDENCE_LEDGER.md](research/STAGE01_PUBLIC_EVIDENCE_LEDGER.md) |
| .com 域名 | `shareready.com` 在 BrandBucket 挂牌出售 | <https://www.shareready.com/> |
| USPTO | 2026-10-21 在 tmsearch 文字商标检索 "shareready"：无结果 | <https://tmsearch.uspto.gov/> |

结论：**高风险**。即使美国库里暂无注册，对方是在售的同定位产品，且公司在欧盟（爱沙尼亚 OÜ），可能存在未注册的在先使用权或欧盟申请（EUIPO 未查）。同名同类会造成用户混淆，也妨碍搜索与应用商店上架。

## 2. 推荐候选（按推荐顺序）

风险等级只针对"已观察到的同名/近似冲突"，不是法律判断。

| 顺序 | 英文名 | 中文名 | 风险 | 一句话理由 |
|---|---|---|---|---|
| 1 | **Anshot** | **安截** | 低 | 安（安全）+ shot（截图），中英同音同义，短，未见同名软件或美国商标 |
| 2 | **Plainshot** | **素截** | 低 | plain = 素净、不藏东西；未见同名软件或美国商标；.app/.dev 未注册 |
| 3 | **Jiejing** | **截净** | 中 | "截净"谐音"洁净"，中文双关最强；但英文难读，美国已有 JIEJING 注册（他类） |
| 4 | **Blankshot** | **留白** | 低（冲突）/ 语义有隐患 | blank out = 涂掉；但英文 "blank shot" 有"空包弹"义，中文"留白"是通用词 |

建议组合：**Anshot · 安截** 作首选；若更看重中文意境，可用 **Anshot / 截净**（英文名与中文名不必逐字对应）。

### 2.1 Anshot · 安截（首选，低风险）

- 含义与发音：中文 "ān jié"，英文 "an-shot"，母语者都能读出且不易拼错。"安"直接表达"安全分享"。
- 同名/近似软件：未发现名为 Anshot / 安截 的软件（网页检索、GitHub、Microsoft Store 定向检索均未命中）。**近似音**：Unshot（截图美化编辑器，私测阶段）——<https://unshot.pro/>；功能是框架美化而非遮挡，但读音接近，需注意。另有 Angel Shot App LLC（人身安全 App，名字差异较大）。
- GitHub：`gh search repos anshot` 0 个结果。
- Microsoft Store：在 apps.microsoft.com 上的定向网页检索未命中（最接近的是 AiShot、AppShot）——仅为间接信号。
- 域名（RDAP，2026-10-21）：`anshot.com` 已注册（2021-03-11 起），跳转 GoDaddy 出售页；`anshot.app`、`anshot.dev` 注册局查询为 404（未注册）；`anshot.cn` 未查到 NS 记录（仅信号）。
- 商标：USPTO 文字检索 "anshot" 无结果；CNIPA、WIPO 未能完成（见 §5）。

### 2.2 Plainshot · 素截（低风险）

- 含义：plain = 朴素、坦白、不藏原像素；"素截"有"干净底子"的意思。缺点：plain 略平淡；与 Greenshot 同为"形容词 + shot"结构（不构成冲突，仅说明这类名字多）。
- 同名/近似软件：未发现。检索结果只出现 PixelShot（安卓截图管理）、PlainApp（安卓远程管理）、PlainSite（法律文件站）。
- GitHub：仅 1 个无关仓库（`bheaton1/plainsHotel`）。
- 域名：`plainshot.com` 已注册（2025-04-02 起），访问返回 403，用途未知；`plainshot.app`、`plainshot.dev` 未注册；`plainshot.cn` 未查到 NS 记录。
- 商标：USPTO "plainshot" 无结果。

### 2.3 截净 · Jiejing（中风险）

- 含义：截图 + 干净；与"洁净"同音，中文传播力最好。英文用户难读（"jie"），更适合作中文名搭配其他英文名。
- 同名软件：未发现名为"截净"的截图工具。注意腾讯有 Mac 截图工具"截图（Jietu）"，拼写较近但不同。
- GitHub：有若干同名小仓库（如 `boyley/jiejing`、`f4zjwy6j94-star/JIEJING-SAAS`、`ipconfiger/Jiejing`），均非截图工具。
- 域名：`jiejing.com` 2002 年起注册；`jiejing.cn`、`jiejing.com.cn` 有 NS 记录（已被占用）；`jiejing.app`、`jiejing.dev` 未注册。
- 商标：USPTO 有 **JIEJING**（第 7 类机械，Shenzhen Jiejing Technology Co., Ltd，LIVE REGISTERED，序列号 97815315）和 **JIEJING CRYSTAL**（第 14/21 类）。都不在软件类，但说明拼音"jiejing"在中国企业中常用；"洁净"作为通用词在 CNIPA 第 9 类的近似情况**未查**，可能面临显著性或近似问题。

### 2.4 Blankshot · 留白（冲突低，语义有隐患）

- 含义：blank out = 涂掉；"留白"是中文美学用语。缺点：英文 "blank shot" 常指空包弹/空枪，可能产生"不中用"的联想；"留白"是通用词，作商标显著性弱，且很多 App 用过类似字眼（未逐一核查）。
- 同名软件：未发现。GitHub 有一个无描述的 `yateendra/BlankShot`。
- 域名：`blankshot.com` 2014 年起注册，未见实质内容；`.app`、`.dev` 未注册。
- 商标：USPTO "blankshot" 无结果。

## 3. 已排除的名字（有冲突证据）

| 名字 | 风险 | 冲突证据 |
|---|---|---|
| SafeShot | 高 | 同类：Matheus Chiappina 的 SafeShot，"隐私优先、本地处理"的截图标注/模糊工具，支持 Windows（[winget 页](https://winget.ragerworks.com/package/Chiappina.SafeShot)）；另有同名 Chrome 扩展。USPTO 有多个 SAFESHOT 第 9 类有效注册（Bertrem Valve、L3 Technologies，非软件但同类别）。 |
| Cloakshot | 高 | 同类：<https://cloakshot.app/>（`.com` 308 跳转至此），截图/图片/PDF 遮挡后复制或保存、本地运行；两个域名都在 2026-09-29 注册。GitHub 另有 `TheGreatAndrew/CloakShot`（AI agent 屏幕隐私盾）。 |
| Veilshot | 高 | 同类：<https://veilshot.app/>，浏览器内截图工具，可模糊敏感信息后分享；GitHub 另有 `kukubanana/veilshot`（Linux 截图工具，MIT）。 |
| PixVeil | 高 | 同类：iOS "分享前模糊敏感内容"应用（第三方列表 [chrome-stats](https://chrome-stats.com/d/id6777958872)）。 |
| Masklet / MaskShot | 高 | 同类拥挤：MaskShot（[Product Hunt](https://www.producthunt.com/p/maskshot)，本地截图清理）、Mask Shot - Clean Screens（[App Store](https://apps.apple.com/us/app/-/id6760220410)）、Maskr。 |
| Tidyshot | 高 | Mac 截图整理工具，含分享前模糊敏感信息（[Setapp](https://setapp.com/apps/tidyshot)）。 |
| CleanShot | 高 | CleanShot X 是知名 Mac 截图工具；USPTO 另有多个 CLEANSHOT 注册（其他类别）。 |
| Snapveil | 中 | 活动照片分享 SaaS（[AlternativeTo](https://alternativeto.net/software/snapveil/about/)），"veil" 系列整体拥挤。 |
| Opaq | 中–高 | OPAQ Networks（云安全/SASE）2020 年被 Fortinet 收购（[报道](https://convergedigest.com/fortinet-acquires-opaq-networks-for-sase/)），同在安全软件领域；`opaq.com/.app/.dev` 都已注册。 |
| HushShot | 中 | Hush: Screen Share Focus（Mac 屏幕共享隐私工具，[AlternativeTo](https://alternativeto.net/software/hush-screen-share-focus/about)）。 |
| Shotblock | 中 | USPTO "SHOTBLOCK!"（枪支安全，已放弃，序列号 90075003）；iOS 库 ShotBlocker；枪械联想。 |
| 墨截 Mojie | 中 | 拼音同"魔戒"；GitHub 上 "mojie" 多为代理服务（"机场"）仓库，联想不佳；`mojie.com/.app/.dev` 都已注册。 |

## 4. 检索方法

| 来源 | 做法 | 结果性质 |
|---|---|---|
| 网页检索 | WebSearch（英文为主，部分中文），每个名字 1–2 次 | `DOCUMENTED`；搜索引擎面向美国，中文网（百度、小红书、应用宝等）覆盖弱 |
| GitHub | `gh search repos <name> --limit 5` | 只看仓库名/描述前 5 条 |
| Microsoft Store | 在 apps.microsoft.com 上的定向网页检索（商店搜索页需 JS，直接抓取无结果） | 间接信号，不等于商店内无同名 |
| 域名 | RDAP：Verisign（.com）、Google Registry（.app/.dev）；404 = 注册局无记录（已用随机域名验证）；.cn/.com.cn 仅查 DNS NS 记录 | 未注册 ≠ 可按普通价格购买（溢价/保留未查） |
| USPTO | tmsearch.uspto.gov 网页界面，文字商标"包含该词"检索；正向对照 "snagit" 返回 4 条（含 TechSmith 第 9 类注册），证明检索有效 | 只做了精确词，未做近似音/近似形检索，未看图形商标 |

## 5. 未能检查 / 未检查（NOT_VERIFIED）

- **CNIPA / 中国商标网**：`wcjs.sbj.cnipa.gov.cn` 对脚本访问返回 403，网站需交互验证；按规则不绕过验证码，因此**完全未查**。这是对中文首发影响最大的缺口。
- **WIPO Global Brand Database**：在内置浏览器中提交了检索，但结果一直停在"Loading/Processing"，连正向对照 "snagit" 也无结果，因此**未得到任何结果**。
- 未检查：EUIPO / TMview（与 ShareReady 的欧盟公司相关）、Google Play、Apple App Store 的系统性搜索、国内应用商店、Gitee、微信公众号 / B 站 / 小红书账号名、域名溢价价格。
- 未检查各名字在其他语言里的负面含义（仅凭常识排查英文与中文）。

## 6. 建议的下一步（M5 改名前，由用户决定）

1. 从 §2 选 1–2 个名字。
2. 用户本人在中国商标网做**第 9 类（软件）和第 42 类**的近似查询（中文名 + 拼音 + 英文名），并在 EUIPO / WIPO 补查。
3. 若确定，先注册可用域名（如 `anshot.app`）与 GitHub 组织/仓库名，再开始 M5 改名。
4. 改名时保留"名称曾为临时代号 ShareReady"的说明，继续不声称商标已清查。
