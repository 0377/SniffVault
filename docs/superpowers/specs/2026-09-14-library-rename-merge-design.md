# 片库重命名与 Series 手动合并设计（Plan 9b）

**日期**: 2026-09-14  
**状态**: 已定稿（2026-09-14 review 修订）  
**修订摘要**: Series 条目标题范围、L9b-9/10、W9b-5/6、U11b 范围、失败 SnackBar 固定  
**前置**: Plan 9a 片库删除（已合并 main，`v0.1.1`）  
**父规格**: `docs/superpowers/specs/2026-09-08-library-management-design.md`（§2.3–2.4、§3.2–3.3、§5.1）  
**后续**: Plan 9c 海报抓取；Plan 9d 设置目录选择器 + 失败任务改 URL 重试  
**实现计划**: `docs/superpowers/plans/2026-09-14-library-rename-merge.md`

---

## 1. 目标

Plan 9a 使用户能删除片库条目，但误下标题、重复 Series 壳仍无法自救。本规格交付 **v0.2 片库管理的第二刀**：重命名展示标题，以及将误建的重复 Series 合并到同 `title + season` 的目标条目。

**Plan 9b 验收一句话：** 用户可在片库详情重命名条目（Single 同步更新唯一分集标题）与 Series 分集标题；可将两个同剧名同季数的 Series 壳合并为一个，idx 冲突时保留目标分集及其播放进度；重命名与合并不修改磁盘文件名；Engine 为唯一写入口，含 FFI、Flutter UI 与自动化测试。

### 1.1 范围决策

| 纳入 Plan 9b | 排除（留给 9c / 9d / v0.3+） |
|--------------|-------------------------------|
| `rename_library_item` / `rename_episode` | 海报抓取与列表展示（9c） |
| `merge_library_items`（Series 手动合并） | 设置 `media_dir` 目录选择器（9d） |
| Engine + FFI + Flutter UI + 测试 | 失败任务改 URL 重试（9d） |
| TV D-pad 可聚焦的新菜单项 | 批量多选、跨季合并 |
| 可选 `v0.1.2` tag（功能预览） | 磁盘文件重命名、数据目录迁移 |

**PM 定稿：9b 一次交付**（重命名 + 合并同批 PR），不拆成两刀。理由：同一 UI 入口与 Engine 写路径；只做重命名会长期留下重复 Series 壳，与 v0.2「片库可管理」叙事不符。

**发版节奏：**

1. 9b 合并后可打 **`v0.1.2` tag**（可选，README 标注 v0.2 预览）。
2. 完整 **`v0.2.0` tag** 仍待 9c + 9d 全部验收（与母规格一致）。

### 1.2 产品原则

- **Engine 是唯一写入口**：重命名、合并均经 `Engine` 公开方法；`LibraryStore` 保持 crate-private。
- **重命名不改磁盘路径**：仅更新 SQLite 展示字段 `library_items.title`、`library_episodes.title`；`file_path` 不变。
- **Single 条目重命名同步分集标题**：`kind == Single` 且 **恰有 1 个分集** 时，`rename_library_item` 在同一事务内将唯一分集的 `title` 设为相同字符串（详情页无分集行菜单，避免标题不一致）。
- **Series 条目标题与分集标题分离**：`kind == Series` 时，`rename_library_item` **仅** 更新 `library_items.title`，**不** 批量修改各分集 `library_episodes.title`；分集改名走 `rename_episode`。
- **合并单事务**：分集迁移、`item_id` 更新、源壳删除在 **单一 SQLite 事务** 内完成；`delete_orphan_files=true` 时磁盘删除在事务提交 **之前** 完成，任一步失败则 DB 不变。
- **idx 冲突保留目标**：目标 Series 已有同 `idx` 分集时，保留 **目标** 行（含 `position_ms`、`file_path`）；丢弃源分集 DB 行；源文件是否删除由 `delete_orphan_files` 决定。
- **LAN 安全**：对被删除或不再有效的分集 ID 调用与 9a 相同的 LAN 清理（revoke token + 匹配时清 `active_cast`）；`CastMetadata` 仍不含 `source_url`。
- **自动合并不变**：入库路径 `register_completed_episode` + `find_series_by_title_season` 行为不改；本功能仅补 **用户手动** 合并重复壳。

