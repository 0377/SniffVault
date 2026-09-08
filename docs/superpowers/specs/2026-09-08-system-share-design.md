# 系统分享与深链入口设计（Plan 6b）

**日期**: 2026-09-08  
**状态**: 已审阅  
**前置计划**: Plan 1–5、Plan 6a（已完成）  
**后续计划**: Plan 7 LAN Cast + TV Leanback；桌面深链补丁（可选）  
**父规格**: `docs/superpowers/specs/2026-08-11-app-ui-player-design.md`、`docs/superpowers/specs/2026-09-07-webview-sniff-design.md`

---

## 1. 目标

在 Plan 5「粘贴 URL」与 Plan 6a「内置浏览取流」之上，补上 **从系统分享菜单把链接送进应用** 的路径：用户在 Android / iOS 从浏览器或其他 App 分享文本/链接到本应用后，主 App 打开 **添加页并预填 URL**，用户仍手动点「解析」走现有 `resolveUrl` → `ResolveWizard` 流程。

**Plan 6b 验收一句话：** 用户在 Android / iOS 通过系统「分享」选中本应用后，主 App 冷启动或前台唤起，自动切到添加 Tab 并预填可解析的 `http(s)` URL；无效分享内容有明确提示；不含自动解析、不含 Universal Links、不含 LAN/TV、不改 Engine/FFI。

### 1.1 范围决策

| 纳入 Plan 6b | 留给后续 |
|--------------|----------|
| Android `ACTION_SEND`（`text/plain`）接收分享 | Universal Links / Android App Links（需域名与运维） |
| iOS Share Extension（最小 UI）→ 唤起主 App | macOS Services、Windows 自定义协议注册 |
| 双端自定义 URL Scheme：`sniffvault://add?url=<encoded>` | Android TV 分享入口优化、Leanback |
| Flutter 统一深链入口 → `go_router` `/add?url=` | 分享后自动解析/入队 |
| 从分享文本中提取首个 `http(s)` URL | 分享文件、非 URL 载荷、多 URL 批量入队 |
| Widget 测试 W10–W13；集成测试 U8（深链预填，CI 门禁） | Engine / FFI 变更 |

**不选方案 B（Universal Links 首发）**：开源自编译产品无固定公网域名，AASA / assetlinks 运维成本高，自定义 Scheme 足以覆盖 Share Extension 回主 App。后续若有官网域名可单开补丁。  
**不选方案 C（五端一次做完）**：桌面用户可直接粘贴或内置浏览；TV 不应接收「分享 URL 取流」（无 WebView、与 Plan 7「投送已缓存」模型冲突）。  
**不选分享直达 `/browse`**：分享只解决「把 URL 填进添加页」；`NeedsBrowser` 仍由用户在添加流或向导内进入内置浏览。

### 1.2 产品原则

- **分享是添加入口，不是自动化下载**：预填 URL 后由用户点「解析」，与粘贴行为一致，避免误触分享即开始网络请求。
- **只接受 `http` / `https`**：与 `browseUrlError` / 添加页校验一致；拒绝 `javascript:`、`file:`、`data:` 等。
- **不扩大攻击面**：Share Extension 只传递 raw 文本并跳转主 App；不在 Extension 内跑 Flutter、不访问引擎、不写 SQLite。
- **深链与路由单一真相**：外部入口一律归一为 `sniffvault://add?url=<encoded>` → Flutter 解析后 `go_router` 导航 `/add?url=`，避免多套逻辑。

### 1.3 与既有计划的边界

| 计划 | 本规格的关系 |
|------|----------------|
| Plan 5 | 复用 `/add?url=`、`AddScreen.initialUrl`、`ResolveWizard`；不改片库/任务/播放器 |
| Plan 6a | 分享不进 `/browse`；`NeedsBrowser` 仍用「打开内置浏览」 |
| Plan 7 | LAN 投送与分享无关；TV 不新增 Share 目标优化 |
| Engine / FFI | **无变更**；分享不携带 Cookie，鉴权仍靠浏览会话 |

---

## 2. 用户流程

### 2.1 Android

1. 用户在 Chrome / 其他 App 打开含链接的页面，点系统 **分享** → 选择本应用。
2. `MainActivity` 收到 `ACTION_SEND`，将 `EXTRA_TEXT`（raw）写入 `sniffvault://add?url=...`（**必须用 `Uri.Builder.appendQueryParameter`** 编码），再 `setIntent(ACTION_VIEW, uri)` 供 `app_links` 读取。
3. 冷启动 / `onNewIntent`（`singleTop`）均走同一路径。
4. 添加页预填 URL；用户点 **解析**。

