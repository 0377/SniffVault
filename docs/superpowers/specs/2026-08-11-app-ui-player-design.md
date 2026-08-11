# App UI + Player 子系统设计（Plan 5）

**日期**: 2026-08-11  
**状态**: 待用户审阅  
**前置计划**: Plan 1–4（已完成）  
**后续计划**: Plan 6 Platform 胶水（WebView/Share）、Plan 7 LAN Cast + TV  
**父规格**: `docs/superpowers/specs/2026-08-11-flutter-ffi-design.md`

---

## 1. 目标

在 Plan 4 交付的 `EngineHost` + FFI 基础上，实现 **可用的跨端应用 UI 与本地播放器**，使用户能在 Android、iOS、Windows、macOS 上完成「粘贴 URL → 解析 → 下载 → 片库浏览 → 续播」闭环。

**Plan 5 验收一句话：** 用户在手机或桌面上粘贴 URL，能解析并下载（含选清晰度、批量选集），在片库看到条目并续播，任务页能管控下载；`NeedsBrowser` 有明确提示；不含内置浏览器、分享扩展与 LAN/TV。

### 1.1 范围决策（B+）

| 纳入 Plan 5 | 留给 Plan 6/7 |
|-------------|---------------|
| 四端可用 UI（Android / iOS / Windows / macOS） | Android TV Leanback（Plan 7） |
| 粘贴 URL → 解析 → 入队完整流程 | 内置 WebView 浏览器 |
| 清晰度选择、分集多选批量下载 | Share Extension、Cookie 同步 UI |
| 片库浏览 + 剧集详情 + 续播标记 | LAN 投送 |
| 任务列表（进度、暂停/恢复/取消、父子折叠） | 嗅探面板（`sniffUrls` 需 WebView） |
| 本地播放器 + 进度回写 `setEpisodePosition` | 海报抓取、字幕、PiP |
| 设置页（全部 `EngineSettings` 字段） | |
| Widget 测试 W1–W4 + 集成测试 U1–U3 | |

**不选纯 MVP（A）**：缺清晰度与分集批量则剧集价值不足；缺进度回写则片库体验不完整。  
**不选五端全量打磨（C）**：Plan 6 接入 WebView/分享后交互仍会变化；TV 归 Plan 7。  
**不选仅 UI 壳（D）**：路线图已命名「UI + Player」，播放器不可拆期。

### 1.2 不在范围

| 项 | 归属 |
|----|------|
| 内置 WebView、Cookie 采集 UI | Plan 6 |
| Share Extension、`sniffUrls` 可视化工作流 | Plan 6 |
| LAN 投送、Android TV Leanback | Plan 7 |
| Engine / FFI 公开 API 变更 | 除非播放器集成发现阻断性缺口 |
| 字幕、PiP、海报自动抓取 | 后续迭代 |

### 1.3 与 Plan 4 / Plan 6 边界

- **Plan 4**：`EngineHost`、JSON FFI、`taskEvents`、F1–F5 smoke；无业务 UI
- **Plan 5**：基于 `EngineHost` 构建 Flutter 应用层；**不修改** FFI 边界（优先）
- **Plan 6**：WebView 钩子向 `resolveUrl` 传入 `ResolveOptions.cookies`；Share 深链进入 `/add?url=...`（go_router 预留）

---

## 2. 技术栈

| 层 | 选型 | 理由 |
|----|------|------|
| 状态管理 | **flutter_riverpod** | 全局 `EngineHost` 生命周期、`taskEvents` 驱动刷新 |
| 路由 | **go_router** | 声明式路由，为 Plan 6 深链预留 |
| 播放器 | **media_kit** + media_kit_video + media_kit_libs_video | libmpv，本地 MP4 与 ffmpeg 合并 HLS 兼容性好 |
| 引擎桥接 | 现有 `EngineHost`（Plan 4） | 不引入 flutter_rust_bridge |

`main()` 中调用 `MediaKit.ensureInitialized()`。新增依赖写入 `app/pubspec.yaml`。

---

## 3. 信息架构与路由

### 3.1 自适应导航 Shell

**窄屏（width < 600）**：`BottomNavigationBar` — 片库 | 任务 | 添加 | 设置  
**宽屏（width ≥ 600）**：`NavigationRail` — 同上四项

播放器路由 `/play/:episodeId` 为全屏覆盖层，不显示底栏 / Rail。

### 3.2 路由表（go_router）

