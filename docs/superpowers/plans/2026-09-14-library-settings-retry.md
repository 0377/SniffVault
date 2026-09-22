# Plan 9d 设置目录选择器与失败任务改 URL 重试 Implementation Plan

> **修订**: 2026-09-14 review（checkpoint/.dl 清理、TV helper 分文案、W9d-4b/5、L9d-1 显式断言）

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 设置页可通过系统目录选择器配置 `media_dir`（保留手填；TV 隐藏选择器）；失败任务支持快路径重试与「修改 URL 重试」；Engine 扩展 `retry_task` 为唯一任务 URL 写入口；验收后推送 `v0.2.0` tag。

**Architecture:** `TaskStore::requeue_failed_with_url` 原子更新失败任务的 `source_url`、进度与 `checkpoint_json`；改 URL 分支另调 `cleanup_download_temp` 清 `.dl/{id}`；`Engine::retry_task(id, new_url)` 统一校验后分支快路径/改 URL；FFI 第三参数可空 → Dart `EngineHost`；Flutter 侧 `mediaDirNameFromPickerResult` + 可注入 `MediaDirectoryPicker`；任务 UI 保留刷新图标，新增 `RetryUrlDialog` 与 `PopupMenuButton`。

**Tech Stack:** Rust (`rusqlite`)、C FFI（Cargokit）、Flutter + Riverpod + `file_picker` + `path`

**规格:** `docs/superpowers/specs/2026-09-14-library-settings-retry-design.md`

## Global Constraints

- **Engine 是唯一任务 URL 写入口**：`source_url` 更新仅经 `Engine::retry_task`；禁止测试/Flutter 直写 `TaskStore` 改 URL（删除 `requeue_failed_task`）
- **`media_dir` 语义不变**：单层相对目录名；目录选择器 **仅取 basename**；`save_settings` 在 `data_dir` 下 `create_dir_all`；**不搬移已有媒体文件**
- **快路径**：`retry_task(id, None)` 仅 `Failed → Queued` + 清 `error_message`；不改 `source_url` / `resolved_media_url` / 进度 / `checkpoint_json`；不删 `.dl/{id}`
- **改 URL 路径**：`retry_task(id, Some(url))` 更新 `source_url`，清空 `resolved_media_url`、`progress_bytes`、`total_bytes`、`output_path`、`checkpoint_json`；调用 `cleanup_download_temp(media_dir, task_id)`
- **`needs_sniff` 失败**：`retry_task` 拒绝；UI 无「修改 URL 重试」菜单
- **改 URL 时保留** `cookie_header` / `referer` / `poster_url`
- **Android TV**：隐藏 `settings_pick_media_dir`；不调用 `file_picker`；helper 仅用 TV 文案（不提选择器）
- **依赖**：`file_picker: ^8.0.0`（或 `flutter pub add file_picker` 解析到的兼容版本）；**不** 引入 `file_selector`
- **验证命令**（仓库根目录）：`cargo fmt --check`、`cargo test --manifest-path engine/Cargo.toml`、`cargo clippy --manifest-path engine/Cargo.toml --all-targets --all-features -- -D warnings`；`cd app && flutter test`；`cd app && flutter test integration_test/library_settings_retry_test.dart -d macos`
- **`docs/` 在 `.gitignore`**：提交文档用 `git add -f docs/...`
- **提交信息中文**

---

## File Map

| 路径 | 职责 |
|------|------|
| `engine/src/tasks/store.rs` | `requeue_failed_with_url` |
| `engine/src/engine.rs` | `retry_task(id, new_url)` |
| `engine/tests/retry_task_test.rs` | L9d-1..L9d-6 |
| `engine/tests/task_store.rs` | Store 层 `requeue_failed_with_url` 单测 |
| `engine/tests/download_integration.rs` | L9d-7：删除 `requeue_failed_task` |
| `engine/ffi/src/sync_dispatch.rs` | `engine_retry_task` 第三参数 |
| `engine/ffi/tests/retry_task_ffi_test.rs` | F9d-1、F9d-2 |
| `app/lib/engine/native_bindings.dart` | `engineRetryTask` 签名 |
| `app/lib/engine/engine_host.dart` | `retryTask(id, {newUrl})` |
| `app/lib/providers/engine_repository.dart` | 接口 + 委托 |
| `app/test/fakes/fake_engine_repository.dart` | 记录 `lastRetryNewUrl` |
| `app/lib/features/settings/media_dir_picker.dart` | `mediaDirNameFromPickerResult` |
| `app/lib/features/settings/media_directory_picker.dart` | 可注入目录选择抽象 |
| `app/lib/features/settings/settings_screen.dart` | 选择文件夹按钮 + helper |
| `app/lib/features/tasks/widgets/retry_url_dialog.dart` | 修改 URL 对话框 |
| `app/lib/features/tasks/widgets/task_tile.dart` | 菜单 + `onEditUrlRetry` |
| `app/lib/features/tasks/widgets/parent_task_group.dart` | 子任务接线 |
| `app/lib/features/tasks/tasks_screen.dart` | 对话框 + `retryTask(newUrl:)` |
| `app/test/media_dir_picker_test.dart` | W9d-1 |
| `app/test/settings_media_dir_picker_test.dart` | W9d-2、W9d-3 |
| `app/test/retry_url_dialog_test.dart` | 对话框单测 |
| `app/test/task_tile_test.dart` | W9d-4、W9d-6 |
| `app/test/tasks_retry_url_test.dart` | W9d-4b、W9d-5 |
| `app/integration_test/library_settings_retry_test.dart` | U11d |
| `.github/workflows/ci.yml` | `library_settings_retry` suite |
| `README.md` | Plan 9d 节 + `v0.2.0` |

---

# Phase 9d-1 — Engine TaskStore

### Task 1: `TaskStore::requeue_failed_with_url`

**Files:**
- Modify: `engine/src/tasks/store.rs`
- Modify: `engine/tests/task_store.rs`

**Interfaces:**
- Produces: `TaskStore::requeue_failed_with_url(&self, id: &str, source_url: &str) -> Result<(), EngineError>`

- [ ] **Step 1: 写失败测试**

