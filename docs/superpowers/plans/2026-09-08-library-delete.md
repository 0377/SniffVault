# Plan 9a 片库删除 Implementation Plan

> **修订**: 2026-09-08 review（路径校验策略、`active_cast`、L9-3/3b/6、TV Shortcuts、U11 完整步骤、发版 `v0.1.1`）

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 用户可在片库详情删除整条目或 Series 单分集；默认同时删除 `media_dir` 内已校验的本地缓存文件；Engine 为唯一写入口，含 FFI 与 Flutter UI。

**Architecture:** `LibraryStore` 增加 `remove_episode` / `count_episodes`；`engine/src/library/delete.rs` 实现「LAN 清理 →（`delete_files` 时）校验并删文件 → SQLite 事务删库」；`ActiveCastSession` 增 `episode_id` 供投送中删除清理；`Engine::remove_*` 为公开门面；FFI → `EngineHost`；Flutter `ConfirmDeleteDialog` + 详情页（含 TV `Shortcuts`）接线。

**Tech Stack:** Rust (`rusqlite`)、C FFI（Cargokit）、Flutter + Riverpod + go_router

**规格:** `docs/superpowers/specs/2026-09-08-library-management-design.md`（§2.1–2.2、§3.1、§4、§7.1–7.3 之 9a 部分）

## Global Constraints

- **Engine 是唯一片库写入口**：UI / FFI 不得直写 `LibraryStore`
- **`media_dir`**：相对 `data_dir` 的单层目录名；**仅 `delete_files=true`** 时对路径 `canonicalize` 并 `starts_with(media_dir)`
- **`delete_files=false`**：**跳过**路径校验与磁盘删除；允许移除 DB 中越界 `file_path` 脏数据
- **原子性**：`delete_files=true` 时先删磁盘文件，全部成功后再在 **单一 SQLite 事务** 内删库；任一步失败则 DB 不变
- **默认删文件**：确认对话框复选框默认 **勾选**「同时删除本地缓存文件」
- **删最后一集**：自动删除空 `library_items` 行（同一事务）
- **LAN**：经 `ensure_lan()` 若服务存在：对每个待删分集 `revoke_for_episode`；若 `active_cast.episode_id` 匹配则 `stop_cast_internal(false)`（不向 TV 发 stop HTTP）
- **9a 发版**：完成后打 `v0.1.1` tag；完整 `v0.2.0` 待 9b–9d
- **验证命令**（仓库根目录）：`cargo fmt --check`、`cargo test`、`cargo clippy -D warnings`；`cd app && flutter test`
- **`docs/` 在 `.gitignore`**：提交文档用 `git add -f docs/...`
- **提交信息中文**

---

## File Map

| 路径 | 职责 |
|------|------|
| `engine/src/library/store.rs` | `remove_episode`、`count_episodes` |
| `engine/src/library/delete.rs` | 路径收集、删文件、事务删库（`pub(crate)`） |
| `engine/src/library/mod.rs` | `mod delete;` |
| `engine/src/lan/stream_token.rs` | `revoke_for_episode` |
| `engine/src/lan/service.rs` | `ActiveCastSession.episode_id`、`finalize_episode_removal` |
| `engine/src/engine.rs` | `remove_library_item`、`remove_episode` 公开 API |
| `engine/tests/library_delete.rs` | L9-1..L9-6 |
| `engine/tests/support/library_dirty.rs` | `inject_outside_file_path` 测试辅助（L9-3/3b） |
| `engine/tests/library_store.rs` | store 层单测补充 |
| `engine/ffi/src/sync_dispatch.rs` | `engine_remove_library_item`、`engine_remove_episode` |
| `engine/ffi/src/lib.rs` | 导出新符号 |
| `engine/ffi/tests/library_delete_ffi_test.rs` | FFI 冒烟 |
| `app/lib/engine/native_bindings.dart` | Dart FFI typedef + lookup |
| `app/lib/engine/engine_host.dart` | `removeLibraryItem`、`removeEpisode` |
| `app/lib/providers/engine_repository.dart` | 抽象 + `EngineHostRepository` |
| `app/test/fakes/fake_engine_repository.dart` | Fake 实现 |
| `app/lib/features/library/widgets/confirm_delete_dialog.dart` | 确认对话框 |
| `app/lib/features/library/library_detail_screen.dart` | AppBar 删除菜单 + TV `Shortcuts` |
| `app/lib/features/library/widgets/episode_tile.dart` | 分集删除入口（Series ≥2 集） |
| `app/test/confirm_delete_dialog_test.dart` | W9-1、W9-2 |
| `app/test/library_delete_screen_test.dart` | W9-3、W9-4 |
| `app/integration_test/library_delete_test.dart` | U11 |
| `.github/workflows/ci.yml` | `flutter-integration` matrix 增 `library` suite |
| `README.md` | Plan 9a / U11 门禁说明 |

---

# Phase 9a — Engine 删除核心

### Task 1: LibraryStore `remove_episode` 与 `count_episodes`

**Files:**
- Modify: `engine/src/library/store.rs`
- Modify: `engine/tests/library_store.rs`

