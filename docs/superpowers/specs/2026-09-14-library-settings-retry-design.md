# 设置目录选择器与失败任务改 URL 重试设计（Plan 9d）

**日期**: 2026-09-14  
**状态**: 已定稿（2026-09-14 review 修订：checkpoint/.dl 清理、TV helper、测试矩阵）  
**前置**: Plan 9a 片库删除（`v0.1.1`）、Plan 9b 重命名与合并、Plan 9c 海报抓取与展示（均已合并 main）  
**父规格**: `docs/superpowers/specs/2026-09-08-library-management-design.md`（§2.6、§5.3）  
**后续**: **`v0.2.0` tag**（9a–9d 全部验收后）  
**实现计划**: `docs/superpowers/plans/2026-09-14-library-settings-retry.md`

---

## 1. 目标

Plan 9a–9c 使片库「能删、能改名、能合并、有封面」，但设置页 `media_dir` 仍为手填文本，失败任务只能原 URL 一键重试，用户无法修正错误链接。本规格交付 **v0.2 最后一刀**：系统目录选择器改善缓存目录配置，以及失败任务在修改 `source_url` 后重新入队。

**Plan 9d 验收一句话：** 设置页可通过系统目录选择器选取文件夹名写入 `media_dir`（保留手填兜底；Android TV 隐藏选择器）；失败任务可立即重试，或通过对话框修改 `source_url` 后重试；Engine 扩展 `retry_task` 为唯一任务 URL 写入口；含 FFI、Flutter UI 与自动化测试；验收后推送 **`v0.2.0` tag**。

### 1.1 范围决策

| 纳入 Plan 9d | 排除（留给 v0.3+） |
|--------------|-------------------|
| 设置页「选择文件夹」+ `file_picker` | 缓存目录整体搬迁与 SQLite 路径批量重写 |
| 保留 `media_dir` 手填 TextField | 将 `media_dir` 改为绝对路径或外置存储直写 |
| `retry_task(task_id, new_url: Option<&str>)` | 编辑 `resolved_media_url`（已有 `set_task_media_url` + 嗅探补全） |
| 失败任务「修改 URL 重试」对话框 | 批量修改多个子任务 URL |
| 父任务「批量重试」保持一键（不改 URL） | 每次重试强制弹对话框 |
| Engine / FFI / Flutter + 测试 | DLNA、片库搜索、Plan 6c 桌面深链 |

**PM 定稿：9d 一次交付**（目录选择器 + 失败改 URL 重试同批 PR），不拆 9d-1/9d-2。理由：二者均为 v0.2「设置好用 + 任务能自救」闭环的最后两块；拆 tag 增加发版成本，收益有限。

**发版节奏：**

1. 9d 合并后推送 **`v0.2.0` tag**（完整 v0.2 体验；README 更新 Plan 9d 节与发版说明）。
2. 不再单独打 `v0.1.4` 预览 tag（9c 已可选 `v0.1.3`；9d 即为 v0.2 收官）。

### 1.2 产品原则

- **Engine 是唯一写入口**：`media_dir` 仍仅经 `Engine::save_settings` + `validate_media_dir`；任务 `source_url` 更新仅经扩展后的 `Engine::retry_task`，禁止 Flutter 或测试直写 `TaskStore` 改 URL（淘汰 `download_integration` 中 `requeue_failed_task` 反模式）。
- **`media_dir` 语义不变**：必须是相对 `data_dir` 的**单层目录名**（AGENTS.md）；目录选择器选中的物理路径 **只取其最后一级目录名** 写入设置，在 `data_dir` 下 `create_dir_all`——**不搬移已有媒体文件**。
- **快路径保留**：失败任务刷新图标 = 立即 `retry_task(id, None)`，与现网行为一致。
- **改 URL 为显式路径**：仅通过「修改 URL 重试」菜单打开对话框，避免网络抖动类失败多一次点击。
- **`needs_sniff` 失败不走改 URL 重试**：`error_message == "needs_sniff"` 的失败任务仍走 Plan 9「嗅探补全」；`retry_task` 继续拒绝此类任务。
- **改 URL 须清断点**：与快路径不同，改 `source_url` 时必须清空 `checkpoint_json` 与 `.dl/{task_id}`，避免旧断点续传到新 URL。
- **LAN / 片库不受影响**：本 Plan 不修改 `CastMetadata`、片库 schema 或投送逻辑。

---

## 2. 用户流程

### 2.1 选择缓存目录（设置页）