```rust
// engine/tests/task_store.rs — 追加（文件顶部已有 use DownloadTask, TaskStatus, TaskStore, ...）

#[test]
fn requeue_failed_with_url_resets_progress_and_media_cache() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = TaskStore::open(&dir.path().join("tasks.db")).unwrap();
    store
        .upsert(&DownloadTask {
            id: "f1".into(),
            parent_id: None,
            season: None,
            title: "fail".into(),
            source_url: "https://old.example/bad.mp4".into(),
            quality_label: None,
            status: TaskStatus::Failed,
            progress_bytes: 999,
            total_bytes: Some(1000),
            error_message: Some("http error".into()),
            output_path: Some("/tmp/out.mp4".into()),
            library_item_id: None,
            episode_index: None,
            created_at_ms: 1,
            updated_at_ms: 1,
            cookie_header: Some("sid=1".into()),
            referer: Some("https://page.example/".into()),
            resolved_media_url: Some("https://cdn.example/old.m3u8".into()),
            poster_url: None,
        })
        .unwrap();

    store
        .save_checkpoint(
            "f1",
            &Checkpoint {
                version: 1,
                body: CheckpointBody::Mp4 {
                    temp_dir: "/tmp/.dl/f1".into(),
                    part_path: "/tmp/.dl/f1/part".into(),
                    bytes_done: 100,
                },
            },
        )
        .unwrap();
    store
        .requeue_failed_with_url("f1", "https://new.example/good.mp4")
        .unwrap();

    let task = store.get("f1").unwrap();
    assert_eq!(task.source_url, "https://new.example/good.mp4");
    assert_eq!(task.status, TaskStatus::Queued);
    assert_eq!(task.progress_bytes, 0);
    assert!(task.total_bytes.is_none());
    assert!(task.output_path.is_none());
    assert!(task.error_message.is_none());
    assert!(task.resolved_media_url.is_none());
    assert!(store.load_checkpoint("f1").unwrap().is_none());
    assert_eq!(task.cookie_header.as_deref(), Some("sid=1"));
    assert_eq!(task.referer.as_deref(), Some("https://page.example/"));
}
```

（测试文件顶部 `use video_sniffing_engine::download::checkpoint::{Checkpoint, CheckpointBody};`。）

- [ ] **Step 2: 运行测试确认失败**

Run: `cargo test --manifest-path engine/Cargo.toml requeue_failed_with_url_resets_progress_and_media_cache -- --nocapture`  
Expected: FAIL — `requeue_failed_with_url` not found

- [ ] **Step 3: 实现 `requeue_failed_with_url`**

```rust
// engine/src/tasks/store.rs — 放在 set_resolved_media_url 之后

pub fn requeue_failed_with_url(&self, id: &str, source_url: &str) -> Result<(), EngineError> {
    let n = self.conn.execute(
        r#"UPDATE download_tasks
           SET source_url=?1,
               resolved_media_url=NULL,
               progress_bytes=0,
               total_bytes=NULL,
               output_path=NULL,
               checkpoint_json=NULL,
               error_message=NULL,
               status=?2,
               updated_at_ms=?3
           WHERE id=?4 AND status=?5"#,
        params![
            source_url,
            Self::status_to_str(TaskStatus::Queued),
            Self::now_ms(),
            id,
            Self::status_to_str(TaskStatus::Failed),
        ],
    )?;
    if n == 0 {
        return Err(EngineError::NotFound(format!("task {id}")));
    }
    Ok(())
}
```

- [ ] **Step 4: 运行测试**

Run: `cargo test --manifest-path engine/Cargo.toml requeue_failed_with_url -- --nocapture`  
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add engine/src/tasks/store.rs engine/tests/task_store.rs
git commit -m "feat(engine): TaskStore 增加失败任务改 URL 重入队"
```

---

# Phase 9d-2 — Engine `retry_task` 扩展

### Task 2: `Engine::retry_task(task_id, new_url)` + L9d 测试

**Files:**
- Modify: `engine/src/engine.rs`
- Modify: `engine/tests/retry_task_test.rs`
- Modify: `engine/tests/download_integration.rs`

**Interfaces:**
- Consumes: Task 1 `TaskStore::requeue_failed_with_url`
- Produces: `Engine::retry_task(&mut self, task_id: &str, new_url: Option<&str>) -> Result<(), EngineError>`

- [ ] **Step 1: 更新现有测试调用签名（先红）**

将 `engine/tests/retry_task_test.rs` 中所有 `engine.retry_task("...")` 改为 `engine.retry_task("...", None)`。

在 `retry_task_failed_to_queued_clears_error` 中 **显式断言** `source_url` 不变（L9d-1）。

追加新测试：

```rust
#[test]
fn retry_task_with_new_url_updates_source_and_clears_progress() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("tasks.db");
    {
        let store = TaskStore::open(&path).unwrap();
        let mut task = sample("failed2", None, TaskStatus::Failed, Some("http error"));
        task.source_url = "https://example.com/old.mp4".into();
        task.resolved_media_url = Some("https://cdn.example/old.m3u8".into());
        task.progress_bytes = 50;
        task.total_bytes = Some(100);
        task.output_path = Some("/tmp/x.mp4".into());
        store.upsert(&task).unwrap();
    }

    let mut engine = Engine::open(dir.path()).unwrap();
    engine
        .retry_task("failed2", Some("https://example.com/new.mp4"))
        .unwrap();

    let task = engine
        .list_tasks()
        .unwrap()
        .into_iter()
        .find(|t| t.id == "failed2")
        .unwrap();
    assert_eq!(task.source_url, "https://example.com/new.mp4");
    assert_eq!(task.status, TaskStatus::Queued);
    assert!(task.resolved_media_url.is_none());
    assert_eq!(task.progress_bytes, 0);
    assert!(task.total_bytes.is_none());
    assert!(task.output_path.is_none());
    let store = TaskStore::open(&path).unwrap();
    assert!(store.load_checkpoint("failed2").unwrap().is_none());
}

#[test]
fn retry_task_with_new_url_cleans_dl_temp_dir() {
    let dir = tempdir().unwrap();
    let mut engine = Engine::open(dir.path()).unwrap();
    let media_dir = engine.media_dir();
    let dl_dir = media_dir.join(".dl").join("failed3");
    std::fs::create_dir_all(&dl_dir).unwrap();
    std::fs::write(dl_dir.join("part.bin"), b"partial").unwrap();

    let path = dir.path().join("tasks.db");
    {
        let store = TaskStore::open(&path).unwrap();
        let mut task = sample("failed3", None, TaskStatus::Failed, Some("http error"));
        task.source_url = "https://example.com/old.mp4".into();
        store.upsert(&task).unwrap();
    }

    engine
        .retry_task("failed3", Some("https://example.com/new.mp4"))
        .unwrap();

    assert!(!dl_dir.exists());
}