| 路径 | 页面 | 说明 |
|------|------|------|
| `/library` | `LibraryScreen` | 默认首页；片库列表 |
| `/library/:itemId` | `LibraryDetailScreen` | Single 直播入口 / Series 分集列表 |
| `/tasks` | `TasksScreen` | 下载任务（父子折叠） |
| `/add` | `AddScreen` | 粘贴 URL + 解析向导 |
| `/settings` | `SettingsScreen` | `EngineSettings` 表单 |
| `/play/:episodeId` | `PlayerScreen` | 全屏播放器 |

Plan 6 预留：`/add?url=<encoded>` 深链直达添加页并预填 URL。

### 3.3 添加内容用户流

```
粘贴 URL → resolveUrl()
    ├─ Single        → 确认标题 → enqueueSingle → ensureDownloads()
    ├─ Candidates    → 选候选；按需 resolveQualities → 选清晰度 → enqueueSingle
    ├─ EpisodeList   → 多选分集（默认全选）→ enqueueEpisodes → ensureDownloads()
    └─ NeedsBrowser  → 说明页：「此站点需登录浏览，内置浏览器将在后续版本支持」
```

入队成功后导航至 `/tasks` 并 SnackBar 提示。`qualityLabel` 默认取 `settings.defaultQualityLabel`（含 `"highest"`）。

### 3.4 片库与播放

- 片库按 `created_at_ms` 降序；Series 显示 `season`；`position_ms > 0` 显示续播角标
- Single：详情页一键播放
- Series：分集列表，每行可选显示进度（`position_ms / duration_ms`，`duration_ms` 为空时不绘条）
- 播放：`LibraryEpisode.file_path` 已为引擎 canonicalize 的绝对路径，直接传给 media_kit
- 进入播放器 `seek(position_ms)`；`Pop` 或页面 dispose 时 `setEpisodePosition(episodeId, positionMs)`

---

## 4. 模块结构

```
app/lib/
  main.dart                    # WidgetsFlutterBinding + MediaKit + ProviderScope
  app.dart                     # MaterialApp.router
  router.dart                  # go_router 配置
  shell/app_shell.dart         # BottomNav / NavigationRail
  features/
    library/                   # library_screen, library_detail_screen, widgets
    tasks/                     # tasks_screen, TaskTile, ParentTaskGroup
    add/                       # add_screen, resolve_wizard, QualityPicker, EpisodeMultiSelect
    settings/                  # settings_screen
    player/                    # player_screen, player_controller
  providers/
    engine_host_provider.dart  # keepAlive；open/dispose
    engine_repository.dart     # 抽象接口（Widget 测试 mock 用）
    library_provider.dart
    tasks_provider.dart
    settings_provider.dart
    download_coordinator.dart  # worker 启停 + taskEvents 分发
  ui/
    error_presenter.dart
    loading_overlay.dart
  engine/                      # Plan 4 已有，本计划不扩 FFI
```

**原则：** Feature 层只依赖 `providers/` 与 `engine/`；禁止 Widget 内直接 `EngineHost.open()`。

---

## 5. 状态与数据流

### 5.1 Provider 职责

| Provider | 职责 |
|----------|------|
| `engineHostProvider` | `EngineHost.open(dataDir)`；app 生命周期内单例；退出时 `dispose` |
| `engineRepositoryProvider` | 对 `EngineHost` 的薄封装，供测试替换 |
| `libraryProvider` | `listLibrary()` |
| `tasksProvider` | `listTasks()` |
| `settingsProvider` | `settings()` / `saveSettings()` |
| `downloadCoordinator` | 订阅 `taskEvents`；维护 `workerActive`；协调 `invalidate` |

### 5.2 下载协调器

| 事件 / 操作 | 动作 |
|-------------|------|
| 用户入队后 | `ensureDownloads()`：若 `!workerActive` 则 `startDownloads()` |
| `TaskUpdated` + `Completed` | `invalidate(tasksProvider)` + `invalidate(libraryProvider)` |
| `TaskUpdated`（其他状态） | 仅 `invalidate(tasksProvider)` |
| `WorkerStopped` | `workerActive = false`；若仍有 `Queued` 任务则再次 `startDownloads()` |
| `startDownloads` 返回 `downloads already running` | 忽略（幂等，不弹错） |

### 5.3 任务列表 UI