1. 设置 → **媒体目录** 字段旁点 **选择文件夹**。
2. 系统目录选择器打开 → 用户选中目录（如 `/Users/me/Downloads/SniffVault` 或 Android SAF 返回的 URI 对应路径）。
3. App 取路径 **最后一级目录名**（上例为 `SniffVault`）填入 `media_dir` 文本框；若与当前值不同，提示「记得点保存」。
4. 用户点 **保存** → `Engine::save_settings` 校验目录名 → 在 `data_dir/SniffVault` 创建目录（若不存在）→ 持久化 `settings.json`。
5. 成功：SnackBar「设置已保存」；失败：字段下方或 SnackBar 展示引擎错误（如目录名含 `/`、为空）。

**说明文案**（`InputDecoration.helperText`，常驻；手机/桌面与 TV 分文案）：

| 平台 | helperText |
|------|------------|
| 手机 / 桌面 | `此处为应用数据目录下的文件夹名称；选择外置路径时仅采用文件夹名，不会自动搬移已有缓存文件。` |
| Android TV | `此处为应用数据目录下的文件夹名称。`（**不** 提及选择器或外置路径） |

**手填兜底**：用户仍可直接编辑 TextField；行为与现网一致（`validate_media_dir` 校验）。

**Android TV**：隐藏 **选择文件夹** 按钮；**不** 调用 `file_picker`；helper 仅用 TV 行文案（见上表）。

### 2.2 失败任务立即重试（快路径）

1. 任务页 → 失败子任务/单任务行 → 点 **刷新** 图标。
2. `retry_task(task_id, None)` → 状态 `Failed → Queued`，清空 `error_message`；**不** 修改 `source_url` / `resolved_media_url` / 进度字段 / `checkpoint_json`；**不** 删除 `.dl/{task_id}` 临时目录（保留断点续传能力）。
3. `DownloadCoordinator.ensureDownloads()` 继续调度。

父任务 **批量重试**（`batch_retry_button`）：对每个可重试子任务调用 `retry_task(id, None)`；**不** 弹 URL 对话框。

### 2.3 失败任务修改 URL 重试

1. 任务页 → 失败任务行 → **更多**（`⋮`）→ **修改 URL 重试**（仅 `taskCanRetry(task)` 为 true 时显示）。
2. 对话框：
   - 标题：`修改 URL 后重试`
   - 多行 `TextField`，预填当前 `source_url`
   - 按钮：取消 / **重试**（primary）
3. 用户编辑 URL → **重试**：
   - trim 后非空；
   - `retry_task(task_id, Some(new_url))`；
   - 成功：关闭对话框 → invalidate `tasksProvider` → `ensureDownloads()`。
4. 失败：SnackBar 展示 `presentEngineError`；对话框保持打开。

**不提供的入口**：`needs_sniff` 失败、已取消/已完成任务无「修改 URL 重试」；无独立「任务详情页」（与现网列表内操作一致）。

---

## 3. Engine API

### 3.1 `retry_task` 扩展

```rust
/// 将失败任务重新入队。`new_url` 为 Some 时更新 source_url 并重置解析/进度状态。
pub fn retry_task(
    &mut self,
    task_id: &str,
    new_url: Option<&str>,
) -> Result<(), EngineError>;
```

**前置条件**（与现网一致）：

- 任务存在；`status == Failed`；
- `error_message != "needs_sniff"`（含 `Some("needs_sniff")`）。

**`new_url == None`（快路径）**：

1. `set_task_status(id, Queued, None)`（清空 `error_message`）。
2. 若存在 `parent_id`，`sync_parent_status`。
3. **不** 修改 `source_url`、`resolved_media_url`、`progress_bytes`、`total_bytes`、`output_path`、`checkpoint_json`；**不** 删除 `.dl/{task_id}`。

**`new_url == Some(url)`**：

1. `url.trim()` 非空，否则 `InvalidArg`。
2. 若 trim 后 URL 与当前 `source_url` **相同**：降级为 `None` 快路径（步骤同上）。
3. 否则在 **单次 `TaskStore` 更新**（或等价原子 upsert）中：
   - `source_url = trim(url)`；
   - `resolved_media_url = None`；
   - `progress_bytes = 0`；
   - `total_bytes = None`；
   - `output_path = None`；
   - `checkpoint_json = NULL`；
   - `error_message = None`；
   - `status = Queued`。
4. 调用 `cleanup_download_temp(media_dir, task_id)` 删除 `{media_dir}/.dl/{task_id}/`（与 `cancel_task` 同源辅助，目录不存在视为成功）。
5. 若存在 `parent_id`，`sync_parent_status`。
6. **不** 修改 `cookie_header` / `referer` / `poster_url`（用户改的是资源页 URL，鉴权上下文保留；若新页需新 Cookie，用户应重新从浏览入队——v0.3 可增强）。

**`TaskStore` 增量**（crate-private）：