---

## 2. 用户流程

### 2.1 重命名条目

1. 片库 → 条目详情 → AppBar **更多**（`⋮`）→ **重命名**。
2. 对话框预填当前标题 → 用户编辑 → **保存**。
3. 成功：SnackBar「已重命名」→ AppBar 标题与列表刷新（`ref.invalidate(libraryProvider)`）。
4. 失败：`presentEngineError` + SnackBar 展示引擎错误（对齐 9a 删除）；**不** 显示成功；对话框可关闭或保留，但 **必须** SnackBar。

**Single**：仅 AppBar 菜单提供重命名（与 9a 一致，正文为播放/投送按钮，无分集行）。

**TV（Leanback）**：与 9a 删除相同，`PopupMenuButton` 须 D-pad 可聚焦；**不**在 TV 网格项上提供重命名。

### 2.2 重命名分集（仅 Series）

1. Series 详情 → 分集行 **更多** → **重命名** → 对话框 → 保存。
2. 成功/失败反馈同 §2.1。
3. Single 或仅 1 集展示为 Single 布局时 **不** 提供分集行重命名（条目级重命名已覆盖）。

### 2.3 Series 手动合并

**场景**：用户误建两个 `item_id` 不同、但 `title + season` 相同的 Series 壳（自动入库未合并的脏数据，或历史 bug）。

1. **源** Series 详情 → AppBar **更多** → **合并到…**（仅当片库中存在 ≥1 个其他同 `title + season` 的 Series 时显示该项）。
2. 底部 sheet / 对话框列出候选目标（排除自身；仅 `kind == Series` 且 `title`、`season` 与源完全一致）。
3. 用户选择目标 → 确认对话框：
   - 标题：`合并到「{目标标题}」？`
   - 正文：`将把本条目下 {N} 个分集移至目标；本条目将被删除。若集号重复，保留目标分集及播放进度。`
   - 复选框（默认 **不勾选**）：`删除被丢弃分集的本地缓存文件`
   - 按钮：取消 / **合并**
4. 成功：SnackBar「已合并」→ `context.go('/library/{targetItemId}')` 或 `pop` 至片库列表并刷新。
5. 失败：SnackBar 错误；源条目与分集不变。

**不在此流程**：跨季合并、Single 合并、改 title/season 作为合并条件。

---

## 3. Engine API

### 3.1 重命名

```rust
/// 更新片库条目标题。Single 时同步更新其唯一分集 title。
pub fn rename_library_item(&self, item_id: &str, title: &str) -> Result<(), EngineError>;

/// 更新分集展示标题；不修改 file_path。
pub fn rename_episode(&self, episode_id: &str, title: &str) -> Result<(), EngineError>;
```

**校验（两者共用 `validate_display_title`）**：

- `title.trim()` 非空；
- trim 后长度 `<= 512`；
- 持久化存 **trim 后** 字符串。

**`rename_library_item` 算法**：

1. 加载条目；不存在 → `NotFound`。
2. 校验标题。
3. 调用 `LibraryStore::rename_library_item_titles(item_id, title, single_episode_id)` 在 **单一 SQLite 事务** 内完成步骤 3 的 UPDATE（Engine **不** 直接持有 `conn()` 或开事务）。

**Series 与 Single 行为差异**：

| `kind` | `rename_library_item` 写入范围 |
|--------|--------------------------------|
| `Single`（恰 1 分集） | `library_items.title` + 唯一分集 `title` |
| `Series` | 仅 `library_items.title` |
| 其它 / 脏数据（分集数 ≠ 1 的 Single） | 仅 `library_items.title`（不猜测同步分集） |

**`rename_episode` 算法**：

1. 加载分集；不存在 → `NotFound`。
2. 校验标题。
3. `UPDATE library_episodes SET title=? WHERE id=?`（单语句，隐式原子）。

### 3.2 合并

```rust
/// 将源 Series 全部分集迁入目标 Series，并删除源条目壳。
/// delete_orphan_files: idx 冲突被丢弃的源分集，是否删除其 media_dir 内已校验文件。
pub fn merge_library_items(
    &mut self,
    source_item_id: &str,
    target_item_id: &str,
    delete_orphan_files: bool,
) -> Result<(), EngineError>;
```