**Interfaces:**
- Produces: `LibraryStore::remove_episode(&self, episode_id: &str) -> Result<(), EngineError>`
- Produces: `LibraryStore::count_episodes(&self, item_id: &str) -> Result<u32, EngineError>`

- [ ] **Step 1: 写失败测试**

```rust
// engine/tests/library_store.rs — 追加

#[test]
fn remove_episode_deletes_one_row() {
    let dir = tempdir().unwrap();
    let store = LibraryStore::open(&dir.path().join("library.db")).unwrap();
    store
        .upsert_item(&LibraryItem {
            id: "s".into(),
            kind: LibraryItemKind::Series,
            title: "剧".into(),
            season: Some(1),
            poster_path: None,
            created_at_ms: 1,
        })
        .unwrap();
    store
        .upsert_episode(&LibraryEpisode {
            id: "e1".into(),
            item_id: "s".into(),
            index: 1,
            title: "第1集".into(),
            file_path: "/tmp/e1.mp4".into(),
            duration_ms: None,
            position_ms: 0,
            source_url: None,
        })
        .unwrap();
    store
        .upsert_episode(&LibraryEpisode {
            id: "e2".into(),
            item_id: "s".into(),
            index: 2,
            title: "第2集".into(),
            file_path: "/tmp/e2.mp4".into(),
            duration_ms: None,
            position_ms: 0,
            source_url: None,
        })
        .unwrap();

    store.remove_episode("e1").unwrap();
    assert_eq!(store.count_episodes("s").unwrap(), 1);
    assert!(store.get_episode("e1").unwrap().is_none());
    assert!(store.get_episode("e2").unwrap().is_some());
}

#[test]
fn remove_episode_missing_returns_not_found() {
    let dir = tempdir().unwrap();
    let store = LibraryStore::open(&dir.path().join("library.db")).unwrap();
    let err = store.remove_episode("nope").unwrap_err();
    assert!(err.to_string().contains("not found"));
}
```

- [ ] **Step 2: 运行测试确认失败**

```bash
cargo test --manifest-path engine/Cargo.toml remove_episode_deletes_one_row remove_episode_missing -- --nocapture
```

Expected: FAIL — `remove_episode` / `count_episodes` 未定义

- [ ] **Step 3: 实现 store 方法**

```rust
// engine/src/library/store.rs — 在 remove_item 之前追加

pub fn count_episodes(&self, item_id: &str) -> Result<u32, EngineError> {
    let count: i64 = self.conn.query_row(
        "SELECT COUNT(*) FROM library_episodes WHERE item_id=?1",
        params![item_id],
        |row| row.get(0),
    )?;
    Ok(count as u32)
}

pub fn remove_episode(&self, episode_id: &str) -> Result<(), EngineError> {
    let n = self
        .conn
        .execute("DELETE FROM library_episodes WHERE id=?1", params![episode_id])?;
    if n == 0 {
        return Err(EngineError::NotFound(format!("episode {episode_id}")));
    }
    Ok(())
}
```

- [ ] **Step 4: 运行测试确认通过**

```bash
cargo test --manifest-path engine/Cargo.toml library_store -- --nocapture
```

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add engine/src/library/store.rs engine/tests/library_store.rs
git commit -m "feat(engine): LibraryStore 支持按分集删除与计数"
```

---

### Task 2: LAN 投送清理（`revoke_for_episode` + `active_cast`）

**Files:**
- Modify: `engine/src/lan/stream_token.rs`
- Modify: `engine/src/lan/service.rs`
- Create: `engine/tests/lan_stream_token_revoke_test.rs`

**Interfaces:**
- Produces: `StreamTokenStore::revoke_for_episode(&mut self, episode_id: &str)`
- Produces: `LanService::finalize_episode_removal(&mut self, episode_id: &str) -> Result<(), EngineError>` — 撤销 token；若 `active_cast.episode_id == episode_id` 则 `stop_cast_internal(false)`
- Modifies: `ActiveCastSession` 增加 `episode_id: String`（`cast_episode` 写入）

- [ ] **Step 1: 写失败测试（token）**

```rust
// engine/tests/lan_stream_token_revoke_test.rs
use video_sniffing_engine::lan::StreamTokenStore;

#[test]
fn revoke_for_episode_invalidates_token() {
    let mut store = StreamTokenStore::new();
    store.register("tok-a", "ep-1", 0);
    store.revoke_for_episode("ep-1");
    let new_tok = store.issue("ep-1", 2);
    assert_ne!(new_tok, "tok-a");
}
```

- [ ] **Step 2: 实现 `revoke_for_episode`**

```rust
// engine/src/lan/stream_token.rs
pub fn revoke_for_episode(&mut self, episode_id: &str) {
    if let Some(token) = self.by_episode.remove(episode_id) {
        self.by_token.remove(&token);
    }
}
```

- [ ] **Step 3: 扩展 `ActiveCastSession` 并在 `cast_episode` 写入 `episode_id`**

```rust
// engine/src/lan/service.rs
struct ActiveCastSession {
    session_id: String,
    stream_token: String,
    episode_id: String,  // 新增
    peer_device_id: String,
    peer_host: String,
    peer_port: u16,
}

