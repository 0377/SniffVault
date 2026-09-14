# Plan 9b 片库重命名与 Series 合并 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 用户可在片库详情重命名条目/分集标题，并将误建的重复 Series 壳合并到同 `title + season` 的目标条目；Engine 为唯一写入口，含 FFI、Flutter UI 与自动化测试。

**Architecture:** `LibraryStore` 增加 title UPDATE 与 merge 事务辅助；`engine/src/library/rename.rs` 校验展示标题；`engine/src/library/merge.rs` 实现 orphan/migrate 分类、LAN 清理、可选删文件与单事务提交；`Engine` 公开三个门面方法；FFI → `EngineHost` → Riverpod UI 复用 9a 详情页菜单模式。

**Tech Stack:** Rust (`rusqlite`)、C FFI（Cargokit）、Flutter + Riverpod + go_router

**规格:** `docs/superpowers/specs/2026-09-14-library-rename-merge-design.md`

## Global Constraints

- **Engine 是唯一片库写入口**：UI / FFI 不得直写 `LibraryStore`
- **重命名不改 `file_path`**：仅更新 `library_items.title`、`library_episodes.title`
- **Single 重命名**：`rename_library_item` 在同一事务内同步唯一分集 title
- **展示标题校验**：trim 后非空、`len <= 512`
- **合并单事务**：migrate UPDATE + orphan DELETE + 删源壳在 **单一 SQLite 事务**；`delete_orphan_files=true` 时磁盘删除在事务 **之前**，失败则 DB 不变
- **idx 冲突保留目标**：保留目标分集及 `position_ms`；orphan 源分集经 LAN 清理
- **`delete_orphan_files` 默认 false**（Flutter 确认框复选框默认不勾选）
- **合并候选**：Flutter 从 `libraryProvider` 过滤，不增 Engine 列表 API
- **验证命令**（仓库根目录）：`cargo fmt --check`、`cargo test`、`cargo clippy -D warnings`；`cd app && flutter test`
- **`docs/` 在 `.gitignore`**：提交文档用 `git add -f docs/...`
- **提交信息中文**

---

## File Map

| 路径 | 职责 |
|------|------|
| `engine/src/library/store.rs` | `update_item_title`、`update_episode_title`、merge 事务 SQL |
| `engine/src/library/rename.rs` | `validate_display_title`（新建） |
| `engine/src/library/merge.rs` | 合并算法（新建） |
| `engine/src/library/mod.rs` | `mod rename; mod merge;` |
| `engine/src/engine.rs` | `rename_library_item`、`rename_episode`、`merge_library_items` |
| `engine/tests/support/library_merge_seed.rs` | 测试用重复 Series 壳种子 |
| `engine/tests/library_rename_merge.rs` | L9b-1..L9b-9 |
| `engine/ffi/src/sync_dispatch.rs` | 三个 C API |
| `engine/ffi/src/lib.rs` | 导出新符号 |
| `engine/ffi/tests/library_rename_merge_ffi_test.rs` | F9b-1、F9b-2 |
| `app/lib/engine/native_bindings.dart` | Dart FFI typedef + lookup |
| `app/lib/engine/engine_host.dart` | rename / merge 方法 |
| `app/lib/providers/engine_repository.dart` | 抽象 + 实现 |
| `app/test/fakes/fake_engine_repository.dart` | Fake 记录调用 |
| `app/lib/features/library/widgets/rename_dialog.dart` | 重命名对话框 |
| `app/lib/features/library/widgets/merge_target_picker_sheet.dart` | 合并目标选择 |
| `app/lib/features/library/widgets/confirm_merge_dialog.dart` | 合并确认 |
| `app/lib/features/library/merge_candidates.dart` | `isMergeCandidate` 过滤 |
| `app/lib/features/library/library_detail_screen.dart` | 菜单与流程 |
| `app/lib/features/library/widgets/episode_tile.dart` | 分集重命名菜单 |
| `app/test/rename_dialog_test.dart` | W9b-1 |
| `app/test/library_rename_merge_screen_test.dart` | W9b-2..W9b-4 |
| `app/integration_test/library_rename_merge_test.dart` | U11b |
| `.github/workflows/ci.yml` | matrix 增 `library_rename_merge` |
| `README.md` | Plan 9b / U11b 门禁 |

---

# Phase 9b-1 — LibraryStore 标题更新

### Task 1: `update_item_title` / `update_episode_title`

**Files:**
- Modify: `engine/src/library/store.rs`
- Modify: `engine/tests/library_store.rs`

**Interfaces:**
- Produces: `LibraryStore::update_item_title(&self, item_id: &str, title: &str) -> Result<(), EngineError>`
- Produces: `LibraryStore::update_episode_title(&self, episode_id: &str, title: &str) -> Result<(), EngineError>`

