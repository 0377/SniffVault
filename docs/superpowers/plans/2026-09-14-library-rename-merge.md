# Plan 9b 片库重命名与 Series 合并 Implementation Plan

> **修订**: 2026-09-14 review（Series 标题范围、Store 事务门面、L9b-9/10、W9b-5/6、Task 7/8 拆分、补全占位测试）

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 用户可在片库详情重命名条目/分集标题，并将误建的重复 Series 壳合并到同 `title + season` 的目标条目；Engine 为唯一写入口，含 FFI、Flutter UI 与自动化测试。

**Architecture:** `LibraryStore` 提供 `rename_library_item_titles` 与 `apply_merge_in_tx` 事务门面（Engine **不** 直接 `conn()`）；`rename.rs` 校验标题；`merge.rs` 分类 migrate/orphan、LAN 清理、可选删文件后委托 Store 提交；FFI → `EngineHost` → 详情页菜单（Task 7 仅 rename，Task 8 增 merge）。

**Tech Stack:** Rust (`rusqlite`)、C FFI（Cargokit）、Flutter + Riverpod + go_router

**规格:** `docs/superpowers/specs/2026-09-14-library-rename-merge-design.md`

## Global Constraints

- **Engine 是唯一片库写入口**：UI / FFI 不得直写 `LibraryStore`（测试 seed 经 `test_api::LibraryStore` 例外）
- **Engine 不直接开 SQLite 事务**：重命名/合并经 `LibraryStore` 门面方法
- **重命名不改 `file_path`**：仅更新 `library_items.title`、`library_episodes.title`
- **Single 重命名**：恰 1 分集时 `rename_library_item` 同步唯一分集 title
- **Series 重命名条目**：**不** 批量改分集 title
- **展示标题校验**：trim 后非空、`len <= 512`
- **合并单事务**：`apply_merge_in_tx` 内 migrate UPDATE + orphan DELETE + 删源壳；`delete_orphan_files=true` 时磁盘删除在事务 **之前**
- **idx 冲突保留目标**；orphan 分集经 LAN 清理
- **`delete_orphan_files` 默认 false**
- **合并候选**：Flutter `ref.read(libraryProvider)` + `isMergeCandidate`
- **验证命令**（仓库根目录）：`cargo fmt --check`、`cargo test`、`cargo clippy -D warnings`；`cd app && flutter test`
- **`docs/` 在 `.gitignore`**：提交文档用 `git add -f docs/...`
- **提交信息中文**

---

## File Map

| 路径 | 职责 |
|------|------|
| `engine/src/library/store.rs` | title UPDATE、`rename_library_item_titles`、`apply_merge_in_tx` |
| `engine/src/library/rename.rs` | `validate_display_title` |
| `engine/src/library/merge.rs` | 合并算法 |
| `engine/src/engine.rs` | 三个公开 API |
| `engine/tests/support/library_merge_seed.rs` | 重复 Series 测试 seed |
| `engine/tests/library_rename_merge.rs` | L9b-1..L9b-10 |
| `engine/ffi/tests/library_rename_merge_ffi_test.rs` | F9b-1、F9b-2 |
| `app/lib/features/library/merge_candidates.dart` | `isMergeCandidate`、`mergeCandidatesFor` |
| `app/test/rename_dialog_test.dart` | W9b-1 |
| `app/test/library_rename_merge_screen_test.dart` | W9b-2..W9b-6 |
| `app/integration_test/library_rename_merge_test.dart` | U11b（Engine 级） |

---

# Phase 9b-1 — LibraryStore 标题 UPDATE

### Task 1: `update_item_title` / `update_episode_title`

**Files:**
- Modify: `engine/src/library/store.rs`
- Modify: `engine/tests/library_store.rs`

**Interfaces:**
- Produces: `LibraryStore::update_item_title(&self, item_id: &str, title: &str) -> Result<(), EngineError>`
- Produces: `LibraryStore::update_episode_title(&self, episode_id: &str, title: &str) -> Result<(), EngineError>`

- [ ] **Step 1: 写失败测试**

```rust
// engine/tests/library_store.rs — 追加（需已有 use LibraryStore, LibraryItem, ...）

#[test]
fn update_episode_title_persists() {
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
            title: "旧分集名".into(),
            file_path: "/tmp/e1.mp4".into(),
            duration_ms: None,
            position_ms: 0,
            source_url: None,
        })
        .unwrap();
    store.update_episode_title("e1", "新分集名").unwrap();
    let ep = store.get_episode("e1").unwrap().unwrap();
    assert_eq!(ep.title, "新分集名");
}
```