- 父任务（`parent_id == null`）可展开显示子任务
- `Running`：进度条 `progress_bytes / total_bytes`（`total_bytes == null` 时不除零，显示 indeterminate）
- 操作：暂停 / 恢复 / 取消（调用 `pauseTask` / `resumeTask` / `cancelTask`）

---

## 6. 错误处理

`EngineException` / `FfiError` 经 `error_presenter` 映射：

| `kind` | 用户文案 |
|--------|----------|
| `http` | 「网络请求失败，请检查连接后重试」（详情可展开） |
| `not_found` | 「找不到对应内容」 |
| `invalid_arg` | 直接展示 `message`（如 `media_dir` 校验） |
| `db` / `io` | 「本地存储异常」（详情可展开） |
| `message` | 直接展示；`downloads already running` 不弹窗 |
| `NeedsBrowser`（ResolveOutcome） | 专用说明页，非 Exception |

解析 / 入队失败保留在当前向导页；其他操作失败用 SnackBar。`EngineHost` 已 dispose 时 Provider 层拦截并引导重启应用。

---

## 7. 播放器

- 依赖：`media_kit`、`media_kit_video`、`media_kit_libs_video`
- 打开 `LibraryEpisode.file_path`（绝对路径）
- 控件：播放/暂停、进度条、返回；无字幕、无 PiP（后续迭代）
- 进度回写：离开播放器时调用 `setEpisodePosition`；建议 debounce _seek 事件（如每 5s 或暂停时）以减少 FFI 调用频率

---

## 8. 设置页

绑定 `EngineSettings` 全部字段：

| 字段 | UI |
|------|-----|
| `media_dir` | 文本；引擎校验单层目录名 |
| `max_concurrency` | 数字步进（≥ 1） |
| `default_quality_label` | 文本；空或 `"highest"` |
| `user_agent` | 可选文本 |
| `device_name` | 文本 |

保存调用 `saveSettings`；失败展示 `error_presenter` 文案。

---

## 9. 测试

### 9.1 Widget 测试（`flutter test`，无 Rust）

| ID | 场景 |
|----|------|
| W1 | `ResolveWizard` 对四种 `ResolveOutcome` 渲染正确分支 |
| W2 | `TaskTile`：`total_bytes == null` 时不除零 |
| W3 | `AppShell`：宽度阈值切换 BottomNav / Rail |
| W4 | `SettingsScreen`：`media_dir` 含 `/` 时展示错误 |

通过 `EngineRepository` 抽象 + mock 实现，避免 Widget 测试加载原生库。

### 9.2 集成测试（`integration_test/`，需 macOS 设备）

| ID | 场景 |
|----|------|
| U1 | 添加页粘贴 fixture MP4 URL → 任务列表出现条目 |
| U2 | 等待 `Completed` → 片库出现对应条目 |
| U3 | 播放 → seek → 退出 → 再进续播位置正确 |
| U4 | 设置页修改 `deviceName` → 重启 app 后保留（交付前本地必跑） |
| U5 | 暂停 / 恢复任务状态正确（交付前本地必跑） |

Plan 4 的 F1–F5 **保留不删**。CI `flutter-smoke` 至少覆盖 F1–F5 + U1–U3。

### 9.3 引擎验证

Plan 5 默认 **不修改** `engine/`。交付前仍运行：

```bash
cargo fmt --manifest-path engine/Cargo.toml --all -- --check
cargo test --manifest-path engine/Cargo.toml --workspace
cargo clippy --manifest-path engine/Cargo.toml --all-targets --all-features -- -D warnings
cd app && flutter test && flutter test integration_test/smoke_test.dart -d macos
```

---

## 10. Plan 5 完成标准

- [ ] Android / iOS / Windows / macOS `flutter build` 通过
- [ ] §3 全部路由与用户流可用
- [ ] 本地 MP4 与 ffmpeg 合并 HLS 文件可播放，进度回写持久化
- [ ] W1–W4 + U1–U3 绿；U4–U5 交付前本地绿
- [ ] README 补充 Plan 5 主流程说明
- [ ] 不含 WebView、Share、LAN、TV UI、字幕/PiP
- [ ] 不暴露或依赖 `register_completed_*`、`drain_downloads_for_test`

---

## 11. 路线图

| # | 计划 | 状态 |
|---|------|------|
| 1–4 | Engine / Downloader / Resolver / FFI | ✅ |
| **5** | **App UI + Player** | **本规格** |
| 6 | Platform 胶水 | 待开始 |
| 7 | LAN Cast + TV | 待开始 |