// cast_episode 内 self.active_cast = Some(ActiveCastSession { ..., episode_id: episode_id.to_string(), ... });
```

- [ ] **Step 4: 实现 `finalize_episode_removal`**

```rust
pub fn finalize_episode_removal(&mut self, episode_id: &str) -> Result<(), EngineError> {
    {
        let mut store = self.stream_tokens.lock().map_err(|_| lock_err())?;
        store.revoke_for_episode(episode_id);
    }
    if self.active_cast.as_ref().is_some_and(|s| s.episode_id == episode_id) {
        self.stop_cast_internal(false)?;
    }
    Ok(())
}
```

- [ ] **Step 5: 运行测试**

```bash
cargo test --manifest-path engine/Cargo.toml revoke_for_episode lan_stream_token -- --nocapture
cargo test --manifest-path engine/Cargo.toml lan_pair_and_cast -- --nocapture
```

Expected: PASS（`cast_episode` 写入 `episode_id` 不得破坏既有 LAN 集成测）

- [ ] **Step 6: Commit**

```bash
git add engine/src/lan/stream_token.rs engine/src/lan/service.rs engine/tests/lan_stream_token_revoke_test.rs
git commit -m "feat(engine): 分集删除时撤销 LAN token 并清理 active_cast"
```

---

### Task 3: `library/delete.rs` 与 `Engine::remove_*`

**Files:**
- Create: `engine/src/library/delete.rs`
- Modify: `engine/src/library/mod.rs`
- Modify: `engine/src/engine.rs`
- Create: `engine/tests/library_delete.rs`

**Interfaces:**
- Consumes: Task 1 store 方法
- Consumes: Task 2 `LanService::finalize_episode_removal`
- Consumes: `ingest::ensure_path_in_media_dir`
- Produces: `Engine::remove_library_item(&mut self, item_id: &str, delete_files: bool) -> Result<(), EngineError>`
- Produces: `Engine::remove_episode(&mut self, episode_id: &str, delete_files: bool) -> Result<(), EngineError>`
- Produces: `LibraryStore::remove_item_in_tx`（`pub(crate)`，供 `delete.rs` 事务删库）

- [ ] **Step 1: 写失败测试 L9-1（删 DB + 文件）**

```rust
// engine/tests/library_delete.rs
use std::fs;
use tempfile::tempdir;
use video_sniffing_engine::Engine;

fn seed_single(engine: &mut Engine, file_name: &str) -> (String, String) {
    let media = engine.media_dir().join(file_name);
    fs::write(&media, b"video-bytes").unwrap();
    let (item, ep) = engine
        .register_completed_single("测试片", media.to_str().unwrap(), None)
        .unwrap();
    (item.id, ep.id)
}

#[test]
fn remove_library_item_deletes_db_and_file() {
    let dir = tempdir().unwrap();
    let mut engine = Engine::open(dir.path()).unwrap();
    let (item_id, _) = seed_single(&mut engine, "clip.mp4");
    let media_path = engine.media_dir().join("clip.mp4");
    assert!(media_path.exists());

    engine.remove_library_item(&item_id, true).unwrap();
    assert!(engine.list_library().unwrap().is_empty());
    assert!(!media_path.exists());
}
```

- [ ] **Step 2: 运行确认失败**

```bash
cargo test --manifest-path engine/Cargo.toml remove_library_item_deletes_db_and_file -- --nocapture
```

Expected: FAIL — `remove_library_item` 未定义

- [ ] **Step 3: 实现 `library/delete.rs` 与 `Engine` 方法**

```rust
// engine/src/library/delete.rs
pub(crate) fn collect_deletion_paths(
    library: &LibraryStore,
    item: &LibraryItem,
    media_dir: &Path,
) -> Result<Vec<PathBuf>, EngineError> {
    let mut paths = Vec::new();
    for ep in library.list_episodes(&item.id)? {
        paths.push(ingest::ensure_path_in_media_dir(media_dir, &ep.file_path)?);
    }
    if let Some(poster) = &item.poster_path {
        paths.push(ingest::ensure_path_in_media_dir(media_dir, poster)?);
    }
    Ok(paths)
}

pub(crate) fn delete_files(paths: &[PathBuf]) -> Result<(), EngineError> {
    for path in paths {
        match std::fs::remove_file(path) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(EngineError::Io(e)),
        }
    }
    Ok(())
}

pub(crate) fn remove_item_record(library: &LibraryStore, item_id: &str) -> Result<(), EngineError> {
    library.get_item(item_id)?;
    let tx = library.conn().unchecked_transaction()?;
    library.remove_item_in_tx(&tx, item_id)?;
    tx.commit()?;
    Ok(())
}
```

在 `LibraryStore` 增加：

```rust
pub(crate) fn conn(&self) -> &Connection { &self.conn }

pub(crate) fn remove_item_in_tx(
    &self,
    tx: &rusqlite::Transaction,
    item_id: &str,
) -> Result<(), EngineError> {
    let n = tx.execute("DELETE FROM library_items WHERE id=?1", params![item_id])?;
    if n == 0 {
        return Err(EngineError::NotFound(format!("library item {item_id}")));
    }
    Ok(())
}
```

**`Engine` 方法：**

```rust
fn finalize_lan_for_episodes(&mut self, episode_ids: &[String]) -> Result<(), EngineError> {
    if self.lan.is_none() {
        return Ok(());
    }
    let lan = self.ensure_lan()?;
    for id in episode_ids {
        lan.finalize_episode_removal(id)?;
    }
    Ok(())
}