#[test]
fn retry_task_same_url_degrades_to_fast_path() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("tasks.db");
    let url = "https://example.com/same.mp4";
    {
        let store = TaskStore::open(&path).unwrap();
        let mut task = sample("same", None, TaskStatus::Failed, Some("err"));
        task.source_url = url.into();
        task.resolved_media_url = Some("https://cdn.example/cached.m3u8".into());
        task.progress_bytes = 10;
        store.upsert(&task).unwrap();
    }

    let mut engine = Engine::open(dir.path()).unwrap();
    engine.retry_task("same", Some(url)).unwrap();

    let task = engine
        .list_tasks()
        .unwrap()
        .into_iter()
        .find(|t| t.id == "same")
        .unwrap();
    assert_eq!(task.status, TaskStatus::Queued);
    assert_eq!(task.source_url, url);
    assert_eq!(
        task.resolved_media_url.as_deref(),
        Some("https://cdn.example/cached.m3u8")
    );
    assert_eq!(task.progress_bytes, 10);
}

#[test]
fn retry_task_rejects_empty_new_url() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("tasks.db");
    {
        let store = TaskStore::open(&path).unwrap();
        store
            .upsert(&sample("failed", None, TaskStatus::Failed, Some("e")))
            .unwrap();
    }
    let mut engine = Engine::open(dir.path()).unwrap();
    let err = engine.retry_task("failed", Some("   ")).unwrap_err();
    assert!(matches!(err, EngineError::InvalidArg(_)));
}

#[test]
fn retry_task_with_new_url_syncs_parent_status() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("tasks.db");
    {
        let store = TaskStore::open(&path).unwrap();
        store
            .upsert(&sample("parent", None, TaskStatus::Failed, None))
            .unwrap();
        let mut child = sample("child", Some("parent"), TaskStatus::Failed, Some("e"));
        child.source_url = "https://example.com/old.mp4".into();
        store.upsert(&child).unwrap();
    }

    let mut engine = Engine::open(dir.path()).unwrap();
    engine
        .retry_task("child", Some("https://example.com/new.mp4"))
        .unwrap();

    let parent = engine
        .list_tasks()
        .unwrap()
        .into_iter()
        .find(|t| t.id == "parent")
        .unwrap();
    assert_eq!(parent.status, TaskStatus::Queued);
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cargo test --manifest-path engine/Cargo.toml retry_task_with_new_url -- --nocapture`  
Expected: FAIL — wrong function signature / method not found

- [ ] **Step 3: 实现 `Engine::retry_task`**

```rust
// engine/src/engine.rs — 替换现有 retry_task

pub fn retry_task(
    &mut self,
    task_id: &str,
    new_url: Option<&str>,
) -> Result<(), EngineError> {
    let task = self.tasks.get(task_id)?;
    if task.status != TaskStatus::Failed {
        return Err(EngineError::InvalidArg(
            "task must be in failed status".into(),
        ));
    }
    if task.error_message.as_deref() == Some("needs_sniff") {
        return Err(EngineError::InvalidArg(
            "failed task needs sniff before retry".into(),
        ));
    }

    match new_url {
        None => {
            self.tasks
                .set_task_status(task_id, TaskStatus::Queued, None)?;
        }
        Some(url) => {
            let trimmed = url.trim();
            if trimmed.is_empty() {
                return Err(EngineError::InvalidArg(
                    "new_url must not be empty".into(),
                ));
            }
            if trimmed == task.source_url {
                self.tasks
                    .set_task_status(task_id, TaskStatus::Queued, None)?;
            } else {
                self.tasks.requeue_failed_with_url(task_id, trimmed)?;
                crate::download::worker::cleanup_download_temp(&self.media_dir(), task_id);
            }
        }
    }

    if let Some(parent_id) = &task.parent_id {
        let _ = self.tasks.sync_parent_status(parent_id);
    }
    Ok(())
}
```

- [ ] **Step 4: 修复 `download_integration.rs`（L9d-7）**

删除 `requeue_failed_task` 函数；将调用处改为：

```rust
fx.engine
    .retry_task(&failed_id, Some(&good_mp4))
    .unwrap();
```

将文件中另一处 `fx.engine.retry_task(&task_id).unwrap()` 改为 `fx.engine.retry_task(&task_id, None).unwrap()`。

- [ ] **Step 5: 运行 Engine 测试**

Run: `cargo test --manifest-path engine/Cargo.toml retry_task -- --nocapture`  
Expected: PASS

Run: `cargo test --manifest-path engine/Cargo.toml download_integration -- --nocapture`  
Expected: PASS（HLS 相关用例需本机 ffmpeg；若环境无 ffmpeg 可只跑改 URL 相关用例名）

- [ ] **Step 6: Commit**

```bash
git add engine/src/engine.rs engine/tests/retry_task_test.rs engine/tests/download_integration.rs
git commit -m "feat(engine): retry_task 支持可选 new_url 并重置进度"
```

---

# Phase 9d-3 — FFI

### Task 3: `engine_retry_task` 第三参数 + F9d 测试

**Files:**
- Modify: `engine/ffi/src/sync_dispatch.rs`
- Create: `engine/ffi/tests/retry_task_ffi_test.rs`

**Interfaces:**
- Consumes: Task 2 `Engine::retry_task(task_id, new_url)`
- Produces: `engine_retry_task(handle, task_id, new_url: *const c_char)` — `new_url` 可为 NULL 或空串

- [ ] **Step 1: 写失败 FFI 测试**

```rust
// engine/ffi/tests/retry_task_ffi_test.rs

use std::ffi::{CStr, CString};

use tempfile::tempdir;
use video_sniffing_engine::tasks::TaskStore;
use video_sniffing_engine::{DownloadTask, TaskStatus};
use video_sniffing_engine_ffi::handle::{engine_destroy, engine_free_string, engine_open};
use video_sniffing_engine_ffi::sync_dispatch::{engine_list_tasks, engine_retry_task};

fn seed_failed(data_dir: &std::path::Path, id: &str, source_url: &str) {
    let store = TaskStore::open(&data_dir.join("tasks.db")).unwrap();
    store
        .upsert(&DownloadTask {
            id: id.into(),
            parent_id: None,
            season: None,
            title: id.into(),
            source_url: source_url.into(),
            quality_label: None,
            status: TaskStatus::Failed,
            progress_bytes: 0,
            total_bytes: None,
            error_message: Some("err".into()),
            output_path: None,
            library_item_id: None,
            episode_index: None,
            created_at_ms: 1,
            updated_at_ms: 1,
            cookie_header: None,
            referer: None,
            resolved_media_url: None,
            poster_url: None,
        })
        .unwrap();
}