- [ ] **Step 1: 写失败测试**

```rust
// engine/tests/library_store.rs — 追加

#[test]
fn update_item_title_persists() {
    let dir = tempdir().unwrap();
    let store = LibraryStore::open(&dir.path().join("library.db")).unwrap();
    store
        .upsert_item(&LibraryItem {
            id: "i1".into(),
            kind: LibraryItemKind::Single,
            title: "旧名".into(),
            season: None,
            poster_path: None,
            created_at_ms: 1,
        })
        .unwrap();
    store.update_item_title("i1", "新名").unwrap();
    let item = store.get_item("i1").unwrap();
    assert_eq!(item.title, "新名");
}

#[test]
fn update_episode_title_persists() {
    let dir = tempdir().unwrap();
    let store = LibraryStore::open(&dir.path().join("library.db")).unwrap();
    // ... upsert item + episode ...
    store.update_episode_title("e1", "新分集名").unwrap();
    let ep = store.get_episode("e1").unwrap().unwrap();
    assert_eq!(ep.title, "新分集名");
}

#[test]
fn update_item_title_missing_returns_not_found() {
    let dir = tempdir().unwrap();
    let store = LibraryStore::open(&dir.path().join("library.db")).unwrap();
    let err = store.update_item_title("missing", "x").unwrap_err();
    assert!(matches!(err, EngineError::NotFound(_)));
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cargo test --manifest-path engine/Cargo.toml library_store::update_item_title -- --nocapture`  
Expected: FAIL — method not found

- [ ] **Step 3: 最小实现**

```rust
// engine/src/library/store.rs

pub fn update_item_title(&self, item_id: &str, title: &str) -> Result<(), EngineError> {
    let n = self.conn.execute(
        "UPDATE library_items SET title=?1 WHERE id=?2",
        params![title, item_id],
    )?;
    if n == 0 {
        return Err(EngineError::NotFound(format!("item {item_id}")));
    }
    Ok(())
}

pub fn update_episode_title(&self, episode_id: &str, title: &str) -> Result<(), EngineError> {
    let n = self.conn.execute(
        "UPDATE library_episodes SET title=?1 WHERE id=?2",
        params![title, episode_id],
    )?;
    if n == 0 {
        return Err(EngineError::NotFound(format!("episode {episode_id}")));
    }
    Ok(())
}
```

- [ ] **Step 4: 运行测试确认通过**

Run: `cargo test --manifest-path engine/Cargo.toml library_store -- --nocapture`  
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add engine/src/library/store.rs engine/tests/library_store.rs
git commit -m "feat(engine): LibraryStore 增加条目与分集标题更新"
```

---

# Phase 9b-2 — Engine 重命名

### Task 2: `validate_display_title` + `rename_*` API

**Files:**
- Create: `engine/src/library/rename.rs`
- Modify: `engine/src/library/mod.rs`
- Modify: `engine/src/engine.rs`
- Create: `engine/tests/library_rename_merge.rs`

**Interfaces:**
- Consumes: Task 1 的 `update_item_title`、`update_episode_title`、`count_episodes`、`list_episodes`
- Produces: `pub(crate) fn validate_display_title(title: &str) -> Result<String, EngineError>`
- Produces: `Engine::rename_library_item(&self, item_id: &str, title: &str) -> Result<(), EngineError>`
- Produces: `Engine::rename_episode(&self, episode_id: &str, title: &str) -> Result<(), EngineError>`

- [ ] **Step 1: 写失败测试 L9b-1..L9b-4**

```rust
// engine/tests/library_rename_merge.rs

use std::fs;
use tempfile::tempdir;
use video_sniffing_engine::{Engine, EngineError, LibraryItemKind};

#[test]
fn rename_library_item_persists_after_reopen() {
    let dir = tempdir().unwrap();
    let mut engine = Engine::open(dir.path()).unwrap();
    let media = engine.media_dir().join("a.mp4");
    fs::write(&media, b"x").unwrap();
    let (item, _) = engine
        .register_completed_single("旧标题", media.to_str().unwrap(), None)
        .unwrap();
    engine.rename_library_item(&item.id, "  新标题  ").unwrap();
    drop(engine);

    let engine = Engine::open(dir.path()).unwrap();
    let items = engine.list_library().unwrap();
    assert_eq!(items[0].title, "新标题");
}

#[test]
fn rename_single_item_syncs_episode_title() {
    let dir = tempdir().unwrap();
    let mut engine = Engine::open(dir.path()).unwrap();
    let media = engine.media_dir().join("a.mp4");
    fs::write(&media, b"x").unwrap();
    let (item, ep) = engine
        .register_completed_single("旧", media.to_str().unwrap(), None)
        .unwrap();
    engine.rename_library_item(&item.id, "新").unwrap();
    let eps = engine.list_episodes(&item.id).unwrap();
    assert_eq!(eps[0].id, ep.id);
    assert_eq!(eps[0].title, "新");
}