pub fn remove_library_item(&mut self, item_id: &str, delete_files: bool) -> Result<(), EngineError> {
    let item = self.library.get_item(item_id)?;
    let episodes = self.library.list_episodes(item_id)?;
    let episode_ids: Vec<String> = episodes.iter().map(|e| e.id.clone()).collect();
    self.finalize_lan_for_episodes(&episode_ids)?;
    if delete_files {
        let paths = crate::library::delete::collect_deletion_paths(
            &self.library, &item, &self.media_dir(),
        )?;
        crate::library::delete::delete_files(&paths)?;
    }
    crate::library::delete::remove_item_record(&self.library, item_id)?;
    Ok(())
}

pub fn remove_episode(&mut self, episode_id: &str, delete_files: bool) -> Result<(), EngineError> {
    let ep = self.library.get_episode(episode_id)?
        .ok_or_else(|| EngineError::NotFound(format!("episode {episode_id}")))?;
    if self.library.count_episodes(&ep.item_id)? == 1 {
        return self.remove_library_item(&ep.item_id, delete_files);
    }
    self.finalize_lan_for_episodes(&[episode_id.to_string()])?;
    if delete_files {
        let path = ingest::ensure_path_in_media_dir(&self.media_dir(), &ep.file_path)?;
        crate::library::delete::delete_files(&[path])?;
    }
    self.library.remove_episode(episode_id)?;
    Ok(())
}
```

- [ ] **Step 4: 追加 L9-2..L9-5、L9-3、L9-3b 测试**

```rust
#[test]
fn remove_library_item_keep_files_on_disk() { /* 同前 L9-2 */ }

#[test]
fn remove_rejects_outside_path_when_delete_files() {
    let dir = tempdir().unwrap();
    let mut engine = Engine::open(dir.path()).unwrap();
    let media = engine.media_dir().join("good.mp4");
    std::fs::write(&media, b"x").unwrap();
    let (item, _) = engine
        .register_completed_single("片", media.to_str().unwrap(), None)
        .unwrap();
    // 经 LibraryStore 注入脏路径（register_completed 无法做到）
    let outside = dir.path().join("outside.mp4");
    std::fs::write(&outside, b"x").unwrap();
    // 测试辅助：在 engine/tests/support/ 或直接打开 library.db 用 LibraryStore::upsert_episode 覆盖 file_path
    inject_outside_file_path(&engine, &item.id, outside.to_str().unwrap());

    let err = engine.remove_library_item(&item.id, true).unwrap_err();
    assert!(err.to_string().contains("media_dir") || err.to_string().contains("media"));
    assert_eq!(engine.list_library().unwrap().len(), 1);
}

#[test]
fn remove_allows_dirty_path_when_keep_files() {
    // 同上注入越界 path，delete_files=false → list_library 为空，outside.mp4 仍在
}

#[test]
fn remove_last_episode_removes_item() { /* 同前 L9-4 */ }

#[test]
fn remove_missing_file_still_commits_db() { /* 同前 L9-5 */ }
```

> **`inject_outside_file_path` 实现提示**：`Engine` 无公开 store 访问；在 `engine/tests/support/library_dirty.rs` 用 `LibraryStore::open(data_dir/library.db)` + `get_episode` + `upsert_episode` 写回越界 `file_path`（保留原 `id`/`item_id`/`index`）。

```bash
cargo test --manifest-path engine/Cargo.toml library_delete -- --nocapture
```

Expected: 7 tests PASS

- [ ] **Step 5: L9-6（Engine 层，loopback 投送后删除）**

```rust
#[test]
fn remove_episode_after_cast_allows_new_cast() {
    use video_sniffing_engine::lan::LanTestConfig;
    let sender_dir = tempfile::tempdir().unwrap();
    let receiver_dir = tempfile::tempdir().unwrap();
    let mut receiver = Engine::open(receiver_dir.path()).unwrap();
    let mut sender = Engine::open(sender_dir.path()).unwrap();
    for e in [&mut sender, &mut receiver] {
        let mut s = e.settings();
        s.lan_enabled = true;
        e.save_settings(s).unwrap();
        e.set_lan_test_config(LanTestConfig { advertise_ip: Some("127.0.0.1".into()) });
    }
    receiver.apply_lan_settings(true).unwrap();
    let pin = receiver.begin_pairing().unwrap();
    let port = receiver.lan_http_port().unwrap();
    sender.apply_lan_settings(false).unwrap();
    sender.pair_peer("127.0.0.1", port, &pin).unwrap();

    let f1 = sender.media_dir().join("ep1.mp4");
    std::fs::write(&f1, b"x").unwrap();
    let (_, ep1) = sender.register_completed_single("片1", f1.to_str().unwrap(), None).unwrap();
    sender.cast_episode(&ep1.id, &receiver.settings().device_id).unwrap();

    sender.remove_episode(&ep1.id, true).unwrap();
    assert!(!sender.has_active_cast()); // 需在 LanService/Engine 加 #[cfg(test)] 查询钩

    let f2 = sender.media_dir().join("ep2.mp4");
    std::fs::write(&f2, b"x").unwrap();
    let (_, ep2) = sender.register_completed_single("片2", f2.to_str().unwrap(), None).unwrap();
    sender.cast_episode(&ep2.id, &receiver.settings().device_id).unwrap();

    sender.stop_cast().unwrap();
    sender.stop_lan().unwrap();
    receiver.stop_lan().unwrap();
}
```

> `has_active_cast()`：`Engine` 上 `#[cfg(test)] pub fn has_active_cast(&self) -> bool` 委托 `LanService`。