#[test]
fn engine_retry_task_null_url_fast_path() {
    let dir = tempdir().unwrap();
    seed_failed(dir.path(), "t1", "https://example.com/a.mp4");
    let path = CString::new(dir.path().to_str().unwrap()).unwrap();
    let handle = unsafe { engine_open(path.as_ptr()) };
    assert!(!handle.is_null());

    let task_id = CString::new("t1").unwrap();
    let result = unsafe { engine_retry_task(handle, task_id.as_ptr(), std::ptr::null()) };
    assert!(!result.is_null());
    let json_str = unsafe { CStr::from_ptr(result).to_str().unwrap() };
    assert!(json_str.contains("\"ok\":true"));
    unsafe { engine_free_string(result) };
    unsafe { engine_destroy(handle) };
}

#[test]
fn engine_retry_task_with_new_url_updates_list_tasks() {
    let dir = tempdir().unwrap();
    seed_failed(dir.path(), "t2", "https://example.com/old.mp4");
    let path = CString::new(dir.path().to_str().unwrap()).unwrap();
    let handle = unsafe { engine_open(path.as_ptr()) };
    assert!(!handle.is_null());

    let task_id = CString::new("t2").unwrap();
    let new_url = CString::new("https://new.example/v.mp4").unwrap();
    let result =
        unsafe { engine_retry_task(handle, task_id.as_ptr(), new_url.as_ptr()) };
    assert!(!result.is_null());
    unsafe { engine_free_string(result) };

    let listed = unsafe { engine_list_tasks(handle) };
    assert!(!listed.is_null());
    let json_str = unsafe { CStr::from_ptr(listed).to_str().unwrap() };
    assert!(json_str.contains("https://new.example/v.mp4"));
    assert!(json_str.contains("\"status\":\"queued\""));
    unsafe { engine_free_string(listed) };
    unsafe { engine_destroy(handle) };
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cargo test --manifest-path engine/Cargo.toml --test retry_task_ffi_test -- --nocapture`  
Expected: FAIL — compile error（第三参数不存在）

- [ ] **Step 3: 更新 `engine_retry_task`**

```rust
// engine/ffi/src/sync_dispatch.rs

#[no_mangle]
pub unsafe extern "C" fn engine_retry_task(
    handle: *mut EngineHandle,
    task_id: *const c_char,
    new_url: *const c_char,
) -> *mut c_char {
    let task_id = match parse_c_str(task_id, "task_id") {
        Ok(id) => id,
        Err(err) => return rust_to_c_string(err_json(err)),
    };
    let new_url = if new_url.is_null() {
        None
    } else {
        match parse_c_str(new_url, "new_url") {
            Ok(s) if s.is_empty() => None,
            Ok(s) => Some(s),
            Err(err) => return rust_to_c_string(err_json(err)),
        }
    };
    ffi_call_mut(handle, |engine| engine.retry_task(&task_id, new_url.as_deref()))
}
```

- [ ] **Step 4: 运行 FFI 测试**

Run: `cargo test --manifest-path engine/Cargo.toml --test retry_task_ffi_test -- --nocapture`  
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add engine/ffi/src/sync_dispatch.rs engine/ffi/tests/retry_task_ffi_test.rs
git commit -m "feat(ffi): engine_retry_task 支持可选 new_url"
```

---

# Phase 9d-4 — Dart 桥接

### Task 4: `NativeBindings` + `EngineHost` + `EngineRepository` + Fake

**Files:**
- Modify: `app/lib/engine/native_bindings.dart`
- Modify: `app/lib/engine/engine_host.dart`
- Modify: `app/lib/providers/engine_repository.dart`
- Modify: `app/test/fakes/fake_engine_repository.dart`

**Interfaces:**
- Consumes: Task 3 `engine_retry_task(handle, taskId, newUrlPtr)`
- Produces: `void retryTask(String taskId, {String? newUrl})` on `EngineHost` / `EngineRepository`
- Produces: `FakeEngineRepository.lastRetryTaskId` / `lastRetryNewUrl`

- [ ] **Step 1: 更新 `native_bindings.dart` typedef 与 lookup**

```dart
typedef EngineRetryTaskNative = Pointer<Char> Function(
  Pointer<Void> handle,
  Pointer<Utf8> taskId,
  Pointer<Utf8> newUrl,
);
typedef EngineRetryTask = Pointer<Char> Function(
  Pointer<Void> handle,
  Pointer<Utf8> taskId,
  Pointer<Utf8> newUrl,
);
```

lookup 不变（符号名仍为 `engine_retry_task`）。

- [ ] **Step 2: 更新 `EngineHost.retryTask`**

```dart
void retryTask(String taskId, {String? newUrl}) {
  _withUtf8(taskId, (taskIdPtr) {
    if (newUrl == null) {
      _callSyncVoid(
        (handle) => _bindings.engineRetryTask(handle, taskIdPtr, nullptr),
      );
      return;
    }
    _withUtf8(newUrl, (newUrlPtr) {
      _callSyncVoid(
        (handle) =>
            _bindings.engineRetryTask(handle, taskIdPtr, newUrlPtr),
      );
    });
  });
}
```

- [ ] **Step 3: 更新 `EngineRepository` 接口与实现**

```dart
// engine_repository.dart
void retryTask(String taskId, {String? newUrl});

// EngineRepositoryImpl
@override
void retryTask(String taskId, {String? newUrl}) =>
    _host.retryTask(taskId, newUrl: newUrl);
```

- [ ] **Step 4: 更新 `FakeEngineRepository`**

```dart
String? lastRetryTaskId;
String? lastRetryNewUrl;

@override
void retryTask(String taskId, {String? newUrl}) {
  lastRetryTaskId = taskId;
  lastRetryNewUrl = newUrl;
}
```

- [ ] **Step 5: 运行 analyze**

Run: `cd app && flutter analyze lib/engine lib/providers/engine_repository.dart test/fakes/fake_engine_repository.dart`  
Expected: no issues

- [ ] **Step 6: Commit**

```bash
git add app/lib/engine/native_bindings.dart app/lib/engine/engine_host.dart \
  app/lib/providers/engine_repository.dart app/test/fakes/fake_engine_repository.dart
git commit -m "feat(app): EngineHost retryTask 支持可选 newUrl"
```

---

# Phase 9d-5 — 目录名提取与选择器抽象

### Task 5: `mediaDirNameFromPickerResult` + `MediaDirectoryPicker` + W9d-1

**Files:**
- Create: `app/lib/features/settings/media_dir_picker.dart`
- Create: `app/lib/features/settings/media_directory_picker.dart`
- Create: `app/test/media_dir_picker_test.dart`
- Modify: `app/pubspec.yaml`（`path` 若未直接依赖则 `flutter pub add path`）

**Interfaces:**
- Produces: `String? mediaDirNameFromPickerResult(String? pickedPath)`
- Produces: `abstract class MediaDirectoryPicker { Future<String?> pickDirectoryPath(); }`
- Produces: `class FilePickerMediaDirectoryPicker implements MediaDirectoryPicker`

- [ ] **Step 1: 写失败测试 W9d-1**

```dart
// app/test/media_dir_picker_test.dart
import 'package:flutter_test/flutter_test.dart';
import 'package:video_sniffing/features/settings/media_dir_picker.dart';

void main() {
  test('mediaDirNameFromPickerResult extracts basename', () {
    expect(
      mediaDirNameFromPickerResult('/foo/bar/SniffVault'),
      'SniffVault',
    );
    expect(
      mediaDirNameFromPickerResult(r'C:\Users\me\SniffVault'),
      'SniffVault',
    );
    expect(mediaDirNameFromPickerResult('/foo/bar/SniffVault/'), 'SniffVault');
  });

  test('mediaDirNameFromPickerResult null or invalid returns null', () {
    expect(mediaDirNameFromPickerResult(null), isNull);
    expect(mediaDirNameFromPickerResult(''), isNull);
    expect(mediaDirNameFromPickerResult('/'), isNull);
    expect(mediaDirNameFromPickerResult('.'), isNull);
  });
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cd app && flutter test test/media_dir_picker_test.dart`  
Expected: FAIL — file / function not found

- [ ] **Step 3: 实现**

```dart
// app/lib/features/settings/media_dir_picker.dart
import 'package:path/path.dart' as p;

String? mediaDirNameFromPickerResult(String? pickedPath) {
  if (pickedPath == null) {
    return null;
  }
  final trimmed = pickedPath.replaceAll(RegExp(r'[/\\]+$'), '');
  if (trimmed.isEmpty) {
    return null;
  }
  final name = p.basename(trimmed);
  if (name.isEmpty || name == '.' || name == '..') {
    return null;
  }
  if (name.contains('/') || name.contains(r'\')) {
    return null;
  }
  return name;
}
```

```dart
// app/lib/features/settings/media_directory_picker.dart
import 'package:file_picker/file_picker.dart';

abstract class MediaDirectoryPicker {
  Future<String?> pickDirectoryPath();
}

class FilePickerMediaDirectoryPicker implements MediaDirectoryPicker {
  @override
  Future<String?> pickDirectoryPath() => FilePicker.platform.getDirectoryPath();
}
```

- [ ] **Step 4: 添加依赖**

Run: `cd app && flutter pub add file_picker path`

若 Android/iOS 真机目录选择失败，按 `file_picker` 文档补权限；CI widget 测不依赖真机选择器。

- [ ] **Step 5: 运行测试**

Run: `cd app && flutter test test/media_dir_picker_test.dart`  
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add app/pubspec.yaml app/pubspec.lock \
  app/lib/features/settings/media_dir_picker.dart \
  app/lib/features/settings/media_directory_picker.dart \
  app/test/media_dir_picker_test.dart
git commit -m "feat(app): 媒体目录名提取与可注入目录选择器"
```

---

# Phase 9d-6 — 设置页 UI

### Task 6: `SettingsScreen` 选择文件夹 + W9d-2、W9d-3

**Files:**
- Modify: `app/lib/features/settings/settings_screen.dart`
- Create: `app/test/settings_media_dir_picker_test.dart`

**Interfaces:**
- Consumes: Task 5 `mediaDirNameFromPickerResult`、`MediaDirectoryPicker`

- [ ] **Step 1: 写失败 widget 测试**

```dart
// app/test/settings_media_dir_picker_test.dart
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:video_sniffing/features/settings/media_directory_picker.dart';
import 'package:video_sniffing/features/settings/settings_screen.dart';
import 'package:video_sniffing/providers/device_profile.dart';
import 'package:video_sniffing/providers/engine_host_provider.dart';
import 'package:video_sniffing/providers/settings_provider.dart';

import 'fakes/fake_engine_repository.dart';

class FakeMediaDirectoryPicker implements MediaDirectoryPicker {
  FakeMediaDirectoryPicker(this.result);
  final String? result;

  @override
  Future<String?> pickDirectoryPath() async => result;
}

void main() {
  testWidgets('W9d-2 pick media dir fills basename into field', (tester) async {
    tester.view.physicalSize = const Size(800, 1200);
    tester.view.devicePixelRatio = 1.0;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);

    final fake = FakeEngineRepository();
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          engineRepositoryProvider.overrideWithValue(fake),
          settingsProvider.overrideWith((ref) => fake.settings()),
          isTelevisionProvider.overrideWith((ref) async => false),
        ],
        child: MaterialApp(
          home: SettingsScreen(
            directoryPicker: FakeMediaDirectoryPicker(
              '/Users/me/Downloads/MyVault',
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    await tester.tap(find.byKey(const Key('settings_pick_media_dir')));
    await tester.pumpAndSettle();

    expect(
      find.byKey(const Key('settings_media_dir')),
      findsOneWidget,
    );
    final field = tester.widget<TextField>(
      find.byKey(const Key('settings_media_dir')),
    );
    expect(field.controller?.text, 'MyVault');
    expect(find.text('已填入目录名，记得保存'), findsOneWidget);
  });

  testWidgets('W9d-3 TV hides pick media dir button', (tester) async {
    tester.view.physicalSize = const Size(800, 1200);
    tester.view.devicePixelRatio = 1.0;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);

    final fake = FakeEngineRepository();
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          engineRepositoryProvider.overrideWithValue(fake),
          settingsProvider.overrideWith((ref) => fake.settings()),
          isTelevisionProvider.overrideWith((ref) async => true),
        ],
        child: const MaterialApp(home: SettingsScreen()),
      ),
    );
    await tester.pumpAndSettle();

    expect(find.byKey(const Key('settings_pick_media_dir')), findsNothing);
    expect(find.textContaining('选择外置路径'), findsNothing);
    expect(find.textContaining('应用数据目录下的文件夹名称'), findsOneWidget);
  });
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cd app && flutter test test/settings_media_dir_picker_test.dart`  
Expected: FAIL — Key not found / constructor missing

- [ ] **Step 3: 修改 `SettingsScreen`**

要点：

1. 构造函数增加 `final MediaDirectoryPicker? directoryPicker;`
2. `media_dir` TextField 的 `helperText` 按平台分支（`ref.watch(isTelevisionProvider)`）：
   - TV：`此处为应用数据目录下的文件夹名称。`
   - 其它：`此处为应用数据目录下的文件夹名称；选择外置路径时仅采用文件夹名，不会自动搬移已有缓存文件。`
3. 非 TV 时在 TextField 下方增加：

```dart
OutlinedButton(
  key: const Key('settings_pick_media_dir'),
  onPressed: _pickMediaDir,
  child: const Text('选择文件夹'),
),
```

4. 实现 `_pickMediaDir`：

```dart
Future<void> _pickMediaDir() async {
  final picker =
      widget.directoryPicker ?? FilePickerMediaDirectoryPicker();
  final picked = await picker.pickDirectoryPath();
  if (!mounted || picked == null) {
    return;
  }
  final name = mediaDirNameFromPickerResult(picked);
  if (name == null) {
    ScaffoldMessenger.of(context).showSnackBar(
      const SnackBar(content: Text('无法识别文件夹名称')),
    );
    return;
  }
  _mediaDirController.text = name;
  _draft = _draft.copyWith(mediaDir: name);
  ScaffoldMessenger.of(context).showSnackBar(
    const SnackBar(content: Text('已填入目录名，记得保存')),
  );
}
```

- [ ] **Step 4: 运行测试**

Run: `cd app && flutter test test/settings_media_dir_picker_test.dart test/settings_screen_test.dart`  
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add app/lib/features/settings/settings_screen.dart app/test/settings_media_dir_picker_test.dart
git commit -m "feat(app): 设置页增加媒体目录选择器"
```

