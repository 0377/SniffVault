# 受控 WebView + 嗅探闭环设计（Plan 6a）

**日期**: 2026-09-07  
**状态**: 待用户审阅  
**前置计划**: Plan 1–5（已完成）  
**后续计划**: Plan 6b 系统分享 / 深链、Plan 7 LAN Cast + TV  
**父规格**: `docs/superpowers/specs/2026-08-11-app-ui-player-design.md`、`docs/superpowers/specs/2026-08-11-flutter-ffi-design.md`

---

## 1. 目标

在 Plan 5 已交付的「粘贴 URL → 解析 → 下载 → 片库播放」之上，补上 **必须登录或必须在页面内才能取到媒体** 的路径：用户在应用内打开自己输入的页面、完成登录、把 Cookie/Referer 交给现有解析器，或把页面网络请求交给现有 `sniffUrls`，再走同一套入队与下载。

**Plan 6a 验收一句话：** 用户在 Android / iOS / Windows / macOS 打开内置浏览，登录后能「解析本页」或从嗅探候选入队；带鉴权的下载在重启后仍带同一 Cookie/Referer 快照；设置可清除浏览 Cookie；不含系统分享、不含 LAN/TV、不做片源导航浏览器。

### 1.1 范围决策

| 纳入 Plan 6a | 留给后续 |
|--------------|----------|
| 主导航「浏览」+ 受控 WebView（地址栏、前进后退、刷新） | 系统分享、iOS Share Extension（Plan 6b） |
| `NeedsBrowser` 一键打开同一浏览页并带入 URL | LAN 投送、Android TV Leanback（Plan 7） |
| 导出 Cookie/Referer/`page_url` 调用 `resolveUrl` / `resolveQualities` | 多标签、书签、历史云同步 |
| 原生网络钩子 → `SniffEvent` → `sniffUrls` → 候选入队 | 广告过滤、站点插件、系统级抓包 |
| 任务级 Cookie/Referer 持久化，供 worker 下载 | DRM 绕过、账号体系 |
| 设置「清除浏览 Cookie」 | 海报、字幕、PiP |

**不选完整 Plan 6（含 Share）**：分享只是把 URL 填进 `/add`，不解决 `NeedsBrowser`。  
**不选先做 Plan 7**：投送依赖片库已有缓存，主路径仍断在取流。  
**不选纯 Flutter 拦截**：子资源（HLS、XHR）大量不可见，嗅探会长期残缺。  
**不选四端各写一套原生浏览器**：与 Plan 5 Flutter 壳分裂，后续分享/TV 更难收。

### 1.2 产品原则

- **受控 WebView，不是浏览器产品**：无站点目录、无官方收藏夹、不引导用户「去哪看」。用户只打开自己输入或从添加流带入的 URL。
- **两条取流都接到现有向导**：解析结果仍用 `ResolveWizard`；嗅探结果映射为 `ResourceCandidate` 后走与 `Candidates` 相同的入队路径。
- **Cookie 仅本机**：浏览 Cookie 存在 WebView 平台 Cookie 仓；入队时把 **快照** 写入任务库供下载。列表/事件 JSON **不得**带出 Cookie。投送/导出（Plan 7）不得带鉴权头。
- **禁止系统级抓包 / VPN / 透明代理**（与项目定位一致）。只允许 **进程内** WebView 请求观察。

### 1.3 与既有计划的边界

| 计划 | 本规格的关系 |
|------|----------------|
| Plan 3 | 不改 `resolve` / `sniff` 分类语义；复用 `ResolveOptions`、`SniffEvent`、`sniff_urls` |
| Plan 4 | 允许 **最小 FFI 扩展**：入队附带可选下载鉴权；`list_tasks` / 任务事件对 Cookie 脱敏 |
| Plan 5 | 五栏导航；`NeedsBrowser` CTA 改为打开浏览；`EngineRepository` 补上已有的 `sniffUrls` |
| Plan 6b | `/add?url=` 已预留；本规格不实现系统分享 |
| Plan 7 | 浏览入口在 Android TV 上隐藏；不在此期做 Leanback |

---

## 2. 用户流程

### 2.1 导航与路由

窄屏 `NavigationBar` / 宽屏 `NavigationRail` 五项，顺序固定：

**片库 | 浏览 | 任务 | 添加 | 设置**

| 路径 | 页面 | Shell |
|------|------|--------|
| `/browse` | `BrowseScreen` | 显示 |
| `/browse?url=<encoded>` | 打开时加载该 URL | 显示 |
| `/browse/wizard` | 复用 `ResolveWizard` | 全屏，无 Shell（与 `/play` 相同 `parentNavigatorKey`） |
| 现有 `/library` `/tasks` `/add` `/settings` `/play/:episodeId` | 不变 | 播放器仍全屏无 Shell |