- [ ] **Step 6: Commit**

```bash
git add engine/src/library/delete.rs engine/src/library/mod.rs engine/src/library/store.rs engine/src/engine.rs engine/tests/library_delete.rs
git commit -m "feat(engine): 实现片库条目与分集删除"
```

---

### Task 4: FFI `engine_remove_library_item` / `engine_remove_episode`

**Files:**
- Modify: `engine/ffi/src/sync_dispatch.rs`
- Modify: `engine/ffi/src/lib.rs`
- Create: `engine/ffi/tests/library_delete_ffi_test.rs`

**Interfaces:**
- Consumes: `Engine::remove_library_item`、`Engine::remove_episode`
- Produces: `engine_remove_library_item(handle, item_id, delete_files: u8) -> *mut c_char`
- Produces: `engine_remove_episode(handle, episode_id, delete_files: u8) -> *mut c_char`

- [ ] **Step 1: 写失败 FFI 测试**

```rust
// engine/ffi/tests/library_delete_ffi_test.rs
use std::ffi::{CStr, CString};
use std::fs;
use tempfile::tempdir;
use video_sniffing_engine::Engine;
use video_sniffing_engine_ffi::handle::{engine_destroy, engine_free_string, engine_open};
use video_sniffing_engine_ffi::sync_dispatch::{
    engine_list_library, engine_remove_library_item,
};

#[test]
fn ffi_remove_library_item_ok_json() {
    let dir = tempdir().unwrap();
    let mut engine = Engine::open(dir.path()).unwrap();
    let media = engine.media_dir().join("f.mp4");
    fs::write(&media, b"x").unwrap();
    let (item, _) = engine
        .register_completed_single("片", media.to_str().unwrap(), None)
        .unwrap();
    drop(engine);

    let path = CString::new(dir.path().to_str().unwrap()).unwrap();
    let handle = unsafe { engine_open(path.as_ptr()) };
    let item_id = CString::new(item.id).unwrap();
    let ptr = unsafe { engine_remove_library_item(handle, item_id.as_ptr(), 1) };
    assert!(!ptr.is_null());
    let json = unsafe { CStr::from_ptr(ptr).to_str().unwrap() };
    let v: serde_json::Value = serde_json::from_str(json).unwrap();
    assert_eq!(v["ok"], true);
    unsafe { engine_free_string(ptr) };

    let lib_ptr = unsafe { engine_list_library(handle) };
    let lib_json = unsafe { CStr::from_ptr(lib_ptr).to_str().unwrap() };
    let lib: serde_json::Value = serde_json::from_str(lib_json).unwrap();
    assert_eq!(lib["data"].as_array().unwrap().len(), 0);
    unsafe { engine_free_string(lib_ptr) };
    unsafe { engine_destroy(handle) };
}
```

- [ ] **Step 2: 实现 FFI（遵循 `engine_set_episode_position` 模式）**

```rust
#[no_mangle]
pub unsafe extern "C" fn engine_remove_library_item(
    handle: *mut EngineHandle,
    item_id: *const c_char,
    delete_files: u8,
) -> *mut c_char {
    let item_id = match parse_c_str(item_id, "item_id") {
        Ok(id) => id,
        Err(err) => return rust_to_c_string(err_json(err)),
    };
    let delete_files = delete_files != 0;
    ffi_call_mut(handle, |engine| {
        engine
            .remove_library_item(&item_id, delete_files)
            .map(|_| ())
    })
}

#[no_mangle]
pub unsafe extern "C" fn engine_remove_episode(
    handle: *mut EngineHandle,
    episode_id: *const c_char,
    delete_files: u8,
) -> *mut c_char {
    let episode_id = match parse_c_str(episode_id, "episode_id") {
        Ok(id) => id,
        Err(err) => return rust_to_c_string(err_json(err)),
    };
    let delete_files = delete_files != 0;
    ffi_call_mut(handle, |engine| {
        engine
            .remove_episode(&episode_id, delete_files)
            .map(|_| ())
    })
}
```

- [ ] **Step 3: 在 `lib.rs` re-export 符号**

- [ ] **Step 4: 运行**