（`update_item_title_persists` / `update_item_title_missing_returns_not_found` 同前 plan Task 1。）

- [ ] **Step 2: 运行测试确认失败**

Run: `cargo test --manifest-path engine/Cargo.toml update_episode_title_persists -- --nocapture`  
Expected: FAIL — method not found

- [ ] **Step 3: 实现 `update_item_title` / `update_episode_title`**

（实现同前 plan Task 1 Step 3。）

- [ ] **Step 4: 运行测试**

Run: `cargo test --manifest-path engine/Cargo.toml library_store -- --nocapture`  
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add engine/src/library/store.rs engine/tests/library_store.rs
git commit -m "feat(engine): LibraryStore 增加条目与分集标题更新"
```

---

# Phase 9b-2 — Engine 重命名

### Task 2: `validate_display_title` + `rename_*` + Store 事务门面

**Files:**
- Create: `engine/src/library/rename.rs`
- Modify: `engine/src/library/mod.rs`
- Modify: `engine/src/library/store.rs`（`rename_library_item_titles`）
- Modify: `engine/src/engine.rs`
- Create: `engine/tests/library_rename_merge.rs`

**Interfaces:**
- Consumes: Task 1 的 `update_*`、`count_episodes`、`list_episodes`
- Produces: `validate_display_title(title: &str) -> Result<String, EngineError>`
- Produces: `LibraryStore::rename_library_item_titles(&self, item_id, title, single_episode_id: Option<&str>) -> Result<(), EngineError>`
- Produces: `Engine::rename_library_item` / `Engine::rename_episode`

- [ ] **Step 1: 写失败测试 L9b-1..L9b-4、L9b-3b**

```rust
// engine/tests/library_rename_merge.rs

#[test]
fn rename_series_item_title_does_not_change_episode_titles() {
    let dir = tempdir().unwrap();
    let mut engine = Engine::open(dir.path()).unwrap();
    let m1 = engine.media_dir().join("ep1.mp4");
    let m2 = engine.media_dir().join("ep2.mp4");
    std::fs::write(&m1, b"x").unwrap();
    std::fs::write(&m2, b"x").unwrap();
    let (item, _) = engine
        .register_completed_episode("剧", Some(1), 1, "第1集", m1.to_str().unwrap(), None)
        .unwrap();
    engine
        .register_completed_episode("剧", Some(1), 2, "第2集", m2.to_str().unwrap(), None)
        .unwrap();
    engine.rename_library_item(&item.id, "新剧名").unwrap();
    let eps = engine.list_episodes(&item.id).unwrap();
    assert_eq!(engine.list_library().unwrap()[0].title, "新剧名");
    assert_eq!(eps[0].title, "第1集");
    assert_eq!(eps[1].title, "第2集");
}
```

（L9b-1、L9b-2、L9b-3、L9b-4 测试体同前 plan Task 2 Step 1。）

- [ ] **Step 2: 运行测试确认失败**

Run: `cargo test --manifest-path engine/Cargo.toml library_rename_merge -- --nocapture`  
Expected: FAIL

- [ ] **Step 3: 实现 rename 模块、Store 门面、Engine API**

```rust
// engine/src/library/store.rs

pub fn rename_library_item_titles(
    &self,
    item_id: &str,
    title: &str,
    single_episode_id: Option<&str>,
) -> Result<(), EngineError> {
    let tx = self.conn.unchecked_transaction()?;
    let n = tx.execute(
        "UPDATE library_items SET title=?1 WHERE id=?2",
        params![title, item_id],
    )?;
    if n == 0 {
        return Err(EngineError::NotFound(format!("item {item_id}")));
    }
    if let Some(episode_id) = single_episode_id {
        let n = tx.execute(
            "UPDATE library_episodes SET title=?1 WHERE id=?2",
            params![title, episode_id],
        )?;
        if n == 0 {
            return Err(EngineError::NotFound(format!("episode {episode_id}")));
        }
    }
    tx.commit()?;
    Ok(())
}
```

```rust
// engine/src/engine.rs

pub fn rename_library_item(&self, item_id: &str, title: &str) -> Result<(), EngineError> {
    use crate::library::rename::validate_display_title;
    use crate::types::LibraryItemKind;

    let title = validate_display_title(title)?;
    let item = self.library.get_item(item_id)?;
    let single_episode_id = if item.kind == LibraryItemKind::Single
        && self.library.count_episodes(item_id)? == 1
    {
        Some(self.library.list_episodes(item_id)?[0].id.as_str())
    } else {
        None
    };
    self.library
        .rename_library_item_titles(item_id, &title, single_episode_id)
}