### 2.2 iOS

1. 用户点 **分享** → 选择 Share Extension（显示名与主 App 一致）。
2. Extension 从 `NSExtensionItem` 读取 `public.url` 或 `public.plain-text` 的 **raw 字符串**（不在 Extension 内做 URL 提取）。
3. Extension 用 **`URLComponents`** 构造 `sniffvault://add?url=<encoded raw>`，再 `extensionContext.open(url)`。
4. 主 App 经 `app_links` 收到 URI，Flutter `DeepLinkHost` 导航至添加页并预填。

Extension UI：极简；失败时显示「未识别到有效链接」后 `completeRequest`。

### 2.3 无效或无法提取 URL

| 场景 | 行为 |
|------|------|
| 分享纯文本无 URL | 打开 `/add`；地址栏不预填有效 URL；SnackBar：「未能识别有效链接」 |
| 载荷含 URL 形态但 scheme 非 `http`/`https`（如 `javascript:`） | 打开 `/add`；不预填；SnackBar：「仅支持 http/https 链接」 |
| 文本含多个 URL | 使用 **第一个** 合法 `http(s)` URL（从左到右） |
| `url` query 缺失 | 打开 `/add`；SnackBar：「未能识别有效链接」 |
| 载荷 > 8192 字符 | 打开 `/add`；SnackBar：「链接过长」 |
| 用户取消 Share Extension | 不唤起主 App |

SnackBar 文案与 `IngressFailure` 一一对应（见 §5）。

### 2.4 与 `NeedsBrowser` 的关系

分享进入添加页后，若 `resolveUrl` 返回 `NeedsBrowser`，行为与 Plan 5/6a 相同。分享 **不能** 绕过登录页直接取流。

---

## 3. 架构

### 3.1 分层

```
外部系统（Share Sheet / Intent）
        ↓
MainActivity / iOS Share Extension   # 只生成 sniffvault:// URI
        ↓
app_links（冷启动 getInitialLink + 热启动 uriLinkStream）
        ↓
app/lib/deep_link/                   # 解析、去重、排队、导航
        ↓
go_router /add?url=
        ↓
AddScreen + ResolveWizard
```

- **禁止**在 `engine/`、`webview_sniff` 内实现分享路由。
- **禁止**Share Extension 链接 `rust_lib` 或访问 `Engine`。

### 3.2 `platforms/share_ingress/`

| 能力 | Android | iOS | 说明 |
|------|---------|-----|------|
| Share Extension 源文件 | — | ✅ | 托管于插件目录，Xcode target 引用 |
| 空壳 Flutter 插件注册 | ✅ | ✅ | **无 Dart API**；`app/pubspec` path 依赖仅为 iOS 工程集成 |
| `ACTION_SEND` / Scheme 注册 | — | — | **在 `MainActivity` / `Runner/Info.plist`**，不在插件 |

Dart 侧 URL 提取在 `app/lib/deep_link/share_url_extractor.dart`（Widget 可测）。

### 3.3 Flutter 深链编排 `DeepLinkHost`

挂载于 `MaterialApp.router` 的 `builder`（仅引擎 `data` 分支）。职责：

1. **单一入口**：`main()` 中 `getInitialLink()` → `setBootstrapIngressUri`；热启动由根级 **`IngressUriListener`** 订阅 `uriLinkStream` 写入 `pendingIngressUriProvider`。`DeepLinkHost` **无** `initialUri` prop。
2. **去重**：`_lastHandledUri`（`uri.toString()` 比较）。
3. **引擎未就绪排队**：`IngressUriListener` 任意时刻写入 pending；`DeepLinkHost` 仅在引擎 `data` 后 `addPostFrameCallback` 消费并清空。
4. **导航**：`router.go('/add?url=...')` only，**不** `goBranch`。
5. **热更新**：`AddScreen.didUpdateWidget` 同步 `initialUrl` 到 `TextEditingController`。

### 3.4 URL 提取与失败分类

`extractHttpUrl(raw)`：

1. 若 `browseUrlError(raw) == null` → 返回 `raw.trim()`。
2. 否则正则取第一个 `https?://...` 候选，再 `browseUrlError`。
3. 无候选 → `null`。

`classifyIngressPayload(raw)`（供 `parseSniffVaultIngress`）：

| 条件 | `IngressFailure` |
|------|------------------|
| 空 / 无 URL 形态 | `noHttpUrl` |
| 有 URL 形态但 scheme 非 http(s) | `invalidScheme` |
| 长度 > 8192 | `payloadTooLong` |
| 提取成功 | `IngressNavigateSuccess` |

### 3.5 自定义 URL Scheme