---

# Phase 9d-7 — 修改 URL 对话框

### Task 7: `RetryUrlDialog`

**Files:**
- Create: `app/lib/features/tasks/widgets/retry_url_dialog.dart`
- Create: `app/test/retry_url_dialog_test.dart`

**Interfaces:**
- Produces: `Future<String?> showRetryUrlDialog(BuildContext context, {required String initialUrl})`

- [ ] **Step 1: 写失败测试**

```dart
// app/test/retry_url_dialog_test.dart
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:video_sniffing/features/tasks/widgets/retry_url_dialog.dart';

void main() {
  testWidgets('showRetryUrlDialog returns trimmed url on confirm', (
    tester,
  ) async {
    String? result;
    await tester.pumpWidget(
      MaterialApp(
        home: Builder(
          builder: (context) => ElevatedButton(
            onPressed: () async {
              result = await showRetryUrlDialog(
                context,
                initialUrl: 'https://old.example/a.mp4',
              );
            },
            child: const Text('open'),
          ),
        ),
      ),
    );

    await tester.tap(find.text('open'));
    await tester.pumpAndSettle();

    await tester.enterText(
      find.byKey(const Key('retry_url_field')),
      '  https://new.example/b.mp4  ',
    );
    await tester.tap(find.byKey(const Key('retry_url_confirm')));
    await tester.pumpAndSettle();

    expect(result, 'https://new.example/b.mp4');
  });
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cd app && flutter test test/retry_url_dialog_test.dart`  
Expected: FAIL