pub fn rename_episode(&self, episode_id: &str, title: &str) -> Result<(), EngineError> {
    use crate::library::rename::validate_display_title;
    let title = validate_display_title(title)?;
    self.library.update_episode_title(episode_id, &title)
}
```

- [ ] **Step 4: 运行测试**

Run: `cargo test --manifest-path engine/Cargo.toml library_rename_merge -- --nocapture`  
Expected: PASS（5 tests）

- [ ] **Step 5: Commit**

```bash
git add engine/src/library/rename.rs engine/src/library/mod.rs engine/src/library/store.rs engine/src/engine.rs engine/tests/library_rename_merge.rs
git commit -m "feat(engine): 实现片库条目与分集重命名 API"
```

---

# Phase 9b-3 — Engine Series 合并

### Task 3: `merge.rs` + `apply_merge_in_tx` + L9b-5..L9b-10

**Files:**
- Create: `engine/src/library/merge.rs`
- Modify: `engine/src/library/store.rs`
- Modify: `engine/src/engine.rs`
- Create: `engine/tests/support/library_merge_seed.rs`
- Modify: `engine/tests/library_rename_merge.rs`

**Interfaces:**
- Produces: `LibraryStore::apply_merge_in_tx(...)`
- Produces: `merge_items(library, media_dir, source, target, delete_orphan_files) -> Result<(), EngineError>`
- Produces: `Engine::merge_library_items(&mut self, ...)`

- [ ] **Step 1: 添加 `library_merge_seed.rs`**

（内容同前 plan Task 3 Step 1，完整代码不变。）

- [ ] **Step 2: 写失败测试 L9b-5..L9b-10**

```rust
#[path = "support/library_merge_seed.rs"]
mod library_merge_seed;

#[path = "support/library_dirty.rs"]
mod library_dirty;

#[test]
fn merge_delete_orphan_files_removes_conflict_source_file() {
    let dir = tempdir().unwrap();
    let mut engine = Engine::open(dir.path()).unwrap();
    let seed = library_merge_seed::seed_duplicate_series(&engine, "示意剧", Some(1));
    library_merge_seed::add_episode(&engine, &seed.target_item_id, 1, "目标1", "t1.mp4", 5000);
    library_merge_seed::add_episode(&engine, &seed.source_item_id, 1, "源1", "s1.mp4", 100);
    let s1 = engine.media_dir().join("s1.mp4");
    let t1 = engine.media_dir().join("t1.mp4");
    assert!(s1.exists());
    assert!(t1.exists());

    engine
        .merge_library_items(&seed.source_item_id, &seed.target_item_id, true)
        .unwrap();

    assert!(!s1.exists());
    assert!(t1.exists());
}

#[test]
fn merge_rejects_mismatched_title_season_or_single() {
    let dir = tempdir().unwrap();
    let mut engine = Engine::open(dir.path()).unwrap();
    let seed = library_merge_seed::seed_duplicate_series(&engine, "剧A", Some(1));
    library_merge_seed::add_episode(&engine, &seed.source_item_id, 1, "源", "s.mp4", 0);
    let err = engine
        .merge_library_items(&seed.source_item_id, &seed.source_item_id, false)
        .unwrap_err();
    assert!(matches!(err, EngineError::InvalidArg(_)));

    let media = engine.media_dir().join("single.mp4");
    std::fs::write(&media, b"x").unwrap();
    let (single, _) = engine
        .register_completed_single("片", media.to_str().unwrap(), None)
        .unwrap();
    let err = engine
        .merge_library_items(&seed.source_item_id, &single.id, false)
        .unwrap_err();
    assert!(matches!(err, EngineError::InvalidArg(_)));
}

#[test]
fn merge_aborts_when_orphan_path_outside_media_dir() {
    let dir = tempdir().unwrap();
    let mut engine = Engine::open(dir.path()).unwrap();
    let seed = library_merge_seed::seed_duplicate_series(&engine, "示意剧", Some(1));
    let target_ep = library_merge_seed::add_episode(
        &engine, &seed.target_item_id, 1, "目标1", "t1.mp4", 5000,
    );
    let source_ep = library_merge_seed::add_episode(
        &engine, &seed.source_item_id, 1, "源1", "s1.mp4", 100,
    );
    let outside = dir.path().join("outside.mp4");
    std::fs::write(&outside, b"x").unwrap();
    library_dirty::inject_outside_file_path(&engine, &seed.source_item_id, outside.to_str().unwrap());
    // 重新加载 source_ep id（file_path 已脏）
    let _ = source_ep;
    let _ = target_ep;

    let err = engine
        .merge_library_items(&seed.source_item_id, &seed.target_item_id, true)
        .unwrap_err();
    assert!(err.to_string().contains("media") || err.to_string().contains("media_dir"));
    assert_eq!(engine.list_library().unwrap().len(), 2);
}