**校验**：

| 条件 | 错误 |
|------|------|
| `source_item_id == target_item_id` | `InvalidArg` |
| 源或目标不存在 | `NotFound` |
| 任一 `kind != Series` | `InvalidArg` |
| `title` 或 `season` 不一致（含 `None` 与 `None`） | `InvalidArg` |
| 源条目无分集 | `InvalidArg`（空壳无需合并，应直接删） |

**算法**：

1. 加载源/目标条目与源侧全部分集列表。
2. 对每个源分集 `ep`：
   - 若目标已有 `get_episode_by_item_index(target_id, ep.index)`：
     - 标记 `ep` 为 **orphan**（冲突：保留目标行）；
     - 对 orphan 的 `ep.id` 执行 LAN 清理（`finalize_lan_for_episodes`）。
   - 否则：将 `ep` 加入 **migrate** 列表。
3. 若 `delete_orphan_files`：对每个 orphan 的 `file_path` 经 `ensure_path_in_media_dir` 校验后 `remove_file`（不存在视为成功）；任失败 → 返回错误，**不修改 DB**。
4. `BEGIN` 事务：
   - 对每个 migrate 分集：`UPDATE library_episodes SET item_id=? WHERE id=?`（`episode_id` 不变，`idx` 不变）；
   - 对每个 orphan：`DELETE FROM library_episodes WHERE id=?`；
   - `DELETE FROM library_items WHERE id=?`（源 `source_item_id`）；
   - `COMMIT`。
5. 若源条目 `poster_path` 非空且目标 `poster_path` 为空：可选在同事务内 `UPDATE library_items SET poster_path=源.poster_path WHERE id=target`（**Nice-to-have**；9c 前可省略，本 spec 不阻塞 9b 验收）。

**合并后不自动重排 idx**；目标侧分集集号集合为并集，冲突时已定义保留策略。

### 3.3 LibraryStore 增量

```rust
pub fn update_item_title(&self, item_id: &str, title: &str) -> Result<(), EngineError>;
pub fn update_episode_title(&self, episode_id: &str, title: &str) -> Result<(), EngineError>;
/// Single 条目重命名：同事务更新 item title；若 `single_episode_id` 为 Some 则同步该分集 title。
pub fn rename_library_item_titles(
    &self,
    item_id: &str,
    title: &str,
    single_episode_id: Option<&str>,
) -> Result<(), EngineError>;
/// merge 事务门面（merge 模块调用，不暴露 SQL 给 Engine 以外）
pub(crate) fn apply_merge_in_tx(
    &self,
    migrate: &[(String, String)], // (episode_id, new_item_id)
    orphan_episode_ids: &[String],
    source_item_id: &str,
) -> Result<(), EngineError>;
/// 同事务批量更新分集 item_id（merge 内部使用）
pub(crate) fn reassign_episode_item_in_tx(
    tx: &Transaction,
    episode_id: &str,
    new_item_id: &str,
) -> Result<(), EngineError>;
```

不新增 `list_merge_candidates` Store API：Flutter 从 `libraryProvider` 内存列表过滤（条目数规模下足够）。

---

## 4. FFI

| C API | 参数 | 响应 |
|-------|------|------|
| `engine_rename_library_item` | `item_id`, `title` | `{ "ok": true }` |
| `engine_rename_episode` | `episode_id`, `title` | `{ "ok": true }` |
| `engine_merge_library_items` | `source_item_id`, `target_item_id`, `delete_orphan_files` (`uint8_t` 0/1) | `{ "ok": true }` |

错误经既有 `ffi_call` / `EngineException` 路径返回 Flutter。Dart `EngineHost` / `EngineRepository` / `FakeEngineRepository` 同步扩展。

---

## 5. Flutter UI

### 5.1 菜单扩展