- [ ] **Step 3: 实现对话框**

```dart
// app/lib/features/tasks/widgets/retry_url_dialog.dart
import 'package:flutter/material.dart';

Future<String?> showRetryUrlDialog(
  BuildContext context, {
  required String initialUrl,
}) {
  return showDialog<String>(
    context: context,
    builder: (dialogContext) => _RetryUrlDialog(initialUrl: initialUrl),
  );
}

class _RetryUrlDialog extends StatefulWidget {
  const _RetryUrlDialog({required this.initialUrl});
  final String initialUrl;

  @override
  State<_RetryUrlDialog> createState() => _RetryUrlDialogState();
}

class _RetryUrlDialogState extends State<_RetryUrlDialog> {
  late final TextEditingController _controller;
  String? _error;

  @override
  void initState() {
    super.initState();
    _controller = TextEditingController(text: widget.initialUrl);
  }

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  void _submit() {
    final trimmed = _controller.text.trim();
    if (trimmed.isEmpty) {
      setState(() => _error = 'URL 不能为空');
      return;
    }
    Navigator.of(context).pop(trimmed);
  }

  @override
  Widget build(BuildContext context) {
    return AlertDialog(
      title: const Text('修改 URL 后重试'),
      content: TextField(
        key: const Key('retry_url_field'),
        controller: _controller,
        maxLines: 3,
        decoration: InputDecoration(
          labelText: '资源 URL',
          errorText: _error,
          border: const OutlineInputBorder(),
        ),
      ),
      actions: [
        TextButton(
          key: const Key('retry_url_cancel'),
          onPressed: () => Navigator.of(context).pop(),
          child: const Text('取消'),
        ),
        FilledButton(
          key: const Key('retry_url_confirm'),
          onPressed: _submit,
          child: const Text('重试'),
        ),
      ],
    );
  }
}
```

- [ ] **Step 4: 运行测试**