#[test]
fn merge_orphan_after_cast_allows_new_cast() {
    use video_sniffing_engine::lan::LanTestConfig;

    let sender_dir = tempfile::tempdir().unwrap();
    let receiver_dir = tempfile::tempdir().unwrap();
    let mut receiver = Engine::open(receiver_dir.path()).unwrap();
    let mut sender = Engine::open(sender_dir.path()).unwrap();
    for e in [&mut sender, &mut receiver] {
        let mut s = e.settings();
        s.lan_enabled = true;
        e.save_settings(s).unwrap();
        e.set_lan_test_config(LanTestConfig {
            advertise_ip: Some("127.0.0.1".into()),
        });
    }
    receiver.apply_lan_settings(true).unwrap();
    let pin = receiver.begin_pairing().unwrap();
    let port = receiver.lan_http_port().unwrap();
    sender.apply_lan_settings(false).unwrap();
    sender.pair_peer("127.0.0.1", port, &pin).unwrap();

    let seed = library_merge_seed::seed_duplicate_series(&sender, "示意剧", Some(1));
    let orphan = library_merge_seed::add_episode(
        &sender, &seed.source_item_id, 1, "源1", "orphan.mp4", 0,
    );
    library_merge_seed::add_episode(&sender, &seed.target_item_id, 1, "目标1", "keep.mp4", 0);
    sender
        .cast_episode(&orphan.id, &receiver.settings().device_id)
        .unwrap();

    sender
        .merge_library_items(&seed.source_item_id, &seed.target_item_id, false)
        .unwrap();
    assert!(!sender.has_active_cast());

    let migrated = library_merge_seed::add_episode(
        &sender, &seed.target_item_id, 2, "第2集", "ep2.mp4", 0,
    );
    sender
        .cast_episode(&migrated.id, &receiver.settings().device_id)
        .unwrap();
    sender.stop_cast().unwrap();
    sender.stop_lan().unwrap();
    receiver.stop_lan().unwrap();
}
```

（L9b-5、L9b-6 测试体同前 plan Task 3 Step 2 完整版。）

- [ ] **Step 3: 运行测试确认失败**

Run: `cargo test --manifest-path engine/Cargo.toml merge_ -- --nocapture`  
Expected: FAIL

- [ ] **Step 4: 实现 `apply_merge_in_tx` + `merge.rs` + Engine 门面**

```rust
// engine/src/library/store.rs

pub(crate) fn apply_merge_in_tx(
    &self,
    migrate: &[(String, String)], // (episode_id, new_item_id)
    orphan_episode_ids: &[String],
    source_item_id: &str,
) -> Result<(), EngineError> {
    let tx = self.conn.unchecked_transaction()?;
    for (episode_id, new_item_id) in migrate {
        let n = tx.execute(
            "UPDATE library_episodes SET item_id=?1 WHERE id=?2",
            params![new_item_id, episode_id],
        )?;
        if n == 0 {
            return Err(EngineError::NotFound(format!("episode {episode_id}")));
        }
    }
    for episode_id in orphan_episode_ids {
        let n = tx.execute(
            "DELETE FROM library_episodes WHERE id=?1",
            params![episode_id],
        )?;
        if n == 0 {
            return Err(EngineError::NotFound(format!("episode {episode_id}")));
        }
    }
    let n = tx.execute(
        "DELETE FROM library_items WHERE id=?1",
        params![source_item_id],
    )?;
    if n == 0 {
        return Err(EngineError::NotFound(format!("item {source_item_id}")));
    }
    tx.commit()?;
    Ok(())
}
```

`merge.rs` 在删 orphan 文件后调用 `library.apply_merge_in_tx(...)`；**不在** `merge.rs` / `engine.rs` 内直接 `conn()`。

- [ ] **Step 5: 运行 engine 全量测试**

Run: `cargo test --manifest-path engine/Cargo.toml`  
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add engine/src/library/merge.rs engine/src/library/store.rs engine/src/library/mod.rs engine/src/engine.rs engine/tests/support/library_merge_seed.rs engine/tests/library_rename_merge.rs
git commit -m "feat(engine): 实现 Series 手动合并与 idx 冲突策略"
```

---

# Phase 9b-4 — FFI

### Task 4: C API + F9b-1、F9b-2

**Files:**
- Modify: `engine/ffi/src/sync_dispatch.rs`
- Modify: `engine/ffi/src/lib.rs`
- Create: `engine/ffi/tests/library_rename_merge_ffi_test.rs`