```rust
/// 失败任务改 URL 后重试：更新 source_url 并清空下载进度与解析缓存字段。
pub fn requeue_failed_with_url(
    &self,
    id: &str,
    source_url: &str,
) -> Result<(), EngineError>;
```

实现须校验任务当前为 `Failed` 且非 `needs_sniff`（或由 `Engine::retry_task` 先校验再调用）。

### 3.2 设置（无 Engine API 变更）

继续沿用 `Engine::save_settings` + `settings::validate_media_dir`。目录选择器逻辑 **仅存在于 Flutter**；写入前仍走同一保存路径。

---

## 4. FFI

### 4.1 `engine_retry_task` 签名扩展

```c
// new_url 可为 NULL，表示快路径重试
char *engine_retry_task(
    EngineHandle *handle,
    const char *task_id,
    const char *new_url   // nullable
);
```

- `new_url == NULL` 或空字符串：快路径。
- 错误经既有 `ffi_call_mut` / JSON 错误响应返回 Flutter。

### 4.2 其它

- **无** 新增 C API；`engine_save_settings` 不变。
- Dart `NativeBindings.retryTask(taskId, {String? newUrl})`；`newUrl` 为 null 时传 NULL。

---

## 5. Flutter UI

### 5.1 依赖

- 新增 **`file_picker`**（`^8.0.0` 或与当前 Flutter SDK 兼容的最新稳定版）。
- 使用 `FilePicker.platform.getDirectoryPath()`；**不** 引入第二套桌面 `file_selector`，降低维护面。

### 5.2 目录名提取（`app/lib/features/settings/media_dir_picker.dart`）

```dart
/// 从用户选择的系统路径提取可写入 settings.media_dir 的单层目录名。
/// 返回 null 表示用户取消选择。
String? mediaDirNameFromPickerResult(String? pickedPath);
```

规则：