Android TV（`uiMode` 为 television）：**不展示「浏览」destination**，也不注册可用的 `/browse` 业务页（深链进入则提示「请在手机或电脑使用内置浏览」）。`NeedsBrowser` 在 TV 上保持说明 + 返回，不提供打开浏览按钮。

### 2.2 浏览页（受控 Chrome）

必备控件：地址栏（可编辑）、后退、前进、刷新、**解析本页**、**嗅探候选**（列表或底栏，有候选时可见数量）。

地址栏规则：

- 允许 `http` / `https`。
- 拒绝 `file:`、`javascript:`、`data:` 作为用户提交的导航目标（粘贴后提示非法地址，不加载）。
- 空地址或 `about:blank` 时「解析本页」禁用。

WebView 的 `User-Agent`：若 `EngineSettings.user_agent` 非空则使用该值，否则平台 WebView 默认 UA。须与随后 `resolveUrl` 使用的引擎 UA 一致（引擎侧已有 settings UA）。

### 2.3 从添加到浏览

`NeedsBrowser` 主文案改为：「此站点需要在内置浏览中打开并登录后再解析。」

按钮：**打开内置浏览**（导航 `/browse?url=<原解析 URL>`）+ **返回**。

### 2.4 解析本页

1. 读取当前主框架 URL 为 `page_url` 与 `referer`。
2. 导出该 URL 作用域下的 Cookie，格式为 HTTP `Cookie` 头：`name=value; name2=value2`（与 Plan 3 R6 一致）。
3. `resolveUrl(page_url, ResolveOptions { cookies, referer, page_url })`。
4. 成功则用 **根 Navigator** 打开现有 `ResolveWizard`（路径 `/browse/wizard`，`parentNavigatorKey` 与播放器相同），避免盖在 WebView 上无法返回或销毁 WebView。向导入参经 Riverpod 传递 `ResolveOutcome` 与 `DownloadAuth`，**禁止**把 Cookie 放进路由 query。
5. 向导入队时把 **同一组** `cookies` / `referer` 传给 `enqueue*`（见 §4），再 `ensureDownloads()`，成功后仍去 `/tasks`。

若仍返回 `NeedsBrowser`：留在向导说明页，提示可返回浏览继续登录或改用嗅探候选；不自动死循环打开浏览。

`resolveQualities` 对 HLS 候选必须传入 **同一** `ResolveOptions`。

### 2.5 嗅探候选

1. 原生钩子把请求打成 `SniffEvent`（§5），Dart 按当前页会话累积。
2. **顶层导航**（主框架 URL 变化）清空该会话缓冲区并重新累积。
3. 缓冲区上限 **500** 条；超出丢弃最旧事件。
4. 自上次事件起 **300ms** 无新事件则调用 `Engine.sniffUrls(events, pageUrl: 当前主框架 URL)`，用返回列表刷新 UI（引擎内已去重）。
5. 用户点选候选 → 若 HLS 且无 `quality`，先 `resolveQualities(mediaUrl, opts)`（opts 含当前 Cookie/Referer/page_url）→ 再走与添加流相同的确认标题 / 入队。

不在 WebView 内自动开始下载。

---

## 3. 架构

### 3.1 混合分层（已选方案 C）

```
app/lib/features/browse/     # 浏览 UI、会话状态、解析/嗅探协调
app/lib/providers/           # Cookie 导出、嗅探缓冲；经 EngineRepository 调引擎
platforms/webview_sniff/     # 最小 Flutter 插件：网络钩子 EventChannel
engine/                      # 入队鉴权快照 + worker 带头发下载；sniff/resolve 语义不变
```

- **Flutter**：地址栏、WebView 控件、Cookie 导出、调用 `resolve*` / `sniffUrls` / `enqueue*`、向导复用。
- **`platforms/webview_sniff`**：在四端 WebView 实现上注册请求观察，发出 JSON `SniffEvent`（字段与 `engine` / Dart 模型一致：`url`、`page_url`、`initiator`）。
- **禁止**：在原生层解析媒体、入队或写 SQLite。

WebView 控件本身用官方 `webview_flutter`（及各端实现）。钩子插件 **必须挂在同一 WebView 实例上**，不能另开看不见的第二个 WebView 只钩空页面。

### 3.2 模块（相对 Plan 5 增量）