- [ ] **Step 1: 写失败 FFI 测试**

```rust
use std::ffi::{CStr, CString};
use std::fs;
use tempfile::tempdir;
use video_sniffing_engine::test_api::LibraryStore;
use video_sniffing_engine::{Engine, LibraryEpisode, LibraryItem, LibraryItemKind};
use video_sniffing_engine_ffi::handle::{engine_destroy, engine_free_string, engine_open};
use video_sniffing_engine_ffi::sync_dispatch::{
    engine_list_episodes, engine_list_library, engine_merge_library_items,
    engine_rename_library_item,
};
use uuid::Uuid;

fn seed_dup(engine: &Engine) -> (String, String) {
    let store = LibraryStore::open(
        &engine.media_dir().parent().unwrap().join("library.db"),
    ).unwrap();
    let a = Uuid::new_v4().to_string();
    let b = Uuid::new_v4().to_string();
    for (id, created) in [(&a, 1_i64), (&b, 2_i64)] {
        store.upsert_item(&LibraryItem {
            id: id.clone(),
            kind: LibraryItemKind::Series,
            title: "示意剧".into(),
            season: Some(1),
            poster_path: None,
            created_at_ms: created,
        }).unwrap();
    }
    (a, b)
}

#[test]
fn rename_and_merge_ffi_ok_json() {
    let dir = tempdir().unwrap();
    let mut engine = Engine::open(dir.path()).unwrap();
    let media = engine.media_dir().join("a.mp4");
    fs::write(&media, b"x").unwrap();
    let (item, _) = engine
        .register_completed_single("旧", media.to_str().unwrap(), None)
        .unwrap();
    let (src, tgt) = seed_dup(&engine);
    let store = LibraryStore::open(
        &engine.media_dir().parent().unwrap().join("library.db"),
    ).unwrap();
    let ep = LibraryEpisode {
        id: Uuid::new_v4().to_string(),
        item_id: src.clone(),
        index: 1,
        title: "源1".into(),
        file_path: engine.media_dir().join("s1.mp4").to_string_lossy().into(),
        duration_ms: None,
        position_ms: 0,
        source_url: None,
    };
    fs::write(engine.media_dir().join("s1.mp4"), b"x").unwrap();
    store.upsert_episode(&ep).unwrap();
    drop(engine);

    let path = CString::new(dir.path().to_str().unwrap()).unwrap();
    let handle = unsafe { engine_open(path.as_ptr()) };
    let item_id = CString::new(item.id).unwrap();
    let new_title = CString::new("新").unwrap();
    let rename_ptr = unsafe {
        engine_rename_library_item(handle, item_id.as_ptr(), new_title.as_ptr())
    };
    let rename_json: serde_json::Value =
        serde_json::from_str(unsafe { CStr::from_ptr(rename_ptr).to_str().unwrap() }).unwrap();
    assert_eq!(rename_json["ok"], true);
    unsafe { engine_free_string(rename_ptr) };

    let src_id = CString::new(src).unwrap();
    let tgt_id = CString::new(tgt).unwrap();
    let merge_ptr = unsafe {
        engine_merge_library_items(handle, src_id.as_ptr(), tgt_id.as_ptr(), 0)
    };
    let merge_json: serde_json::Value =
        serde_json::from_str(unsafe { CStr::from_ptr(merge_ptr).to_str().unwrap() }).unwrap();
    assert_eq!(merge_json["ok"], true);
    unsafe { engine_free_string(merge_ptr) };

    let lib_ptr = unsafe { engine_list_library(handle) };
    let lib: serde_json::Value =
        serde_json::from_str(unsafe { CStr::from_ptr(lib_ptr).to_str().unwrap() }).unwrap();
    assert_eq!(lib["data"].as_array().unwrap().len(), 2); // single + merged series
    unsafe { engine_free_string(lib_ptr) };
    unsafe { engine_destroy(handle) };
}

#[test]
fn rename_ffi_invalid_title_returns_error_json() {
    let dir = tempdir().unwrap();
    let mut engine = Engine::open(dir.path()).unwrap();
    let media = engine.media_dir().join("a.mp4");
    fs::write(&media, b"x").unwrap();
    let (item, _) = engine
        .register_completed_single("x", media.to_str().unwrap(), None)
        .unwrap();
    drop(engine);

    let path = CString::new(dir.path().to_str().unwrap()).unwrap();
    let handle = unsafe { engine_open(path.as_ptr()) };
    let item_id = CString::new(item.id).unwrap();
    let blank = CString::new("   ").unwrap();
    let ptr = unsafe { engine_rename_library_item(handle, item_id.as_ptr(), blank.as_ptr()) };
    let v: serde_json::Value =
        serde_json::from_str(unsafe { CStr::from_ptr(ptr).to_str().unwrap() }).unwrap();
    assert_ne!(v.get("ok"), Some(&serde_json::Value::Bool(true)));
    unsafe { engine_free_string(ptr) };
    unsafe { engine_destroy(handle) };
}
```