#[test]
fn rename_episode_keeps_file_path() {
    let dir = tempdir().unwrap();
    let mut engine = Engine::open(dir.path()).unwrap();
    let media = engine.media_dir().join("ep1.mp4");
    fs::write(&media, b"x").unwrap();
    let (item, ep) = engine
        .register_completed_episode("剧", Some(1), 1, "旧集名", media.to_str().unwrap(), None)
        .unwrap();
    let path_before = ep.file_path.clone();
    engine.rename_episode(&ep.id, "新集名").unwrap();
    let eps = engine.list_episodes(&item.id).unwrap();
    assert_eq!(eps[0].title, "新集名");
    assert_eq!(eps[0].file_path, path_before);
}

#[test]
fn rename_rejects_empty_and_too_long_title() {
    let dir = tempdir().unwrap();
    let mut engine = Engine::open(dir.path()).unwrap();
    let media = engine.media_dir().join("a.mp4");
    fs::write(&media, b"x").unwrap();
    let (item, ep) = engine
        .register_completed_single("x", media.to_str().unwrap(), None)
        .unwrap();
    assert!(matches!(
        engine.rename_library_item(&item.id, "   "),
        Err(EngineError::InvalidArg(_))
    ));
    let long = "a".repeat(513);
    assert!(matches!(
        engine.rename_episode(&ep.id, &long),
        Err(EngineError::InvalidArg(_))
    ));
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cargo test --manifest-path engine/Cargo.toml library_rename_merge -- --nocapture`  
Expected: FAIL — `rename_library_item` not found

- [ ] **Step 3: 实现 rename 模块与 Engine API**

```rust
// engine/src/library/rename.rs
use crate::error::EngineError;

pub(crate) fn validate_display_title(title: &str) -> Result<String, EngineError> {
    let trimmed = title.trim();
    if trimmed.is_empty() {
        return Err(EngineError::InvalidArg("title must not be empty".into()));
    }
    if trimmed.len() > 512 {
        return Err(EngineError::InvalidArg("title too long".into()));
    }
    Ok(trimmed.to_string())
}
```

```rust
// engine/src/engine.rs — 追加

pub fn rename_library_item(&self, item_id: &str, title: &str) -> Result<(), EngineError> {
    use crate::library::rename::validate_display_title;
    use crate::types::LibraryItemKind;

    let title = validate_display_title(title)?;
    let item = self.library.get_item(item_id)?;
    let tx = self.library.conn().unchecked_transaction()?;
    self.library
        .update_item_title_in_tx(&tx, item_id, &title)?;
    if item.kind == LibraryItemKind::Single && self.library.count_episodes(item_id)? == 1 {
        let eps = self.library.list_episodes(item_id)?;
        self.library
            .update_episode_title_in_tx(&tx, &eps[0].id, &title)?;
    }
    tx.commit()?;
    Ok(())
}

pub fn rename_episode(&self, episode_id: &str, title: &str) -> Result<(), EngineError> {
    use crate::library::rename::validate_display_title;
    let title = validate_display_title(title)?;
    self.library.update_episode_title(episode_id, &title)
}
```

在 `store.rs` 增加 `update_item_title_in_tx` / `update_episode_title_in_tx`（供 Single 同事务；公开 `update_*` 可委托给单语句版本）。

- [ ] **Step 4: 运行测试确认通过**

Run: `cargo test --manifest-path engine/Cargo.toml library_rename_merge -- --nocapture`  
Expected: PASS（4 tests）

- [ ] **Step 5: Commit**

```bash
git add engine/src/library/rename.rs engine/src/library/mod.rs engine/src/library/store.rs engine/src/engine.rs engine/tests/library_rename_merge.rs
git commit -m "feat(engine): 实现片库条目与分集重命名 API"
```

---

# Phase 9b-3 — Engine Series 合并

### Task 3: `merge.rs` + `merge_library_items`

**Files:**
- Create: `engine/src/library/merge.rs`
- Modify: `engine/src/library/mod.rs`
- Modify: `engine/src/library/store.rs`（merge 事务 SQL）
- Modify: `engine/src/engine.rs`
- Create: `engine/tests/support/library_merge_seed.rs`
- Modify: `engine/tests/library_rename_merge.rs`

**Interfaces:**
- Consumes: Task 1 store 方法；`Engine::finalize_lan_for_episodes`（已有 private）
- Consumes: `ingest::ensure_path_in_media_dir`；`library::delete::delete_files`
- Produces: `pub(crate) fn merge_items(...) -> Result<(), EngineError>`（在 merge.rs）
- Produces: `Engine::merge_library_items(&mut self, source_item_id: &str, target_item_id: &str, delete_orphan_files: bool) -> Result<(), EngineError>`

- [ ] **Step 1: 写测试种子辅助**

```rust
// engine/tests/support/library_merge_seed.rs
use std::fs;
use uuid::Uuid;
use video_sniffing_engine::test_api::LibraryStore;
use video_sniffing_engine::{Engine, LibraryEpisode, LibraryItem, LibraryItemKind};

pub struct DuplicateSeriesSeed {
    pub source_item_id: String,
    pub target_item_id: String,
}

pub fn seed_duplicate_series(
    engine: &Engine,
    title: &str,
    season: Option<u32>,
) -> DuplicateSeriesSeed {
    let media_dir = engine.media_dir();
    let data_dir = media_dir.parent().unwrap();
    let store = LibraryStore::open(&data_dir.join("library.db")).unwrap();
    let source_item_id = Uuid::new_v4().to_string();
    let target_item_id = Uuid::new_v4().to_string();
    store.upsert_item(&LibraryItem {
        id: source_item_id.clone(),
        kind: LibraryItemKind::Series,
        title: title.into(),
        season,
        poster_path: None,
        created_at_ms: 1,
    }).unwrap();
    store.upsert_item(&LibraryItem {
        id: target_item_id.clone(),
        kind: LibraryItemKind::Series,
        title: title.into(),
        season,
        poster_path: None,
        created_at_ms: 2,
    }).unwrap();
    DuplicateSeriesSeed { source_item_id, target_item_id }
}

pub fn add_episode(
    engine: &Engine,
    item_id: &str,
    index: u32,
    title: &str,
    file_name: &str,
    position_ms: i64,
) -> LibraryEpisode {
    let media = engine.media_dir().join(file_name);
    fs::write(&media, b"x").unwrap();
    let store = LibraryStore::open(
        &engine.media_dir().parent().unwrap().join("library.db"),
    ).unwrap();
    let ep = LibraryEpisode {
        id: Uuid::new_v4().to_string(),
        item_id: item_id.into(),
        index,
        title: title.into(),
        file_path: media.to_string_lossy().into(),
        duration_ms: Some(10_000),
        position_ms,
        source_url: None,
    };
    store.upsert_episode(&ep).unwrap();
    ep
}
```

- [ ] **Step 2: 写失败测试 L9b-5..L9b-9**

```rust
#[path = "support/library_merge_seed.rs"]
mod library_merge_seed;

#[test]
fn merge_moves_episodes_and_deletes_source_shell() {
    let dir = tempdir().unwrap();
    let mut engine = Engine::open(dir.path()).unwrap();
    let seed = library_merge_seed::seed_duplicate_series(&engine, "示意剧", Some(1));
    library_merge_seed::add_episode(&engine, &seed.source_item_id, 1, "源1", "s1.mp4", 0);
    library_merge_seed::add_episode(&engine, &seed.source_item_id, 2, "源2", "s2.mp4", 0);

    engine
        .merge_library_items(&seed.source_item_id, &seed.target_item_id, false)
        .unwrap();

    let items = engine.list_library().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].id, seed.target_item_id);
    let eps = engine.list_episodes(&seed.target_item_id).unwrap();
    assert_eq!(eps.len(), 2);
}