```
app/lib/features/browse/
  browse_screen.dart
  browse_chrome.dart          # 地址栏与导航按钮
  sniff_candidate_list.dart
platforms/webview_sniff/      # 插件包，app 依赖 path
  lib/webview_sniff.dart      # attach(controller) + Stream<SniffEvent>
  android/ ios/ macos/ windows/
```

`EngineRepository` 增加：

```dart
List<ResourceCandidate> sniffUrls(List<SniffEvent> events, {String? pageUrl});
```

`enqueueSingle` / `enqueueEpisodes` 增加可选 `DownloadAuth { String? cookies; String? referer; }`。无鉴权的添加流传 `null`，行为与 Plan 5 相同。

### 3.3 浏览会话状态

`BrowseSession`（Riverpod，随 `/browse` 存活）：

| 字段 | 含义 |
|------|------|
| `currentUrl` | 主框架 URL |
| `canGoBack` / `canGoForward` | 导航按钮 |
| `sniffEvents` | 本页累积事件 |
| `candidates` | 最近一次 `sniffUrls` 结果 |
| `auth` | 最近一次成功导出的 Cookie/Referer（解析或入队前刷新） |

不把 Cookie 写入日志、SnackBar 或任务 UI。

---

## 4. 引擎：任务鉴权快照

仅解析带 Cookie、下载不带 Cookie，登录视频会在 worker 阶段 401/403。本规格将此视为 **阻断缺口**，必须修。

### 4.1 持久化

`download_tasks` 增加可空列：

- `cookie_header TEXT`
- `referer TEXT`

父任务与子任务在 **同一事务** 写入（现有 `enqueue_episodes` 约束不变）；子任务复制与父任务相同的鉴权快照。

`source_url` 仍可能含查询鉴权参数；Cookie 与其同等对待：**仅本机任务库**。

### 4.2 公开 JSON 脱敏

`DownloadTask` 经 FFI 出现在 `list_tasks` 与 `task_updated.task` 时：

- **不序列化** `cookie_header` / `referer`（无键或恒为省略）。
- `source_url` 保持现状（Plan 5 任务磁贴以标题为主；不在本规格扩大 URL 展示）。

引擎内部 `TaskStore` / worker **可读**这两列。测试必须断言：FFI JSON 无 Cookie 原文；worker 请求带 `Cookie` / `Referer` 头。

### 4.3 FFI

鉴权 JSON 形状（字段均可省略）：

```json
{ "cookies": "sid=ok", "referer": "https://example/page" }
```

- `engine_enqueue_single`：增加最后一个可选 `opts_json`（`null` = 无鉴权）。应用与 `engine/ffi` 同仓构建，不保持旧 C ABI。
- `engine_enqueue_episodes`：不新增 C 参数；把同样的 `cookies` / `referer` 键并入 **现有** 入队 JSON。缺省则与今日行为一致。

`HttpClient` 下载 MP4 / HLS（playlist 与分片）必须带上该任务的 Cookie 与 Referer；不能只在「抓 HTML」路径支持。

无 Cookie 的旧任务与新任务 `opts_json=null`：请求头与今日行为一致。

### 4.4 不把浏览 Cookie 仓放进 Engine

WebView 登录态以 **平台 CookieManager** 为准（随 WebView 持久化）。引擎只保存 **入队瞬间快照**。用户在浏览中退出登录不影响已入队任务；过期则下载失败并走现有任务 `Failed` + 错误展示。

设置「清除浏览 Cookie」只清 WebView Cookie，**不清**已入队任务快照。

---

## 5. 原生钩子与 initiator 映射

插件事件 JSON 与 Dart `SniffEvent.toJson()` 一致。`page_url` 为发出该请求时的主框架 URL；未知则省略，由 `sniffUrls` 的第二个参数补齐。

| 端 | 机制 | 覆盖预期 |
|----|------|----------|
| Android | `WebViewClient.shouldInterceptRequest`（观察后 **放行原请求**，不改写 body） | 高：常见 http(s) 子资源 |
| Windows | WebView2 `WebResourceRequested`（同样只观察） | 高 |
| iOS / macOS | `WKNavigationDelegate` + 注入脚本观察 `fetch` / `XHR` / `HTMLMediaElement` | 中：导航与脚本可见请求 |

**明确不保证：** MSE / `blob:` / 纯内存播放器、Service Worker 隐匿请求、非 WebView 发出的请求。这些路径用户仍可用「解析本页」（Cookie 注入 HTML/API 解析）。

`initiator` 映射（无法区分时用 `other`，禁止编造 `media`）：

| 条件 | `SniffInitiator` |
|------|------------------|
| 主框架导航 | `navigation` |
| MIME 或 URL 可判断为音视频 / `.m3u8` / `.mp4` | `media` |
| 其它子资源 | `sub_resource` |
| 其余 | `other` |