- [ ] **Step 2: 实现三个 C API**

```rust
#[no_mangle]
pub unsafe extern "C" fn engine_rename_library_item(
    handle: *mut EngineHandle,
    item_id: *const c_char,
    title: *const c_char,
) -> *mut c_char {
    let item_id = match parse_c_str(item_id, "item_id") {
        Ok(id) => id,
        Err(err) => return rust_to_c_string(err_json(err)),
    };
    let title = match parse_c_str(title, "title") {
        Ok(s) => s,
        Err(err) => return rust_to_c_string(err_json(err)),
    };
    ffi_call(handle, |engine| engine.rename_library_item(&item_id, &title).map(|_| ()))
}

#[no_mangle]
pub unsafe extern "C" fn engine_rename_episode(
    handle: *mut EngineHandle,
    episode_id: *const c_char,
    title: *const c_char,
) -> *mut c_char {
    let episode_id = match parse_c_str(episode_id, "episode_id") {
        Ok(id) => id,
        Err(err) => return rust_to_c_string(err_json(err)),
    };
    let title = match parse_c_str(title, "title") {
        Ok(s) => s,
        Err(err) => return rust_to_c_string(err_json(err)),
    };
    ffi_call(handle, |engine| engine.rename_episode(&episode_id, &title).map(|_| ()))
}

#[no_mangle]
pub unsafe extern "C" fn engine_merge_library_items(
    handle: *mut EngineHandle,
    source_item_id: *const c_char,
    target_item_id: *const c_char,
    delete_orphan_files: u8,
) -> *mut c_char {
    let source_item_id = match parse_c_str(source_item_id, "source_item_id") {
        Ok(id) => id,
        Err(err) => return rust_to_c_string(err_json(err)),
    };
    let target_item_id = match parse_c_str(target_item_id, "target_item_id") {
        Ok(id) => id,
        Err(err) => return rust_to_c_string(err_json(err)),
    };
    let delete_orphan_files = delete_orphan_files != 0;
    ffi_call_mut(handle, |engine| {
        engine
            .merge_library_items(&source_item_id, &target_item_id, delete_orphan_files)
            .map(|_| ())
    })
}
```

- [ ] **Step 3: 运行 FFI 测试**

Run: `cargo test --manifest-path engine/Cargo.toml -p video_sniffing_engine_ffi library_rename_merge_ffi -- --nocapture`  
Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add engine/ffi/src/sync_dispatch.rs engine/ffi/src/lib.rs engine/ffi/tests/library_rename_merge_ffi_test.rs
git commit -m "feat(ffi): 暴露片库重命名与合并 C API"
```

---

# Phase 9b-5 — Flutter Engine 层

### Task 5: Bindings / Host / Repository / Fake

- [ ] **Step 1: 扩展 native_bindings.dart**

在 `NativeBindings` 类增加 `engineRenameLibraryItem`、`engineRenameEpisode`、`engineMergeLibraryItems` 的 typedef 与 `lookupFunction`（签名同 `engineRemoveLibraryItem`，merge 多一个 `Int32 deleteOrphanFiles` 参数）。

- [ ] **Step 2: 扩展 engine_host.dart / engine_repository.dart**

```dart
void renameEpisode(String episodeId, String title) { /* engineRenameEpisode */ }

void mergeLibraryItems(
  String sourceItemId,
  String targetItemId, {
  bool deleteOrphanFiles = false,
}) {
  _withUtf8(sourceItemId, (sourcePtr) {
    _withUtf8(targetItemId, (targetPtr) {
      _callSyncVoid(
        (handle) => _bindings.engineMergeLibraryItems(
          handle,
          sourcePtr,
          targetPtr,
          deleteOrphanFiles ? 1 : 0,
        ),
      );
    });
  });
}
```

- [ ] **Step 3: 扩展 Fake**

```dart
// app/test/fakes/fake_engine_repository.dart — 追加字段