- `pickedPath == null` → `null`（取消）。
- 使用 `path` 包的 `basename`（或 `Platform.pathSeparator` 安全分割）；去掉末尾分隔符。
- 结果须通过引擎侧同等规则（非空、非 `.`、无 `/` `\`、`..`）——UI 可在保存前本地预检，最终以 `saveSettings` 错误为准。
- **不** 尝试将外置路径 symlink 或复制进 `data_dir`。

`SettingsScreen` 变更：

| 控件 | Key | 说明 |
|------|-----|------|
| 选择文件夹按钮 | `settings_pick_media_dir` | `OutlinedButton`，TV 隐藏 |
| 媒体目录字段 | `settings_media_dir` | 保留现有 TextField |
| 说明 helper | — | §2.1 文案 |

选择成功后：更新 controller 与 `_draft.mediaDir`；SnackBar「已填入目录名，记得保存」（与 UA 预设 chip 一致）。

### 5.3 任务列表

**`TaskTile`**：

- 保留刷新图标快路径（`onRetry`）。
- 新增可选 `onEditUrlRetry`；当 `taskCanRetry(task)` 时，trailing 区域在刷新旁增加 `PopupMenuButton`（`Key: task_edit_url_menu_{id}`）→ **修改 URL 重试**。

**`RetryUrlDialog`**（`features/tasks/widgets/retry_url_dialog.dart`）：

- `showRetryUrlDialog(context, initialUrl: task.sourceUrl)` → `Future<String?>`（确认返回 trim 后 URL，取消返回 null）。
- `Key: retry_url_field`、`retry_url_confirm`、`retry_url_cancel`。

**`TasksScreen` / `ParentTaskGroup`**：

- 接线 `onEditUrlRetry` → 对话框 → `repo.retryTask(id, newUrl: url)`。
- 批量重试逻辑 **不变**。

### 5.4 `EngineRepository` / `FakeEngineRepository`

```dart
void retryTask(String taskId, {String? newUrl});
```

`FakeEngineRepository` 记录最后一次 `retryTask` 的 `newUrl`，供 widget 测试断言。

---

## 6. 错误处理

| 场景 | 行为 |
|------|------|
| `media_dir` 校验失败 | `saveSettings` → `EngineException`；设置页展示错误（与现网 W4 一致） |
| 用户取消目录选择器 | 无 SnackBar；草稿不变 |
| 选择结果 basename 为空（如根路径） | SnackBar「无法识别文件夹名称」；不写入字段 |
| `retry_task` 非 Failed | `InvalidArg` |
| `needs_sniff` 失败任务 | `InvalidArg`；UI 不显示改 URL 入口 |
| `new_url` trim 后为空 | 对话框内联错误「URL 不能为空」（**不**关闭对话框）；不调用 Engine |
| `new_url` 与当前 `source_url` 相同 | 等价快路径，成功 |
| 任务不存在 | `NotFound` |

---

## 7. 测试

### 7.1 Engine

| ID | 场景 |
|----|------|
| L9d-1 | `retry_task(id, None)`：Failed → Queued，清 `error_message`，`source_url` 不变 |
| L9d-2 | `retry_task(id, Some(new))`：更新 `source_url`，清空 `resolved_media_url`、`progress_bytes`、`total_bytes`、`output_path`、`checkpoint_json`；`.dl/{id}` 已删除 |
| L9d-3 | `new_url` 与现 `source_url` 相同 → 同 L9d-1 |
| L9d-4 | `retry_task` 拒绝 `needs_sniff` 失败（沿用现有用例，签名扩展后仍通过） |
| L9d-5 | `retry_task` 拒绝非 Failed |
| L9d-6 | 子任务改 URL 重试后 `sync_parent_status` |
| L9d-7 | `download_integration` 中原 `requeue_failed_task` 改为 `engine.retry_task(..., Some(url))` |

### 7.2 FFI

| ID | 场景 |
|----|------|
| F9d-1 | `engine_retry_task(handle, id, NULL)` 成功 |
| F9d-2 | `engine_retry_task(handle, id, "https://new.example/v.mp4")` 成功且 `list_tasks` 反映新 URL |

### 7.3 Flutter Widget

| ID | 场景 |
|----|------|
| W9d-1 | `mediaDirNameFromPickerResult`：`/foo/bar/SniffVault` → `SniffVault`；`null` → `null` |
| W9d-2 | 设置页：点 `settings_pick_media_dir`（注入 mock `FilePicker`）→ 字段更新为 basename |
| W9d-3 | TV：`isTelevisionProvider == true` 时不显示 `settings_pick_media_dir` |
| W9d-4 | `TaskTile`：菜单「修改 URL 重试」触发 `onEditUrlRetry` |
| W9d-4b | `TasksScreen`（或等效接线测）：对话框确认 → `FakeEngineRepository.lastRetryNewUrl` 为编辑后 URL |
| W9d-5 | 失败任务：刷新图标 → `FakeEngineRepository.lastRetryNewUrl == null` |
| W9d-6 | `needs_sniff` 失败：无改 URL 菜单（沿用 / 扩展现有 `task_tile_test`） |

目录选择器 widget 测试 **统一** 使用可注入的 `MediaDirectoryPicker` 接口（**不** 依赖 method channel mock）。

### 7.4 集成

| ID | 场景 |
|----|------|
| U11d | Engine 级（经 `EngineHost` / FFI）：seed Failed 任务 → `retryTask(id, newUrl: …)` → `listTasks` 为 Queued 且 `sourceUrl` 更新；**不要求**完整下载跑通 |

CI：并入现有 `flutter-integration` matrix，新增 **`library_settings_retry`** suite job 跑 `integration_test/library_settings_retry_test.dart`（与 U11b/U11c 模式一致）。

---

## 8. 与 AGENTS.md 对齐检查

- [x] `media_dir` 单层相对名 + `validate_media_dir`；不改为绝对路径
- [x] 任务 URL 经 `Engine::retry_task` 写入，不暴露 TaskStore 公开写口
- [x] 不搬移已有媒体文件（外置路径仅取目录名）
- [x] LAN 元数据不含 `source_url`
- [x] 不提供站外破解能力
- [x] `needs_sniff` 与嗅探补全流程边界清晰

---

## 9. 不在本 Plan 范围

- 缓存目录搬迁向导与 SQLite `file_path` 批量重写（v0.3）
- 修改 `resolved_media_url` 的通用 UI（非嗅探场景）
- 失败任务编辑 `cookie_header` / `referer`
- 父任务批量「修改 URL」
- iOS / Android 发版流水线变更
- Plan 7 安全补丁、Plan 6c 深链

---

## 10. PM 定稿摘要

| 决策项 | 结论 |
|--------|------|
| 下一子计划 | **9d 本 spec**（v0.2 收官） |
| 交付节奏 | **9d 一次交付**，合并后 **`v0.2.0` tag** |
| 目录选择器 | **`file_picker`** + 保留手填；外置路径 **仅取 basename** |
| 目录语义 | 仍在 `data_dir` 下建子目录；**不搬文件** |
| Android TV | **隐藏**选择文件夹 |
| 重试快路径 | 刷新图标 → `retry_task(id, None)` |
| 改 URL | 菜单 + 对话框 → `retry_task(id, Some(url))` |
| 改 URL 时重置 | `resolved_media_url`、进度、`output_path`、`checkpoint_json`、`.dl/{id}`；保留 cookie/referer |
| `needs_sniff` | **拒绝** `retry_task`；无改 URL 菜单 |
| 测试反模式 | 删除 `requeue_failed_task` 直写 TaskStore |