| 位置 | 菜单项 | 可见条件 |
|------|--------|----------|
| `LibraryDetailScreen` AppBar | 重命名 | 条目存在 |
| `LibraryDetailScreen` AppBar | 合并到… | `kind == Series` 且存在 ≥1 个其他同 title+season 的 Series |
| `LibraryDetailScreen` AppBar | 删除 | 已有（9a） |
| `EpisodeTile` | 重命名 | 与 9a 删除一致：`kind == Series` 且 `episodes.length >= 2` |
| `EpisodeTile` | 删除此分集 | 已有（9a） |

菜单顺序建议：**重命名** → **合并到…** → **删除**（destructive 置底）。

### 5.2 新组件

| 组件 | 路径 | 职责 |
|------|------|------|
| `RenameDialog` | `features/library/widgets/rename_dialog.dart` | 单行 `TextField`，保存/取消；测试 Key `rename_dialog_field` |
| `MergeTargetPickerSheet` | `features/library/widgets/merge_target_picker_sheet.dart` | 候选列表；项 Key `merge_target_{itemId}` |
| `ConfirmMergeDialog` | `features/library/widgets/confirm_merge_dialog.dart` | 正文 + `delete_orphan_files` 复选框默认 **false** |

对外 API 建议：

- `showRenameDialogResult(context, initialTitle:)` → `String?`
- `showMergeTargetPicker(context, candidates:)` → `LibraryItem?`
- `showConfirmMergeDialogResult(...)` → `({bool confirmed, bool deleteOrphanFiles})?`

### 5.3 候选过滤（Flutter）

```dart
bool isMergeCandidate(LibraryItem source, LibraryItem other) =>
  other.id != source.id &&
  other.kind == LibraryItemKind.series &&
  source.kind == LibraryItemKind.series &&
  other.title == source.title &&
  other.season == source.season;
```

从 `ref.read(libraryProvider)` 过滤，**不**新增 Engine 列表 API。

### 5.4 错误与刷新

- 成功：`ref.invalidate(libraryProvider)`；合并后导航至目标详情或片库列表。
- 失败：`try/catch (EngineException)` → `presentEngineError` + SnackBar（对齐 9a 删除）。

---

## 6. 错误处理

| 场景 | 行为 |
|------|------|
| 条目/分集不存在 | `EngineError::NotFound` |
| 标题为空或超长 | `InvalidArg` |
| 合并 source == target | `InvalidArg` |
| 合并 kind 非 Series 或 title/season 不匹配 | `InvalidArg` |
| 源 Series 无分集 | `InvalidArg` |
| `delete_orphan_files=true` 且 orphan 路径越界 | `InvalidArg`，DB 不变 |
| orphan 文件删除失败 | 错误含路径；DB 不变 |
| 正在播放的分集被 merge 丢弃 | 允许；播放器已有「找不到分集」处理 |
| 正在投送的 orphan 分集 | LAN 清理后 TV 拉流失败 |

---

## 7. 测试

### 7.1 Engine

| ID | 场景 |
|----|------|
| L9b-1 | `rename_library_item` 持久化；重启 Engine 后标题仍在 |
| L9b-2 | Single 重命名条目后唯一分集 title 同步 |
| L9b-3 | `rename_episode` 仅改分集 title，`file_path` 不变 |
| L9b-3b | Series 的 `rename_library_item` 仅改条目标题，各分集 title 不变 |
| L9b-4 | 空标题 / 超长标题 → `InvalidArg` |
| L9b-5 | 合并：源 2 集迁入空目标，源壳删除，共 2 集 |
| L9b-6 | idx 冲突：目标第 1 集 progress=5000，源第 1 集 progress=100 → 合并后仍为 5000 |
| L9b-7 | idx 冲突 + `delete_orphan_files=true` → 源冲突文件删除，目标文件保留 |
| L9b-8 | title/season 不匹配 / Single 参与 → `InvalidArg` |
| L9b-9 | `delete_orphan_files=true` 且 orphan 路径越界 → `InvalidArg`，两壳与分集 DB 不变 |
| L9b-10 | 投送 orphan 分集后 merge → `active_cast` 清空，可对其它分集再次 `cast_episode` |

### 7.2 FFI

| ID | 场景 |
|----|------|
| F9b-1 | 三个新 API 成功路径 JSON |
| F9b-2 | 非法 title 错误 JSON |

### 7.3 Flutter Widget