| 项 | 值 |
|----|-----|
| Scheme | `sniffvault` |
| Host | `add` |
| Query | `url`（raw 分享文本或完整 URL，由平台 API 编码） |
| 示例 | `sniffvault://add?url=https%3A%2F%2Fexample.com%2Fwatch%3Fv%3D1` |

Android：`Uri.Builder().scheme("sniffvault").authority("add").appendQueryParameter("url", text)`。  
iOS：`URLComponents` + `URLQueryItem(name: "url", value: raw)`。

---

## 4. 平台实现要点

### 4.1 Android

Manifest：`ACTION_SEND`（`text/plain`）+ `ACTION_VIEW`（`sniffvault` / `add`）。

`MainActivity.onCreate` / `onNewIntent`：`toIngressUri(intent)` → `setIntent(Intent(ACTION_VIEW, uri))`。  
**禁止** `Uri.encode(text)` 手拼 query 字符串。

### 4.2 iOS

- `Runner/Info.plist`：`CFBundleURLTypes` → `sniffvault`。
- Share Extension：`platforms/share_ingress/ios/ShareExtension/`；Bundle Id `com.videosniffing.videoSniffing.ShareExtension`。
- Extension **只传 raw**，用 `URLComponents` 编码；不在 Extension 内调用 `extractHttpUrl`。
- 无 App Group。

### 4.3 依赖

```yaml
app_links: ^6.x
share_ingress:
  path: ../platforms/share_ingress   # 无 Dart import 亦可，仅为 iOS target
```

`ingressUriStreamProvider` 类型为 **`Provider<Stream<Uri>>`**（非 `StreamProvider`），便于测试 override。

---

## 5. 错误处理与安全

| 场景 | SnackBar / 行为 |
|------|-----------------|
| 非 `sniffvault` / 非 `add` host | 忽略 |
| `missingPayload` | 「未能识别有效链接」 |
| `noHttpUrl` | 「未能识别有效链接」 |
| `invalidScheme` | 「仅支持 http/https 链接」 |
| `payloadTooLong` | 「链接过长」 |
| 重复相同 URI | 忽略（去重） |
| 引擎 loading | 写入 `pendingIngressUriProvider`，就绪后处理 |
| 日志 | 仅 host + path |

---

## 6. 模块结构（增量）

```
platforms/share_ingress/
  lib/share_ingress.dart          # 空 library；无公开 API
  ios/ShareExtension/             # ShareViewController.swift, Info.plist
app/lib/deep_link/
  share_url_extractor.dart
  ingress_uri.dart
  deep_link_providers.dart
  deep_link_host.dart          # DeepLinkHost + IngressUriListener
app/lib/features/add/add_screen.dart
app/lib/main.dart              # setBootstrapIngressUri + IngressUriListener 包裹
app/lib/app.dart               # builder: DeepLinkHost
```

---

## 7. 测试

| ID | 场景 |
|----|------|
| W10 | `DeepLinkHost` + `sniffvault://add?url=...` → `/add` 预填正确 |
| W11 | `javascript:` payload → SnackBar「仅支持 http/https 链接」 |
| W12 | `extractHttpUrl` 多 URL 取第一个 |
| W13 | `AddScreen` 第二次深链更新地址栏（须 `ProviderScope`） |

集成：

| ID | 场景 |
|----|------|
| U8 | **CI 门禁（macOS）**：`integration_test/deep_link_test.dart` |
| U9 | 真机 Share Sheet（手工，CI 不跑） |

---

## 8. 文档与 CI

- `README.md`：Plan 6b 小节 + U8 命令
- `platforms/README.md`：`share_ingress` 职责
- `.github/workflows/ci.yml`：`flutter-integration` matrix 增加 `deeplink` suite，跑 `deep_link_test.dart`

---

## 9. 完成标准

- [x] Android/iOS 分享与 `sniffvault://` 打开添加页并预填
- [x] 深链去重；引擎未就绪不丢链（`pendingIngressUriProvider`）
- [x] SnackBar 文案与 §5 一致
- [x] W10–W13、U8 绿；CI `flutter-integration (deeplink)` 绿
- [x] 不含 Universal Links、桌面协议、自动解析、Engine 变更

---

## 10. 路线图

| # | 计划 | 状态 |
|---|------|------|
| 1–5 | Engine / 下载 / 解析 / FFI / App UI | ✅ |
| 6a | 受控 WebView + 嗅探 + 任务鉴权快照 | ✅ |
| **6b** | **系统分享 → `/add?url=`** | ✅ |
| 6c（可选） | macOS / Windows 自定义协议 | 未开始 |
| 7 | LAN Cast + TV Leanback | 未开始 |