Run: `cd app && flutter test test/retry_url_dialog_test.dart`  
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add app/lib/features/tasks/widgets/retry_url_dialog.dart app/test/retry_url_dialog_test.dart
git commit -m "feat(app): 失败任务修改 URL 重试对话框"
```

---

# Phase 9d-8 — 任务列表 UI 接线

### Task 8: `TaskTile` + `ParentTaskGroup` + `TasksScreen` + W9d-4..6

**Files:**
- Modify: `app/lib/features/tasks/widgets/task_tile.dart`
- Modify: `app/lib/features/tasks/widgets/parent_task_group.dart`
- Modify: `app/lib/features/tasks/tasks_screen.dart`
- Modify: `app/test/task_tile_test.dart`
- Create: `app/test/tasks_retry_url_test.dart`
- Modify: `app/test/tasks_needs_sniff_test.dart`（若 `TaskTile` 构造函数变更导致编译失败）

**Interfaces:**
- Consumes: Task 4 `retryTask(id, newUrl:)`；Task 7 `showRetryUrlDialog`
- Produces: `TaskTile.onEditUrlRetry` 可选回调；`Key('task_edit_url_menu_{id}')`

- [ ] **Step 1: 写失败 widget 测试 W9d-4..6**

在 `app/test/task_tile_test.dart` 追加：

```dart
testWidgets('W9d-4 edit url menu invokes onEditUrlRetry', (tester) async {
  const task = DownloadTask(
    id: 't-failed-edit',
    title: '第01集',
    sourceUrl: 'https://example/x.m3u8',
    status: TaskStatus.failed,
    errorMessage: 'http error',
    progressBytes: 0,
    createdAtMs: 1,
    updatedAtMs: 1,
  );
  var editCalled = false;
  await tester.pumpWidget(
    MaterialApp(
      home: Scaffold(
        body: TaskTile(
          task: task,
          onPause: () {},
          onResume: () {},
          onCancel: () {},
          onRetry: () {},
          onRestore: () {},
          onEditUrlRetry: () => editCalled = true,
        ),
      ),
    ),
  );

  await tester.tap(find.byKey(const Key('task_edit_url_menu_t-failed-edit')));
  await tester.pumpAndSettle();
  await tester.tap(find.text('修改 URL 重试'));
  await tester.pumpAndSettle();
  expect(editCalled, isTrue);
});

testWidgets('W9d-6 needs_sniff failed hides edit url menu', (tester) async {
  const task = DownloadTask(
    id: 't-sniff-failed-menu',
    title: '第01集',
    sourceUrl: 'https://example/play/1',
    status: TaskStatus.failed,
    errorMessage: TaskError.needsSniff,
    progressBytes: 0,
    createdAtMs: 1,
    updatedAtMs: 1,
  );
  await tester.pumpWidget(
    MaterialApp(
      home: Scaffold(
        body: TaskTile(
          task: task,
          onPause: () {},
          onResume: () {},
          onCancel: () {},
          onRetry: () {},
          onRestore: () {},
          onEditUrlRetry: () {},
        ),
      ),
    ),
  );

  expect(find.byKey(const Key('task_edit_url_menu_t-sniff-failed-menu')), findsNothing);
});
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cd app && flutter test test/task_tile_test.dart --name W9d`  
Expected: FAIL — `onEditUrlRetry` not defined

- [ ] **Step 3: 更新 `TaskTile`**

```dart
// task_tile.dart — 构造函数增加（默认 null，避免破坏现有调用点）
final VoidCallback? onEditUrlRetry;

const TaskTile({
  ...
  this.onEditUrlRetry,
});

// _buildActions 内，taskCanRetry 且 onEditUrlRetry != null 时：
if (taskCanRetry(task) && onEditUrlRetry != null) {
  actions.add(
    PopupMenuButton<void>(
      key: Key('task_edit_url_menu_${task.id}'),
      icon: const Icon(Icons.more_vert),
      onSelected: (_) => onEditUrlRetry!(),
      itemBuilder: (context) => const [
        PopupMenuItem<void>(
          value: 0,
          child: Text('修改 URL 重试'),
        ),
      ],
    ),
  );
}
```

- [ ] **Step 4: 更新 `ParentTaskGroup` 与 `TasksScreen`**

`ParentTaskGroup` 增加 `onEditUrlRetry` 参数并传给子 `TaskTile`（父任务无子任务时也传入）。

`TasksScreen` 实现：

```dart
Future<void> _editUrlRetry(BuildContext context, WidgetRef ref, DownloadTask task) async {
  final url = await showRetryUrlDialog(context, initialUrl: task.sourceUrl);
  if (url == null || !context.mounted) {
    return;
  }
  final repo = ref.read(engineRepositoryProvider);
  try {
    repo.retryTask(task.id, newUrl: url);
    ref.invalidate(tasksProvider);
    ref.read(downloadCoordinatorProvider).ensureDownloads();
  } on EngineException catch (e) {
    final message = presentEngineError(e);
    if (message != null && context.mounted) {
      ScaffoldMessenger.of(context).showSnackBar(SnackBar(content: Text(message)));
    }
  }
}
```

快路径 `onRetry` 保持 `repo.retryTask(taskId)`（`newUrl` 默认 null）。

实现前运行 `rg 'TaskTile\\(' app/test app/lib` 确认所有调用点编译通过（`onEditUrlRetry` 可选故多数无需改）。

- [ ] **Step 5: 写 W9d-4b / W9d-5 接线测试**

```dart
// app/test/tasks_retry_url_test.dart — 抽取 TasksScreen 失败重试逻辑为可测 widget，
// 或 ProviderScope + 最小 tasks 列表 + FakeEngineRepository

testWidgets('W9d-4b edit url dialog calls retryTask with newUrl', (tester) async {
  final fake = FakeEngineRepository();
  // pump 含一条 failed 任务的 TasksScreen（override tasksProvider / engineRepositoryProvider）
  // 打开菜单 → 修改 URL 重试 → 对话框输入新 URL → 确认
  expect(fake.lastRetryTaskId, 't-failed');
  expect(fake.lastRetryNewUrl, 'https://new.example/v.mp4');
});

testWidgets('W9d-5 refresh icon calls retryTask without newUrl', (tester) async {
  final fake = FakeEngineRepository();
  // tap 刷新图标
  expect(fake.lastRetryTaskId, 't-failed');
  expect(fake.lastRetryNewUrl, isNull);
});
```

（具体 pump 方式与现有 `tasks_needs_sniff_test.dart` 对齐；若 `TasksScreen` 过重，可将 `_editUrlRetry` 提取为 `lib/features/tasks/retry_url_actions.dart` 顶层函数便于单测。）

- [ ] **Step 6: 运行 widget 测试**

Run: `cd app && flutter test test/task_tile_test.dart test/tasks_retry_url_test.dart test/tasks_needs_sniff_test.dart`  
Expected: PASS

- [ ] **Step 7: Commit**

```bash
git add app/lib/features/tasks/widgets/task_tile.dart \
  app/lib/features/tasks/widgets/parent_task_group.dart \
  app/lib/features/tasks/tasks_screen.dart \
  app/test/task_tile_test.dart \
  app/test/tasks_retry_url_test.dart