| ID | 场景 |
|----|------|
| W9b-1 | `RenameDialog` 保存回调传入 trim 后标题 |
| W9b-2 | 详情菜单重命名成功 invalidate 列表 |
| W9b-3 | 合并确认默认不删 orphan 文件 |
| W9b-4 | 合并成功调用 `mergeLibraryItems(source, target, deleteOrphanFiles: false)` |
| W9b-5 | 重命名失败 SnackBar，Fake 未被调用或条目未变 |
| W9b-6 | 分集行菜单重命名调用 `renameEpisode` |

### 7.4 集成

| ID | 场景 |
|----|------|
| U11b | **Engine 级**（经 `EngineHost` / FFI）：seed 两个同 title+season Series 壳各 1 集 → `mergeLibraryItems` → 片库剩 1 条、2 分集；**不要求**走完整 UI 向导 |

CI：在既有 `flutter-integration` 矩阵中新增 `library_rename_merge` suite（或扩展现有 `library` job），与 U11 并列本地门禁。

### 7.5 验证命令

```bash
cargo fmt --manifest-path engine/Cargo.toml --all -- --check
cargo test --manifest-path engine/Cargo.toml
cargo clippy --manifest-path engine/Cargo.toml --all-targets --all-features -- -D warnings
cd app && flutter test
cd app && flutter test integration_test/library_rename_merge_test.dart -d macos
```

---

## 8. 文件变更摘要

| 路径 | 变更 |
|------|------|
| `engine/src/library/store.rs` | `update_item_title`、`update_episode_title`、merge 事务辅助 |
| `engine/src/library/merge.rs` | 合并算法（新建，`pub(crate)`） |
| `engine/src/engine.rs` | 三个公开 API |
| `engine/tests/library_rename_merge.rs` | L9b-* |
| `engine/ffi/src/sync_dispatch.rs` | 三个 C API |
| `engine/ffi/tests/library_rename_merge_ffi_test.rs` | F9b-* |
| `app/lib/engine/*` | bindings / host / repository |
| `app/lib/features/library/widgets/rename_dialog.dart` | 新建 |
| `app/lib/features/library/widgets/merge_target_picker_sheet.dart` | 新建 |
| `app/lib/features/library/widgets/confirm_merge_dialog.dart` | 新建 |
| `app/lib/features/library/library_detail_screen.dart` | 菜单与流程 |
| `app/lib/features/library/widgets/episode_tile.dart` | 分集重命名菜单 |
| `app/test/*` | W9b-* |
| `app/integration_test/library_rename_merge_test.dart` | U11b |
| `README.md` | Plan 9b / U11b 门禁说明 |
| `.github/workflows/ci.yml` | integration matrix |

---

## 9. 与 AGENTS.md 对齐检查

- [x] 经 `Engine` 写入片库
- [x] 多行持久化在单事务（merge；Single 重命名条目+分集）
- [x] `delete_orphan_files` 删文件时 `canonicalize` + `media_dir` 约束
- [x] 分集唯一键 `(item_id, idx)`；冲突保留目标
- [x] Series 合并键 `title + season`（含 `None`）
- [x] LAN 元数据不含 `source_url`
- [x] 不提供站外破解能力

---

## 10. 不在本 Plan 范围

- 海报抓取与 `poster_path` 写入（9c）
- 合并时自动迁移/合并海报（Nice-to-have 已标注可选）
- 修改 `season` 或条目 `kind`
- 片库搜索、排序、批量操作
- 磁盘文件重命名或路径批量重写
- 失败任务改 URL 重试（9d）

---

## 11. PM 定稿摘要

| 决策项 | 结论 |
|--------|------|
| v0.2 下一刀 | **Plan 9b 重命名 + 合并，一次交付** |
| Single 重命名 | 条目 title 与唯一分集 title **同步** |
| 合并冲突 | **保留目标** 分集与 `position_ms` |
| 合并默认删 orphan 文件 | **否**（复选框默认不勾选） |
| 合并候选列表 | **Flutter 侧过滤**，不增 Store 列表 API |
| 9b 发版 | 可选 **`v0.1.2`**；**`v0.2.0` = 9a–9d 全部完成** |
| 下一子计划 | **9c 海报** → **9d 设置与任务** |