`sniff_urls` 仍只按 **URL 分类** 产出候选；initiator 供后续分析，本规格不改变分类器。

钩子 **不得** 把 Cookie 头放进事件 payload。

---

## 6. 错误处理

| 场景 | 行为 |
|------|------|
| 非法地址 | 地址栏校验错误，不导航 |
| WebView 主框架加载失败 | 页内错误态 + 可刷新；不崩溃 |
| 钩子流断开 / 某端未实现 | 嗅探列表空；「解析本页」仍可用；设置或首次进入可提示「本端嗅探能力有限」一次 |
| `resolveUrl` 网络/引擎错误 | 沿用 `presentEngineError`；留在浏览或向导，不丢 WebView 会话 |
| 入队/下载 401 类 | 任务失败；现有任务错误文案；不自动清 Cookie |
| TV 打开 `/browse` | 说明页，无 WebView |

调试日志只打 URL host 与 initiator，不打 Cookie 或完整 query（query 可能含 token；实现用已有 URL 脱敏或只打 path）。

---

## 7. 设置

在现有 `EngineSettings` 表单之下增加 **浏览数据**（非引擎字段）：

- 按钮「清除浏览 Cookie」：调用平台 `CookieManager` 删除全部 Cookie，成功 SnackBar「已清除浏览 Cookie」。
- 不增加「Cookie 文本框」或手动粘贴 Cookie（避免把应用做成通用盗号工具 UI）。

---

## 8. 测试

Widget 测试仍通过 `EngineRepository` fake，不加载原生库。

| ID | 场景 |
|----|------|
| W3' | Shell 五项：窄屏 `NavigationBar`、宽屏 `NavigationRail`；可见「浏览」 |
| W5 | `NeedsBrowser` 显示「打开内置浏览」（TV 形态测试可 skip 或断言无此按钮） |
| W6 | 非法地址（`javascript:`）不调用加载 |
| W7 | fake `sniffUrls` 返回候选时列表展示并可点选进入入队路径 |
| W8 | fake `resolveUrl` 在「解析本页」时收到非空 `opts.cookies`（由 fake Cookie 导出注入） |
| W9 | `enqueueSingle` 在浏览入队路径收到 `DownloadAuth`；添加页无鉴权路径仍不传 |

引擎 / FFI：

| ID | 场景 |
|----|------|
| T1 | 带 Cookie 入队后 worker 请求含 `Cookie` 头；重启 `Engine::open` 后未完成任务仍带快照 |
| T2 | `list_tasks` JSON 不含 cookie 原文 |
| T3 | `opts_json` 为空时下载请求无强制 Cookie 头（与旧行为一致） |
| T4 | `enqueue_episodes` 父子同一事务且子任务鉴权一致；失败回滚无部分行 |

集成（macOS，可与现有 fixture HTTP 服务）：

| ID | 场景 |
|----|------|
| U6 | 浏览打开需 Cookie 的 HTML fixture → 解析本页得到非 `NeedsBrowser` 或明确候选 |
| U7 | 页面引用 fixture MP4 → 嗅探列表出现该 URL（若 WKWebView 钩不到则记录为端能力缺口，U6 仍必须绿） |

Plan 5 的 W1–W4、U1–U3 与 F1–F5 **保持绿色**。交付前运行 AGENTS.md 中的 cargo fmt / test / clippy 与 `cd app && flutter test`。

---

## 9. 完成标准

- [ ] Android / iOS / Windows / macOS：浏览页可打开 http(s)、解析本页、候选入队（嗅探覆盖按 §5 矩阵，iOS/macOS 允许仅解析闭环）
- [ ] `NeedsBrowser` 能进入同一浏览会话
- [ ] 任务鉴权快照：T1–T4 绿；UI/事件无 Cookie
- [ ] 设置可清除浏览 Cookie
- [ ] W3'、W5–W9 与既有 flutter 测试绿；U6 绿
- [ ] README 补充浏览主路径；`platforms/README.md` 指向本插件
- [ ] **不含** Share、LAN、TV 浏览、站点目录、系统抓包、DRM 绕过
- [ ] 不绕过 `Engine` 写任务/片库

---

## 10. 路线图

| # | 计划 | 状态 |
|---|------|------|
| 1–5 | Engine / 下载 / 解析 / FFI / App UI | ✅ |
| **6a** | **受控 WebView + 嗅探 + 任务鉴权快照** | **本规格** |
| 6b | 系统分享 → `/add?url=` | 未开始 |
| 7 | LAN Cast + TV | 未开始 |