#[test]
fn merge_idx_conflict_keeps_target_progress() {
    let dir = tempdir().unwrap();
    let mut engine = Engine::open(dir.path()).unwrap();
    let seed = library_merge_seed::seed_duplicate_series(&engine, "示意剧", Some(1));
    library_merge_seed::add_episode(&engine, &seed.target_item_id, 1, "目标1", "t1.mp4", 5000);
    library_merge_seed::add_episode(&engine, &seed.source_item_id, 1, "源1", "s1.mp4", 100);

    engine
        .merge_library_items(&seed.source_item_id, &seed.target_item_id, false)
        .unwrap();

    let eps = engine.list_episodes(&seed.target_item_id).unwrap();
    assert_eq!(eps.len(), 1);
    assert_eq!(eps[0].position_ms, 5000);
    assert!(engine.media_dir().join("s1.mp4").exists());
}

#[test]
fn merge_delete_orphan_files_removes_conflict_source_file() {
    // 同上 setup，merge(..., true)，assert !s1.mp4.exists() && t1.mp4.exists()
}

#[test]
fn merge_rejects_mismatched_title_season_or_single() {
    // Single + Series → InvalidArg；不同 season → InvalidArg；source==target → InvalidArg
}

#[test]
fn merge_aborts_db_when_orphan_delete_fails() {
    // delete_orphan_files=true + orphan 路径越界 → InvalidArg；两壳仍在
}
```

- [ ] **Step 3: 运行测试确认失败**

Run: `cargo test --manifest-path engine/Cargo.toml merge_ -- --nocapture`  
Expected: FAIL

- [ ] **Step 4: 实现 merge 模块**

```rust
// engine/src/library/merge.rs — 核心逻辑概要
pub(crate) fn merge_items(
    library: &LibraryStore,
    media_dir: &Path,
    source: &LibraryItem,
    target: &LibraryItem,
    delete_orphan_files: bool,
) -> Result<(), EngineError> {
    validate_merge_pair(source, target)?;
    let source_eps = library.list_episodes(&source.id)?;
    if source_eps.is_empty() {
        return Err(EngineError::InvalidArg("source series has no episodes".into()));
    }

    let mut migrate = Vec::new();
    let mut orphans = Vec::new();
    for ep in &source_eps {
        if library.get_episode_by_item_index(&target.id, ep.index)?.is_some() {
            orphans.push(ep.clone());
        } else {
            migrate.push(ep.clone());
        }
    }
    // delete orphan files if requested (ensure_path_in_media_dir + delete_files)
    let tx = library.conn().unchecked_transaction()?;
    for ep in &migrate {
        reassign_episode_item_in_tx(&tx, &ep.id, &target.id)?;
    }
    for ep in &orphans {
        delete_episode_in_tx(&tx, &ep.id)?;
    }
    delete_item_in_tx(&tx, &source.id)?;
    tx.commit()?;
    Ok(())
}
```

```rust
// engine/src/engine.rs
pub fn merge_library_items(
    &mut self,
    source_item_id: &str,
    target_item_id: &str,
    delete_orphan_files: bool,
) -> Result<(), EngineError> {
    if source_item_id == target_item_id {
        return Err(EngineError::InvalidArg("source and target must differ".into()));
    }
    let source = self.library.get_item(source_item_id)?;
    let target = self.library.get_item(target_item_id)?;
    let source_eps = self.library.list_episodes(source_item_id)?;
    let orphan_ids: Vec<String> = source_eps
        .iter()
        .filter(|ep| {
            self.library
                .get_episode_by_item_index(target_item_id, ep.index)
                .ok()
                .flatten()
                .is_some()
        })
        .map(|ep| ep.id.clone())
        .collect();
    self.finalize_lan_for_episodes(&orphan_ids)?;
    crate::library::merge::merge_items(
        &self.library,
        &self.media_dir(),
        &source,
        &target,
        delete_orphan_files,
    )
}
```

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

### Task 4: C API + FFI 测试

**Files:**
- Modify: `engine/ffi/src/sync_dispatch.rs`
- Modify: `engine/ffi/src/lib.rs`
- Modify: `app/rust/src/sync_dispatch.rs`（符号链接 crate 同源，改 ffi 即可）
- Modify: `app/rust/src/lib.rs`
- Create: `engine/ffi/tests/library_rename_merge_ffi_test.rs`

**Interfaces:**
- Consumes: Task 2–3 的 Engine 公开方法
- Produces: `engine_rename_library_item`、`engine_rename_episode`、`engine_merge_library_items`

- [ ] **Step 1: 写失败 FFI 测试 F9b-1、F9b-2**

```rust
// engine/ffi/tests/library_rename_merge_ffi_test.rs
use video_sniffing_engine_ffi::sync_dispatch::{
    engine_merge_library_items, engine_rename_episode, engine_rename_library_item,
};