```bash
cargo test --manifest-path engine/Cargo.toml library_delete_ffi -- --nocapture
cargo clippy --manifest-path engine/Cargo.toml --all-targets --all-features -- -D warnings
```

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add engine/ffi/src/sync_dispatch.rs engine/ffi/src/lib.rs engine/ffi/tests/library_delete_ffi_test.rs
git commit -m "feat(ffi): 暴露片库删除 API"
```

---

# Phase 9a — Flutter 桥接与 UI

### Task 5: EngineHost + Repository + Fake

**Files:**
- Modify: `app/lib/engine/native_bindings.dart`
- Modify: `app/lib/engine/engine_host.dart`
- Modify: `app/lib/providers/engine_repository.dart`
- Modify: `app/test/fakes/fake_engine_repository.dart`

**Interfaces:**
- Consumes: FFI `engine_remove_library_item`、`engine_remove_episode`
- Produces: `EngineRepository.removeLibraryItem(String itemId, {bool deleteFiles})`
- Produces: `EngineRepository.removeEpisode(String episodeId, {bool deleteFiles})`

- [ ] **Step 1: 在 `native_bindings.dart` 增加 typedef 与 lookup**

```dart
typedef EngineRemoveLibraryItemNative = Pointer<Char> Function(
  Pointer<Void> handle,
  Pointer<Utf8> itemId,
  Uint8 deleteFiles,
);
// ... engineRemoveLibraryItem 字段与构造函数赋值
```

- [ ] **Step 2: `EngineHost` 方法**

```dart
void removeLibraryItem(String itemId, {bool deleteFiles = true}) {
  _invokeMut(() {
    final idPtr = itemId.toNativeUtf8();
    try {
      return _bindings.engineRemoveLibraryItem(
        _handle,
        idPtr,
        deleteFiles ? 1 : 0,
      );
    } finally {
      malloc.free(idPtr);
    }
  });
}

void removeEpisode(String episodeId, {bool deleteFiles = true}) {
  // 同上，调用 engineRemoveEpisode
}
```

- [ ] **Step 3: `EngineRepository` 抽象 + `EngineHostRepository` 委托**

- [ ] **Step 4: `FakeEngineRepository` 实现（供 widget 测）**

```dart
bool? lastDeleteFiles;
String? lastRemovedItemId;
String? lastRemovedEpisodeId;

@override
void removeLibraryItem(String itemId, {bool deleteFiles = true}) {
  lastDeleteFiles = deleteFiles;
  lastRemovedItemId = itemId;
  libraryItems = libraryItems.where((i) => i.id != itemId).toList();
  episodesByItemId.remove(itemId);
}

@override
void removeEpisode(String episodeId, {bool deleteFiles = true}) {
  lastDeleteFiles = deleteFiles;
  lastRemovedEpisodeId = episodeId;
  for (final entry in episodesByItemId.entries.toList()) {
    final next = entry.value.where((e) => e.id != episodeId).toList();
    if (next.length != entry.value.length) {
      episodesByItemId[entry.key] = next;
      if (next.isEmpty) {
        libraryItems = libraryItems.where((i) => i.id != entry.key).toList();
        episodesByItemId.remove(entry.key);
      }
      break;
    }
  }
}
```

- [ ] **Step 5: 运行**

```bash
cd app && flutter analyze lib/engine/ lib/providers/engine_repository.dart
```

Expected: No issues

- [ ] **Step 6: Commit**

```bash
git add app/lib/engine/native_bindings.dart app/lib/engine/engine_host.dart app/lib/providers/engine_repository.dart app/test/fakes/fake_engine_repository.dart
git commit -m "feat(app): EngineHost 片库删除 FFI 桥接"
```

---

### Task 6: `ConfirmDeleteDialog` 组件（W9-1、W9-2）

**Files:**
- Create: `app/lib/features/library/widgets/confirm_delete_dialog.dart`
- Create: `app/test/confirm_delete_dialog_test.dart`

**Interfaces:**
- Produces: `Future<({bool confirmed, bool deleteFiles})?> showConfirmDeleteDialogResult(BuildContext context, {required String title, required String message})`

- [ ] **Step 1: 写失败测试 W9-1**

```dart
// app/test/confirm_delete_dialog_test.dart
testWidgets('W9 delete dialog defaults deleteFiles to true', (tester) async {
  ({bool confirmed, bool deleteFiles})? result;
  await tester.pumpWidget(
    MaterialApp(
      home: Builder(
        builder: (context) => ElevatedButton(
          onPressed: () async {
            result = await showConfirmDeleteDialogResult(
              context,
              title: '删除「测试」？',
              message: '将删除 1 个文件',
            );
          },
          child: const Text('open'),
        ),
      ),
    ),
  );
  await tester.tap(find.text('open'));
  await tester.pumpAndSettle();
  expect(find.byType(Checkbox), findsOneWidget);
  expect(tester.widget<Checkbox>(find.byType(Checkbox)).value, isTrue);
  await tester.tap(find.text('删除'));
  await tester.pumpAndSettle();
  expect(result?.confirmed, isTrue);
  expect(result?.deleteFiles, isTrue);
});
```

- [ ] **Step 2: 实现对话框**

```dart
// app/lib/features/library/widgets/confirm_delete_dialog.dart
class ConfirmDeleteDialog extends StatefulWidget {
  const ConfirmDeleteDialog({
    super.key,
    required this.title,
    required this.message,
    this.deleteFilesDefault = true,
  });

  final String title;
  final String message;
  final bool deleteFilesDefault;