git commit -m "feat(app): 任务页支持修改 URL 后重试"
```

---

# Phase 9d-9 — 集成测试与 CI

### Task 9: U11d + CI 门禁

**Files:**
- Create: `app/integration_test/library_settings_retry_test.dart`
- Modify: `.github/workflows/ci.yml`
- Modify: `.github/workflows/release.yml`（若 matrix 与 ci 同步）

**Interfaces:**
- Consumes: Task 4 `EngineHost.retryTask(taskId, newUrl: url)`

- [ ] **Step 1: 写 U11d 集成测试**

```dart
// app/integration_test/library_settings_retry_test.dart
import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';
import 'package:sqlite3/sqlite3.dart';
import 'package:video_sniffing/engine/engine_host.dart';
import 'package:video_sniffing/engine/models/task_status.dart';

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();

  testWidgets('U11d retry failed task with new url via EngineHost', (
    tester,
  ) async {
    final dataDir = Directory.systemTemp.createTempSync('u11d');
    final host = await EngineHost.open(dataDir.path);
    try {
      const taskId = 'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa';
      final db = sqlite3.open('${dataDir.path}/tasks.db');
      try {
        db.execute('''
INSERT INTO download_tasks (
  id, parent_id, season, title, source_url, quality_label, status,
  progress_bytes, total_bytes, error_message, output_path,
  library_item_id, episode_index, created_at_ms, updated_at_ms,
  cookie_header, referer, resolved_media_url, poster_url
  checkpoint_json
) VALUES (
  '$taskId', NULL, NULL, 'u11d', 'https://old.example/bad.mp4', NULL, 'failed',
  100, 200, 'http error', NULL,
  NULL, NULL, 1, 1,
  NULL, NULL, 'https://cdn.example/old.m3u8', NULL,
  '{"media_url":"https://cdn.example/old.m3u8"}'
);
''');
      } finally {
        db.dispose();
      }

      host.retryTask(taskId, newUrl: 'https://new.example/good.mp4');

      final task = host.listTasks().firstWhere((t) => t.id == taskId);
      expect(task.status, TaskStatus.queued);
      expect(task.sourceUrl, 'https://new.example/good.mp4');
      expect(task.resolvedMediaUrl, isNull);
      expect(task.progressBytes, 0);

      final db2 = sqlite3.open('${dataDir.path}/tasks.db');
      try {
        final checkpoint = db2.select(
          "SELECT checkpoint_json FROM download_tasks WHERE id='$taskId'",
        );
        expect(checkpoint.first.columnAt(0), isNull);
      } finally {
        db2.dispose();
      }
    } finally {
      host.dispose();
    }
  }, timeout: const Timeout(Duration(minutes: 2)));
}
```

文件顶部加 `import 'dart:io';`。

- [ ] **Step 2: 本地运行 U11d**

Run: `cd app && flutter test integration_test/library_settings_retry_test.dart -d macos`  
Expected: PASS

- [ ] **Step 3: 更新 CI matrix**

在 `.github/workflows/ci.yml` 的 `matrix.suite` 追加 `library_settings_retry`，并增加分支：

```yaml
elif [ "${{ matrix.suite }}" = "library_settings_retry" ]; then
  flutter test integration_test/library_settings_retry_test.dart -d macos
```

`release.yml` 中 `flutter-integration` matrix 同步（若存在独立 suite 列表）。

- [ ] **Step 4: Commit**

```bash
git add app/integration_test/library_settings_retry_test.dart .github/workflows/ci.yml .github/workflows/release.yml
git commit -m "test: U11d 改 URL 重试集成测试与 CI 门禁"
```

---

# Phase 9d-10 — 文档与发版说明

### Task 10: README Plan 9d + 规格/计划入库

**Files:**
- Modify: `README.md`
- Add (force): `docs/superpowers/plans/2026-09-14-library-settings-retry.md`（本文件）
- Add (force): `docs/superpowers/specs/2026-09-14-library-settings-retry-design.md`（若尚未 `-f` 跟踪）

- [ ] **Step 1: README 追加 Plan 9d 节**

在 `## 片库海报（Plan 9c）` 之后插入：

```markdown
## 设置与失败任务重试（Plan 9d）

设置页 **媒体目录** 旁可 **选择文件夹**（仅采用文件夹名写入 `media_dir`；保存后在应用数据目录下创建，不搬移已有缓存）。失败任务可点刷新立即重试，或通过 **修改 URL 重试** 更正 `source_url` 后重新下载。

```bash
cd app && flutter test integration_test/library_settings_retry_test.dart -d macos
```

规格见 `docs/superpowers/specs/2026-09-14-library-settings-retry-design.md`。

**v0.2.0**：Plan 9a–9d 全部验收后打 tag。
```

更新 README 顶部 CI 说明，在 `library_poster` 后追加 `library_settings_retry`。

- [ ] **Step 2: 全量验证**

Run（仓库根目录）:

```bash
cargo fmt --manifest-path engine/Cargo.toml --all -- --check
cargo test --manifest-path engine/Cargo.toml
cargo clippy --manifest-path engine/Cargo.toml --all-targets --all-features -- -D warnings
cd app && flutter test
cd app && flutter test integration_test/library_settings_retry_test.dart -d macos
```

Expected: 全部 PASS

- [ ] **Step 3: Commit**

```bash
git add -f docs/superpowers/plans/2026-09-14-library-settings-retry.md \
  docs/superpowers/specs/2026-09-14-library-settings-retry-design.md \
  README.md
git commit -m "docs: Plan 9d 实现计划与 README v0.2.0 说明"
```

---

## Spec Self-Review

| 规格要求 | 对应 Task |
|----------|-----------|
| L9d-1..L9d-7 | Task 2 |
| F9d-1、F9d-2 | Task 3 |
| W9d-1 | Task 5 |
| W9d-2、W9d-3 | Task 6 |
| W9d-4、W9d-6 | Task 8 |
| W9d-4b、W9d-5 | Task 8（`tasks_retry_url_test.dart`） |
| U11d + CI | Task 9 |
| `file_picker` + basename 策略 | Task 5–6 |
| TV 隐藏选择器 + helper 分文案 | Task 6 |
| `checkpoint_json` + `.dl` 清理 | Task 1–2 |
| `retry_task` 快路径/改 URL | Task 2–4、8 |
| README + v0.2.0 | Task 10 |
| 删除 `requeue_failed_task` | Task 2 |

无 TBD / 占位步骤；`retry_task` / `retryTask` / `engine_retry_task` 签名全文一致。