#[test]
fn rename_and_merge_ffi_ok_json() {
    // seed via Engine + duplicate helper, then FFI rename + merge, assert JSON ok
}

#[test]
fn rename_ffi_invalid_title_returns_error_json() {
    let ptr = unsafe { engine_rename_library_item(handle, item_id.as_ptr(), blank.as_ptr()) };
    let v: serde_json::Value = serde_json::from_str(json).unwrap();
    assert_ne!(v["ok"], true);
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cargo test --manifest-path engine/Cargo.toml -p video_sniffing_engine_ffi library_rename_merge_ffi -- --nocapture`  
Expected: FAIL — symbol not found

- [ ] **Step 3: 实现 FFI**

```rust
#[no_mangle]
pub unsafe extern "C" fn engine_rename_library_item(
    handle: *mut EngineHandle,
    item_id: *const c_char,
    title: *const c_char,
) -> *mut c_char {
    let item_id = match parse_c_str(item_id, "item_id") { /* ... */ };
    let title = match parse_c_str(title, "title") { /* ... */ };
    ffi_call(handle, |engine| engine.rename_library_item(&item_id, &title).map(|_| ()))
}

#[no_mangle]
pub unsafe extern "C" fn engine_rename_episode(/* 同上 */) -> *mut c_char { /* ... */ }

#[no_mangle]
pub unsafe extern "C" fn engine_merge_library_items(
    handle: *mut EngineHandle,
    source_item_id: *const c_char,
    target_item_id: *const c_char,
    delete_orphan_files: u8,
) -> *mut c_char {
    let delete_orphan_files = delete_orphan_files != 0;
    ffi_call_mut(handle, |engine| {
        engine
            .merge_library_items(&source, &target, delete_orphan_files)
            .map(|_| ())
    })
}
```

在 `engine/ffi/src/lib.rs` 的 `pub use sync_dispatch::{ ... }` 追加三个符号。

- [ ] **Step 4: 运行 FFI 测试**

Run: `cargo test --manifest-path engine/Cargo.toml -p video_sniffing_engine_ffi library_rename_merge_ffi -- --nocapture`  
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add engine/ffi/src/sync_dispatch.rs engine/ffi/src/lib.rs engine/ffi/tests/library_rename_merge_ffi_test.rs
git commit -m "feat(ffi): 暴露片库重命名与合并 C API"
```

---

# Phase 9b-5 — Flutter Engine 层

### Task 5: Bindings / Host / Repository / Fake

**Files:**
- Modify: `app/lib/engine/native_bindings.dart`
- Modify: `app/lib/engine/engine_host.dart`
- Modify: `app/lib/providers/engine_repository.dart`
- Modify: `app/test/fakes/fake_engine_repository.dart`

**Interfaces:**
- Consumes: Task 4 FFI 符号
- Produces: `EngineRepository.renameLibraryItem`、`renameEpisode`、`mergeLibraryItems`

- [ ] **Step 1: 扩展 native_bindings.dart**

```dart
typedef EngineRenameLibraryItemNative = Pointer<Utf8> Function(
  Pointer<Void> handle,
  Pointer<Utf8> itemId,
  Pointer<Utf8> title,
);
typedef EngineRenameLibraryItem = Pointer<Utf8> Function(
  Pointer<Void> handle,
  Pointer<Utf8> itemId,
  Pointer<Utf8> title,
);
// 同理 engineRenameEpisode、engineMergeLibraryItems(source, target, deleteOrphanFiles: int)
```

- [ ] **Step 2: 扩展 EngineHost**

```dart
void renameLibraryItem(String itemId, String title) {
  _withUtf8(itemId, (itemIdPtr) {
    _withUtf8(title, (titlePtr) {
      _callSyncVoid(
        (handle) => _bindings.engineRenameLibraryItem(handle, itemIdPtr, titlePtr),
      );
    });
  });
}

void mergeLibraryItems(
  String sourceItemId,
  String targetItemId, {
  bool deleteOrphanFiles = false,
}) { /* engineMergeLibraryItems(..., deleteOrphanFiles ? 1 : 0) */ }
```

- [ ] **Step 3: 扩展 EngineRepository 抽象与 Fake**

Fake 记录 `lastRenamedItemId`、`lastMergedSourceId`、`lastMergedTargetId`、`lastMergeDeleteOrphanFiles`。

- [ ] **Step 4: 验证 analyze**

Run: `cd app && dart analyze lib/engine lib/providers test/fakes`  
Expected: No issues

- [ ] **Step 5: Commit**

```bash
git add app/lib/engine/native_bindings.dart app/lib/engine/engine_host.dart app/lib/providers/engine_repository.dart app/test/fakes/fake_engine_repository.dart
git commit -m "feat(app): EngineHost 接线片库重命名与合并 FFI"
```

---

# Phase 9b-6 — RenameDialog 组件

### Task 6: `RenameDialog` + W9b-1

**Files:**
- Create: `app/lib/features/library/widgets/rename_dialog.dart`
- Create: `app/test/rename_dialog_test.dart`

**Interfaces:**
- Produces: `showRenameDialogResult(context, {required String initialTitle}) -> Future<String?>`

- [ ] **Step 1: 写失败 widget 测试 W9b-1**

```dart
testWidgets('RenameDialog returns trimmed title on save', (tester) async {
  String? result;
  await tester.pumpWidget(MaterialApp(
    home: Builder(builder: (context) {
      return ElevatedButton(
        onPressed: () async {
          result = await showRenameDialogResult(context, initialTitle: '  旧  ');
        },
        child: const Text('open'),
      );
    }),
  ));
  await tester.tap(find.text('open'));
  await tester.pumpAndSettle();
  await tester.enterText(find.byKey(const Key('rename_dialog_field')), '  新名  ');
  await tester.tap(find.widgetWithText(FilledButton, '保存'));
  await tester.pumpAndSettle();
  expect(result, '新名');
});
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cd app && flutter test test/rename_dialog_test.dart`  
Expected: FAIL

- [ ] **Step 3: 实现 RenameDialog**

```dart
class RenameDialog extends StatefulWidget { /* TextField key: rename_dialog_field */ }

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

保存时 `Navigator.pop(context, controller.text.trim())`；空标题禁用保存按钮。

- [ ] **Step 4: 运行测试确认通过**

Run: `cd app && flutter test test/rename_dialog_test.dart`  
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add app/lib/features/library/widgets/rename_dialog.dart app/test/rename_dialog_test.dart
git commit -m "feat(app): 添加片库重命名对话框组件"
```

---

# Phase 9b-7 — 详情页与分集重命名 UI

### Task 7: `LibraryDetailScreen` + `EpisodeTile` + W9b-2

**Files:**
- Modify: `app/lib/features/library/library_detail_screen.dart`
- Modify: `app/lib/features/library/widgets/episode_tile.dart`
- Create: `app/test/library_rename_merge_screen_test.dart`（先写 W9b-2）

**Interfaces:**
- Consumes: Task 5 Repository；Task 6 `showRenameDialogResult`

- [ ] **Step 1: 写失败测试 W9b-2**

```dart
testWidgets('rename item from detail menu invalidates library', (tester) async {
  // Fake 单条目，打开详情，菜单 → 重命名 → 输入 → 保存
  expect(fake.lastRenamedItemId, 'item-1');
  expect(fake.lastRenamedTitle, '新标题');
  expect(find.text('已重命名'), findsOneWidget);
});
```

- [ ] **Step 2: 扩展 EpisodeTile**

```dart
final VoidCallback? onRename;
// PopupMenu 增 'rename' → '重命名'，在 'delete' 之前
```

- [ ] **Step 3: 扩展 LibraryDetailScreen 菜单**

```dart
itemBuilder: (context) {
  final candidates = /* isMergeCandidate 过滤 */;
  return [
    const PopupMenuItem(value: 'rename', child: Text('重命名')),
    if (candidates.isNotEmpty)
      const PopupMenuItem(value: 'merge', child: Text('合并到…')),
    const PopupMenuItem(value: 'delete', child: Text('删除')),
  ];
}
```

`_onMenuSelected` 增 `'rename'` 分支调用 `showRenameDialogResult` → `repo.renameLibraryItem`。

- [ ] **Step 4: 运行 widget 测试**

Run: `cd app && flutter test test/library_rename_merge_screen_test.dart --name rename`  
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add app/lib/features/library/library_detail_screen.dart app/lib/features/library/widgets/episode_tile.dart app/lib/features/library/merge_candidates.dart app/test/library_rename_merge_screen_test.dart
git commit -m "feat(app): 片库详情与分集重命名 UI"
```

---

# Phase 9b-8 — 合并 UI

### Task 8: Merge 对话框 + 流程 + W9b-3、W9b-4

**Files:**
- Create: `app/lib/features/library/widgets/merge_target_picker_sheet.dart`
- Create: `app/lib/features/library/widgets/confirm_merge_dialog.dart`
- Modify: `app/lib/features/library/library_detail_screen.dart`
- Modify: `app/test/library_rename_merge_screen_test.dart`

- [ ] **Step 1: 写失败测试 W9b-3、W9b-4**

```dart
testWidgets('confirm merge dialog defaults deleteOrphanFiles false', (tester) async { /* ... */ });

testWidgets('merge flow calls repo.mergeLibraryItems', (tester) async {
  // 两个同 title+season Series fake items，走 merge 菜单
  expect(fake.lastMergedSourceId, 'source');
  expect(fake.lastMergeDeleteOrphanFiles, isFalse);
  expect(find.text('已合并'), findsOneWidget);
});
```

- [ ] **Step 2: 实现 ConfirmMergeDialog**

```dart
class ConfirmMergeDialog extends StatefulWidget {
  const ConfirmMergeDialog({
    required this.targetTitle,
    required this.episodeCount,
    this.deleteOrphanFilesDefault = false,
  });
}
// 复选框：删除被丢弃分集的本地缓存文件，默认 false
```

- [ ] **Step 3: 实现 MergeTargetPickerSheet**

```dart
Future<LibraryItem?> showMergeTargetPicker(
  BuildContext context, {
  required List<LibraryItem> candidates,
}) {
  return showModalBottomSheet<LibraryItem>(
    context: context,
    builder: (_) => ListView(
      children: [
        for (final item in candidates)
          ListTile(
            key: Key('merge_target_${item.id}'),
            title: Text(item.title),
            onTap: () => Navigator.pop(context, item),
          ),
      ],
    ),
  );
}
```

- [ ] **Step 4: 接线 `_onMenuSelected` merge 分支**

```dart
if (value == 'merge') {
  final target = await showMergeTargetPicker(context, candidates: candidates);
  if (target == null || !context.mounted) return;
  final result = await showConfirmMergeDialogResult(
    context,
    targetTitle: target.title,
    episodeCount: episodes.length,
  );
  if (result == null || !result.confirmed) return;
  try {
    repo.mergeLibraryItems(item.id, target.id, deleteOrphanFiles: result.deleteOrphanFiles);
    ref.invalidate(libraryProvider);
    ScaffoldMessenger.of(context).showSnackBar(const SnackBar(content: Text('已合并')));
    context.go('/library/${target.id}');
  } on EngineException catch (e) { /* presentEngineError */ }
}
```

- [ ] **Step 5: 运行 widget 测试**

Run: `cd app && flutter test test/library_rename_merge_screen_test.dart`  
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add app/lib/features/library/widgets/merge_target_picker_sheet.dart app/lib/features/library/widgets/confirm_merge_dialog.dart app/lib/features/library/library_detail_screen.dart app/test/library_rename_merge_screen_test.dart
git commit -m "feat(app): Series 手动合并选择与确认 UI"
```

---

# Phase 9b-9 — 集成测试、CI 与文档

### Task 9: U11b + README + CI

**Files:**
- Create: `app/integration_test/library_rename_merge_test.dart`
- Modify: `.github/workflows/ci.yml`
- Modify: `README.md`
- Modify: `docs/superpowers/specs/2026-09-14-library-rename-merge-design.md`（状态改为「已定稿」）

**Interfaces:**
- Consumes: Task 3–8 全部完成

- [ ] **Step 1: 写 U11b 集成测试**

```dart
// app/integration_test/library_rename_merge_test.dart
testWidgets('U11b merge duplicate series via engine', (tester) async {
  // 使用 integration support 或 EngineHost 直接 seed 两个 Series + merge
  // 断言 list_library 剩 1 条、2 分集
}, timeout: const Timeout(Duration(minutes: 5)));
```

若 UI 路径过长，可拆：`engine_smoke` 风格直接调 FFI merge；或扩展现有 `app_ui_flow` 在片库详情走 merge 菜单（与 U11 删除风格一致）。

- [ ] **Step 2: 本地运行 U11b**

Run: `cd app && flutter test integration_test/library_rename_merge_test.dart -d macos`  
Expected: PASS

- [ ] **Step 3: 更新 CI matrix**

```yaml
# .github/workflows/ci.yml
matrix:
  suite: [engine, ui, deeplink, cast, browse, library, library_rename_merge]

# 分支内增：
elif [ "${{ matrix.suite }}" = "library_rename_merge" ]; then
  flutter test integration_test/library_rename_merge_test.dart -d macos
```

- [ ] **Step 4: 更新 README**

在「片库删除（Plan 9a）」节后追加「片库重命名与合并（Plan 9b）」：主路径说明 + U11b 门禁命令。

- [ ] **Step 5: 全量验证**

```bash
cargo fmt --manifest-path engine/Cargo.toml --all -- --check
cargo test --manifest-path engine/Cargo.toml
cargo clippy --manifest-path engine/Cargo.toml --all-targets --all-features -- -D warnings
cd app && flutter test
cd app && flutter test integration_test/library_rename_merge_test.dart -d macos
```

Expected: 全部 PASS

- [ ] **Step 6: Commit**

```bash
git add app/integration_test/library_rename_merge_test.dart .github/workflows/ci.yml README.md docs/superpowers/specs/2026-09-14-library-rename-merge-design.md
git commit -m "test: 添加 Plan 9b U11b 集成门禁与 CI 矩阵"
```

---

## Spec Self-Review（计划自检）

| 规格要求 | 对应 Task |
|----------|-----------|
| L9b-1..L9b-4 重命名 | Task 2 |
| L9b-5..L9b-9 合并 | Task 3 |
| F9b-1、F9b-2 FFI | Task 4 |
| W9b-1..W9b-4 Widget | Task 6–8 |
| U11b 集成 | Task 9 |
| Single 同步分集 title | Task 2 Step 3 |
| merge 默认不删 orphan | Task 8 ConfirmMergeDialog |
| Flutter 侧 merge 候选过滤 | Task 7 merge_candidates.dart |
| TV D-pad 菜单 | Task 7（沿用 PopupMenuButton，与 9a 同） |
| README / CI | Task 9 |

**Placeholder 扫描：** 无 TBD；各 Step 含具体路径与命令。

**类型一致性：** `mergeLibraryItems(source, target, deleteOrphanFiles: bool)` 自 Task 5 起贯穿 Task 8 Fake 断言。

---

## Execution Handoff

Plan complete and saved to `docs/superpowers/plans/2026-09-14-library-rename-merge.md`.

**两种执行方式：**

1. **Subagent-Driven（推荐）** — 每个 Task 派发独立 subagent，Task 间做 review，迭代快  
2. **Inline Execution** — 本会话内用 executing-plans 批量执行，检查点暂停 review  

**你想用哪种？**