  @override
  State<ConfirmDeleteDialog> createState() => _ConfirmDeleteDialogState();
}

class _ConfirmDeleteDialogState extends State<ConfirmDeleteDialog> {
  late bool _deleteFiles;

  @override
  void initState() {
    super.initState();
    _deleteFiles = widget.deleteFilesDefault;
  }

  @override
  Widget build(BuildContext context) {
    return AlertDialog(
      title: Text(widget.title),
      content: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(widget.message),
          CheckboxListTile(
            value: _deleteFiles,
            onChanged: (v) => setState(() => _deleteFiles = v ?? true),
            title: const Text('同时删除本地缓存文件'),
            controlAffinity: ListTileControlAffinity.leading,
            contentPadding: EdgeInsets.zero,
          ),
        ],
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.pop(context),
          child: const Text('取消'),
        ),
        FilledButton(
          onPressed: () => Navigator.pop(context, (confirmed: true, deleteFiles: _deleteFiles)),
          child: const Text('删除'),
        ),
      ],
    );
  }
}

Future<({bool confirmed, bool deleteFiles})?> showConfirmDeleteDialogResult(
  BuildContext context, {
  required String title,
  required String message,
  bool deleteFilesDefault = true,
}) {
  return showDialog<({bool confirmed, bool deleteFiles})>(
    context: context,
    builder: (_) => ConfirmDeleteDialog(
      title: title,
      message: message,
      deleteFilesDefault: deleteFilesDefault,
    ),
  );
}
```

- [ ] **Step 3: W9-2 取消不改变状态**

```dart
testWidgets('W9 cancel returns null', (tester) async {
  ({bool confirmed, bool deleteFiles})? result;
  await tester.pumpWidget(
    MaterialApp(
      home: Builder(
        builder: (context) => ElevatedButton(
          onPressed: () async {
            result = await showConfirmDeleteDialogResult(
              context,
              title: '删除「测试」？',
              message: '将删除 1 个文件',
            );
          },
          child: const Text('open'),
        ),
      ),
    ),
  );
  await tester.tap(find.text('open'));
  await tester.pumpAndSettle();
  await tester.tap(find.text('取消'));
  await tester.pumpAndSettle();
  expect(result, isNull);
});
```

- [ ] **Step 4: 运行**

```bash
cd app && flutter test test/confirm_delete_dialog_test.dart
```

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add app/lib/features/library/widgets/confirm_delete_dialog.dart app/test/confirm_delete_dialog_test.dart
git commit -m "feat(app): 片库删除确认对话框"
```

---

### Task 7: 详情页与分集删除 UI

**Files:**
- Modify: `app/lib/features/library/library_detail_screen.dart`
- Modify: `app/lib/features/library/widgets/episode_tile.dart`
- Create: `app/test/library_delete_screen_test.dart`

**Interfaces:**
- Consumes: `showConfirmDeleteDialogResult`、`EngineRepository.removeLibraryItem` / `removeEpisode`、`libraryProvider`

- [ ] **Step 1: W9-3 / W9-4 失败测试**

```dart
// app/test/library_delete_screen_test.dart — 复用 cast_entry_test 的 ProviderScope 模式
testWidgets('W9 delete item calls repo and invalidates library', (tester) async { /* 删整条目 */ });

testWidgets('W9 delete episode calls removeEpisode', (tester) async {
  // Series 2 集；点第二集 PopupMenu → 删除此分集 → 确认
  // expect fake.lastRemovedEpisodeId、episodesByItemId 剩 1 条
});
```

- [ ] **Step 2: `LibraryDetailScreen` AppBar 菜单 + TV `Shortcuts`**

```dart
return Shortcuts(
  shortcuts: const {
    LogicalKeyboardKey.contextMenu: const ActivateIntent(),
  },
  child: Actions(
    actions: {
      ActivateIntent: CallbackAction<ActivateIntent>(
        onInvoke: (_) {
          _showDeleteMenu(context); // 打开与 PopupMenuButton 相同的删除确认
          return null;
        },
      ),
    },
    child: Focus(
      autofocus: true,
      child: Scaffold(
        appBar: AppBar(
          title: Text(item?.title ?? '片库详情'),
          actions: [
            if (item != null)
              PopupMenuButton<String>(
                key: const Key('library_detail_menu'),
                onSelected: (value) => _onMenuSelected(context, ref, value, item!, episodes),
                itemBuilder: (_) => const [
                  PopupMenuItem(value: 'delete', child: Text('删除')),
                ],
              ),
          ],
        ),
        body: _buildBody(...),
      ),
    ),
  ),
);
```

`_onMenuSelected` 内删除分支使用 `showConfirmDeleteDialogResult`（逻辑同前）。

- [ ] **Step 3: `EpisodeTile` 增加 `onDelete`（仅 Series 且 `episodes.length >= 2` 时传入）**

```dart
// episode_tile.dart — onDelete != null 时 trailing 含 PopupMenuButton「删除此分集」
```

- [ ] **Step 4: 运行 widget 测试**