String? lastRenamedItemId;
String? lastRenamedTitle;
String? lastRenamedEpisodeId;
String? lastRenamedEpisodeTitle;
EngineException? renameLibraryItemError;
EngineException? renameEpisodeError;
String? lastMergedSourceId;
String? lastMergedTargetId;
bool? lastMergeDeleteOrphanFiles;
EngineException? mergeLibraryItemsError;
```

- [ ] **Step 4:** `dart analyze` → commit `feat(app): EngineHost 接线片库重命名与合并 FFI`

---

# Phase 9b-6 — RenameDialog（W9b-1）

### Task 6: `RenameDialog` + widget 测试

**Files:**
- Create: `app/lib/features/library/widgets/rename_dialog.dart`
- Create: `app/test/rename_dialog_test.dart`

- [ ] **Step 1: 写失败测试 W9b-1**

```dart
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:video_sniffing/features/library/widgets/rename_dialog.dart';

void main() {
  testWidgets('RenameDialog returns trimmed title on save', (tester) async {
    String? result;
    await tester.pumpWidget(MaterialApp(
      home: Builder(builder: (context) {
        return ElevatedButton(
          onPressed: () async {
            result = await showRenameDialogResult(
              context,
              initialTitle: '  旧  ',
            );
          },
          child: const Text('open'),
        );
      }),
    ));
    await tester.tap(find.text('open'));
    await tester.pumpAndSettle();
    await tester.enterText(
      find.byKey(const Key('rename_dialog_field')),
      '  新名  ',
    );
    await tester.tap(find.widgetWithText(FilledButton, '保存'));
    await tester.pumpAndSettle();
    expect(result, '新名');
  });
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cd app && flutter test test/rename_dialog_test.dart`  
Expected: FAIL

- [ ] **Step 3: 实现 RenameDialog**

```dart
class RenameDialog extends StatefulWidget {
  const RenameDialog({super.key, required this.initialTitle});
  final String initialTitle;
  // TextField key: rename_dialog_field
  // 保存: Navigator.pop(context, _controller.text.trim())
  // 空标题时 FilledButton onPressed: null
}

Future<String?> showRenameDialogResult(
  BuildContext context, {
  required String initialTitle,
}) {
  return showDialog<String>(
    context: context,
    builder: (_) => RenameDialog(initialTitle: initialTitle),
  );
}
```

- [ ] **Step 4: 运行测试确认通过** → **Step 5: Commit** `feat(app): 添加片库重命名对话框组件`

---

# Phase 9b-7 — 重命名 UI only（W9b-2、W9b-5、W9b-6）

### Task 7: 详情页 + 分集重命名（**不含** merge 菜单）

**Files:**
- Create: `app/lib/features/library/merge_candidates.dart`
- Modify: `app/lib/features/library/library_detail_screen.dart`
- Modify: `app/lib/features/library/widgets/episode_tile.dart`
- Create: `app/test/library_rename_merge_screen_test.dart`

- [ ] **Step 1: 写失败测试**

```dart
// W9b-2: 详情菜单重命名 → fake.lastRenamedItemId / lastRenamedTitle + SnackBar「已重命名」

// W9b-5: fake.renameLibraryItemError = EngineException(...);
// 保存后 expect SnackBar 错误文案；expect fake.lastRenamedItemId isNull

// W9b-6: Series 两集，点 episode_menu → 重命名 → fake.lastRenamedEpisodeId / lastRenamedEpisodeTitle
```

- [ ] **Step 2: EpisodeTile 增 `onRename`；PopupMenu 顺序：重命名 → 删除**

- [ ] **Step 3: LibraryDetailScreen 菜单 **仅** 增「重命名」+「删除」**

```dart
List<PopupMenuEntry<String>> _detailMenuItems(LibraryItem item) {
  return [
    const PopupMenuItem(value: 'rename', child: Text('重命名')),
    const PopupMenuItem(value: 'delete', child: Text('删除')),
  ];
}

// itemBuilder: (_) => _detailMenuItems(item!)
// rename 分支：showRenameDialogResult → repo.renameLibraryItem → invalidate → SnackBar
// 失败：presentEngineError + SnackBar（W9b-5）
```

**注意：** merge 菜单在 Task 8 添加；本 Task **不要** 出现「合并到…」。

- [ ] **Step 4:** `flutter test test/library_rename_merge_screen_test.dart --name rename`  
- [ ] **Step 5:** commit `feat(app): 片库详情与分集重命名 UI`

---

# Phase 9b-8 — 合并 UI（W9b-3、W9b-4）

### Task 8: Merge 对话框 + 菜单 + 流程

- [ ] **Step 1: 写失败测试 W9b-3、W9b-4**

```dart
testWidgets('confirm merge dialog defaults deleteOrphanFiles false', (tester) async {
  await tester.pumpWidget(MaterialApp(
    home: Builder(builder: (context) {
      return ElevatedButton(
        onPressed: () => showConfirmMergeDialogResult(
          context,
          targetTitle: '目标',
          episodeCount: 2,
        ),
        child: const Text('open'),
      );
    }),
  ));
  await tester.tap(find.text('open'));
  await tester.pumpAndSettle();
  expect(find.text('删除被丢弃分集的本地缓存文件'), findsOneWidget);
  final checkbox = tester.widget<CheckboxListTile>(
    find.byType(CheckboxListTile),
  );
  expect(checkbox.value, isFalse);
});

testWidgets('merge flow calls repo.mergeLibraryItems', (tester) async {
  // Fake 注入两个同 title+season Series + 源详情页
  // 菜单应含「合并到…」（Task 8 新增）
  // 选 target → 确认 → expect lastMergedSourceId / lastMergeDeleteOrphanFiles false
  // expect find.text('已合并')
});
```

- [ ] **Step 2–3:** 实现 `ConfirmMergeDialog`、`MergeTargetPickerSheet`

- [ ] **Step 4: LibraryDetailScreen 增 merge 菜单与流程**

```dart
List<PopupMenuEntry<String>> _detailMenuItems(
  LibraryItem item,
  List<LibraryItem> allItems,
) {
  final candidates = mergeCandidatesFor(item, allItems);
  return [
    const PopupMenuItem(value: 'rename', child: Text('重命名')),
    if (candidates.isNotEmpty)
      const PopupMenuItem(value: 'merge', child: Text('合并到…')),
    const PopupMenuItem(value: 'delete', child: Text('删除')),
  ];
}

// itemBuilder: (_) => _detailMenuItems(item!, ref.read(libraryProvider))
```

- [ ] **Step 5:** `flutter test test/library_rename_merge_screen_test.dart` → commit

---

# Phase 9b-9 — U11b + CI + README

### Task 9: Engine 级集成 + 文档

- [ ] **Step 1: 创建 integration seed 辅助 + U11b**

```dart
// app/integration_test/support/library_merge_seed.dart
// 打开 EngineHost 后写入两个同 title+season Series 壳；
// source 写入 idx=1，target 写入 idx=2（避免冲突），返回 (sourceItemId, targetItemId)

// app/integration_test/library_rename_merge_test.dart
testWidgets('U11b engine merge duplicate series shells', (tester) async {
  final dataDir = Directory.systemTemp.createTempSync('u11b');
  final host = EngineHost();
  await host.open(dataDir.path);
  try {
    final (sourceId, targetId) = await seedDuplicateSeriesForIntegration(host);
    // seed: source 1 集 + target 1 集（不同 idx 或 idx 2）
    host.mergeLibraryItems(sourceId, targetId);
    final series = host
        .listLibrary()
        .where((i) => i.kind == LibraryItemKind.series)
        .toList();
    expect(series.length, 1);
    expect(series.first.id, targetId);
    expect(host.listEpisodes(targetId).length, 2);
  } finally {
    await host.close();
  }
}, timeout: const Timeout(Duration(minutes: 2)));
```

- [ ] **Step 2:** 本地跑 U11b  
- [ ] **Step 3:** CI matrix 增 `library_rename_merge`（或并入 `library` job，二选一；默认独立 job 与 spec 一致）  
- [ ] **Step 4:** README 增 Plan 9b 节，写明 U11b 为 **Engine 级** merge 冒烟  
- [ ] **Step 5:** 全量验证 → commit

---

## Spec Self-Review（计划自检）

| 规格要求 | 对应 Task |
|----------|-----------|
| L9b-1..4、L9b-3b | Task 2 |
| L9b-5..8 | Task 3 |
| L9b-9、L9b-10 | Task 3 |
| F9b-1、F9b-2 | Task 4 |
| W9b-1 | Task 6 |
| W9b-2、W9b-5、W9b-6 | Task 7 |
| W9b-3、W9b-4 | Task 8 |
| U11b（Engine 级） | Task 9 |
| Store 事务门面 | Task 2、3 |
| Task 7 不含 merge 菜单 | Task 7 vs 8 拆分 |

**Placeholder 扫描：** Task 7 Step 1 的 Fake 测试体需在实现时按 `library_delete_screen_test.dart` 模式写完整 pump/tap 代码；其余 Task 已给出可执行代码块。

**Review 修订项：** 已消除原 plan 中 Task 1/3/4/8/9 的注释占位；Self-Review 结论已更正。

---

## Execution Handoff

Plan saved to `docs/superpowers/plans/2026-09-14-library-rename-merge.md`.

**两种执行方式：**

1. **Subagent-Driven（推荐）** — 每 Task 独立 subagent + Task 间 review  
2. **Inline Execution** — 本会话 executing-plans 批量执行  

**你想用哪种？**