```bash
cd app && flutter test test/library_delete_screen_test.dart test/confirm_delete_dialog_test.dart
```

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add app/lib/features/library/library_detail_screen.dart app/lib/features/library/widgets/episode_tile.dart app/test/library_delete_screen_test.dart
git commit -m "feat(app): 片库详情删除入口与分集菜单"
```

---

### Task 8: U11 集成测试、CI 与文档

**Files:**
- Create: `app/integration_test/library_delete_test.dart`
- Modify: `.github/workflows/ci.yml`
- Modify: `README.md`

**Interfaces:**
- Consumes: 完整 Engine + `EngineHost` 删除 API；复用 `integration_test/support/app_ui_flow.dart` 的 fixture 下载模式

- [x] **Step 1: 写 U11 集成测试**

```dart
// app/integration_test/library_delete_test.dart
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';
import 'package:media_kit/media_kit.dart';
import 'package:video_sniffing/app.dart';

import 'support/app_ui_flow.dart';
import 'support/test_pump.dart';

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();
  MediaKit.ensureInitialized();

  testWidgets('U11 library delete removes db entry and file', (tester) async {
    await runAppUiSmokeFlow(
      tester,
      stopBeforePlay: true,
      onLibraryReady: (tester, mediaPath) async {
        await tester.tap(find.byKey(const Key('library_list')).first);
        await pumpUntil(tester, () => find.byKey(const Key('library_detail_menu')).evaluate().isNotEmpty);

        await tester.tap(find.byKey(const Key('library_detail_menu')));
        await pumpUntil(tester, () => find.text('删除').evaluate().isNotEmpty);
        await tester.tap(find.text('删除'));
        await pumpUntil(tester, () => find.text('同时删除本地缓存文件').evaluate().isNotEmpty);
        await tester.tap(find.text('删除').last);
        await pumpUntil(tester, () => find.text('片库为空').evaluate().isNotEmpty);

        expect(File(mediaPath).existsSync(), isFalse);
      },
    );
  }, timeout: const Timeout(Duration(minutes: 5)));
}
```

> 若 `app_ui_flow.dart` 无 `stopBeforePlay` / `onLibraryReady` 钩，在 Task 8 先增最小扩展（下载完成 → 返回片库路径 + `mediaPath`），勿复制整套 smoke 逻辑。

- [x] **Step 2: CI matrix 增加 `library` suite**

```yaml
matrix:
  suite: [engine, ui, deeplink, cast, browse, library]
# ...
elif [ "${{ matrix.suite }}" = "library" ]; then
  flutter test integration_test/library_delete_test.dart -d macos
```

- [x] **Step 3: README 追加 Plan 9a 小节**

```markdown
## 片库删除（Plan 9a）

片库详情 → ⋮ → 删除；Series（≥2 集）可对单分集删除。默认同时删除本地缓存文件。9a 发版 tag：`v0.1.1`。

\`\`\`bash
cd app && flutter test integration_test/library_delete_test.dart -d macos
\`\`\`
```

- [x] **Step 4: 全量验证**

```bash
cargo fmt --manifest-path engine/Cargo.toml --all -- --check
cargo test --manifest-path engine/Cargo.toml
cargo clippy --manifest-path engine/Cargo.toml --all-targets --all-features -- -D warnings
cd app && flutter test
cd app && flutter test integration_test/library_delete_test.dart -d macos
```

Expected: 全部 PASS

- [x] **Step 5: Commit**

```bash
git add -f docs/superpowers/plans/2026-09-08-library-delete.md docs/superpowers/specs/2026-09-08-library-management-design.md
git add app/integration_test/library_delete_test.dart .github/workflows/ci.yml README.md
git commit -m "test(app): U11 片库删除集成测与 CI 门禁"
```

---

## Spec Self-Review（计划自检）

| 规格章节 | 对应 Task |
|----------|-----------|
| §2.1 删除整条目 UI + TV Shortcuts | Task 7 |
| §2.2 删除单分集（Series ≥2） | Task 7 |
| §3.1 Engine API + `active_cast` | Task 2–3 |
| §3.1 `delete_files=false` 跳过校验 | Task 3 L9-3b |
| §4 FFI | Task 4 |
| §5.1 确认框默认删文件 | Task 6 |
| §6 错误处理 / 越界路径 | Task 3 L9-3 / L9-3b |
| L9-1..L9-6 | Task 1–3 |
| W9-1..W9-4 | Task 6–7 |
| U11 | Task 8 |

**Placeholder 扫描:** U11 依赖 `app_ui_flow` 小扩展；L9-3 依赖 `inject_outside_file_path` 测试辅助；L9-6 依赖 `has_active_cast` 测试钩 — 均在 Task 3/8 写明。

**类型一致性:** `delete_files` / `deleteFiles`；FFI `u8`；`showConfirmDeleteDialogResult` 返回 record。

**发版:** 9a → `v0.1.1`；完整 v0.2.0 待 9b–9d。

---

## Execution Handoff

**Plan complete and saved to `docs/superpowers/plans/2026-09-08-library-delete.md`. Two execution options:**

**1. Subagent-Driven (recommended)** — 每个 Task 派发独立子 agent，任务间做代码审查，迭代快

**2. Inline Execution** — 本会话按 Task 1→8 顺序执行，检查点处暂停供你审阅

**Which approach?**
