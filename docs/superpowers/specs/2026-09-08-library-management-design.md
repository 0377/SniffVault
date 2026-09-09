# 片库管理与 v0.2 体验设计（Plan 9）

**日期**: 2026-09-08  
**状态**: 已审阅修订（2026-09-08 review：路径校验策略、active_cast、L9-3、TV、发版 tag）  
**前置计划**: Plan 1–8（Plan 8 已合并 main，待发 `v0.1.0` tag）  
**后续计划**: Plan 6c 桌面深链（可选）；Plan 7 安全加固补丁；SenderHttp 流式化；字幕 / PiP  
**父规格**: `docs/superpowers/specs/2026-08-11-app-ui-player-design.md`、`docs/superpowers/specs/2026-09-08-shippable-v01-design.md`  
**实现计划（9a）**: `docs/superpowers/plans/2026-09-08-library-delete.md`

---

## 1. 目标

Plan 8 交付了「可安装、可闭环」的 v0.1，但片库 **只能增不能管**：误下、重复、占磁盘时用户无法自救；列表无封面辨识度低；设置页 `media_dir` 仍为手填文本。本规格定义 **v0.2 体验迭代**，使产品从「能用」进入「用得久」。

**Plan 9 验收一句话：** 用户可在片库删除整条目或单分集（默认释放磁盘空间）；可重命名条目与分集；解析入库时自动抓取封面并在列表展示；设置页可用系统目录选择器修改缓存目录；推送 `v0.2.0` tag 前上述主路径有自动化测试覆盖。

### 1.1 范围决策

| 纳入 Plan 9（v0.2） | 留给 v0.3+ |
|---------------------|------------|
| 片库删除（条目 / 单分集，可选删文件） | Series 跨季合并 |
| 片库重命名（条目标题、分集标题） | 片库搜索 / 筛选 / 排序自定义 |
| Series 手动合并（同 `title + season` 冲突时用户选择目标） | 批量操作（多选删除） |
| 海报抓取（`og:image` 优先）+ 列表/详情展示 | 字幕、PiP |
| 设置：`media_dir` 系统目录选择器 | 数据目录整体迁移向导 |
| 任务页：失败任务「修改 URL 重试」公开 API + UI | DLNA / Chromecast |
| Engine / FFI 增量 API | iOS 自动发版 |

**不选方案 B（先做海报/设置抛光）**：视觉与配置优化无法解决「片库只增不减」的日常痛点；v0.1 用户积累内容后删除需求会快速出现。  
**不选方案 C（先做 Plan 7 技术债）**：PIN 重放、loopback 等对用户不可见；可在 9a 之后穿插小补丁，不阻塞 v0.2 主线。  
**选定方案 A（片库管理优先 → 海报 → 设置抛光）**：与 Plan 8 §1.1 已承诺的 v0.2 范围一致，且底层已有 `LibraryStore::remove_item`、 `poster_path` 字段与 `ensure_path_in_media_dir`，工程风险可控。

### 1.2 子计划拆分

| 子计划 | 交付物 | 建议顺序 |
|--------|--------|----------|
| **9a** | 片库删除（Engine + FFI + UI + 测试） | **第一刀** |
| **9b** | 重命名 + Series 手动合并 | 9a 之后 |
| **9c** | 海报抓取与展示 | 9b 可与 9a 末期并行 |
| **9d** | 设置目录选择器 + 失败任务重试 | 9c 之后 |

**发版节奏：**

1. **并行轨道**：main 上推送 `v0.1.0` tag（不阻塞 9a 开发）。
2. **9a 完成**：推送 `v0.1.1` tag（仅含片库删除；README 标注为 v0.2 功能预览亦可）。
3. **v0.2.0 tag**：9a–9d **全部**验收后打 tag（完整 v0.2 体验）。

### 1.3 产品原则

- **Engine 是唯一写入口**：删除、重命名、合并、海报路径更新均经 `Engine` 公开方法；`LibraryStore` 保持 crate-private。
- **文件删除必须防目录逃逸**：`delete_files=true` 时，仅当 `canonicalize(file_path)` 位于当前 `media_dir` 下才执行 `remove_file`；禁止字符串前缀判断。
- **仅移出片库可跳过路径校验**：`delete_files=false` 时 **不** 调用 `ensure_path_in_media_dir`；允许用户从片库移除历史脏数据（越界 `file_path`），磁盘文件原样保留。
- **默认释放空间**：删除确认对话框 **默认勾选「同时删除本地缓存文件」**；用户可取消勾选改为「仅移出片库」。
- **原子性**：数据库变更在 **单一 SQLite 事务** 内完成；`delete_files=true` 时 **先删磁盘文件、全部成功后再提交事务**；任一步失败则整操作失败、库内记录不变。
- **投送安全**：删除前撤销受影响分集的 LAN stream token；若该分集为当前 `active_cast` 会话，清理发送端投送状态（不向 TV 发 stop HTTP）。
- **海报仍仅本机**：封面文件存于 `media_dir` 下；LAN `CastMetadata` 不含海报 URL 或 `source_url`。

---

## 2. 用户流程

### 2.1 删除整条目（9a）

1. 片库 → 条目详情 → AppBar **更多**（`⋮`）→ **删除**。
2. 确认对话框：
   - 标题：`删除「{title}」？`
   - 正文：Series 显示「将删除 N 个分集」；Single 显示「将删除 1 个文件」。
   - 复选框（默认 **勾选**）：`同时删除本地缓存文件`
   - 按钮：取消 / **删除**（destructive）
3. 成功：SnackBar「已删除」→ 返回片库列表并刷新。
4. 失败：SnackBar 展示引擎错误（如文件被占用、路径不可访问）。

**TV（Leanback）**：TV 从片库网格进入 **同一** `LibraryDetailScreen`（`/library/:itemId`）。详情 AppBar 的删除菜单须支持 D-pad 焦点；另注册 `Shortcuts`：`LogicalKeyboardKey.contextMenu` 或 `LogicalKeyboardKey.enter` + 菜单键打开删除确认（与手机语义一致）。**不在** `tv_library_grid` 网格项上提供删除（避免误触）。

### 2.2 删除单个分集（9a，仅 Series）

1. Series 详情 → 分集行 **更多** → **删除此分集**。
2. 确认对话框同上（针对单集）。
3. 若删除后该 Series **无剩余分集**：自动删除空 `library_items` 行（同一事务）。
4. 若删除后仍有分集：仅删该 `library_episodes` 行。

Single 类型仅提供「删除整条目」，不提供单集删除（仅 1 集）。

### 2.3 重命名（9b）

1. 详情 AppBar **更多** → **重命名** → 对话框编辑标题 → 保存。
2. Series 分集行 **更多** → **重命名** → 编辑分集标题。
3. 重命名 **不** 修改磁盘文件名（避免破坏路径引用与投送 token）；仅更新 SQLite 展示字段。

### 2.4 Series 手动合并（9b）

**触发**：用户将新剧集下载到与已有条目相同的 `title + season`（引擎已自动合并）时无需此流程。本功能面向 **用户误建重复 Series 壳**（不同 `item_id`、相同剧名季数）：

1. 详情 AppBar **更多** → **合并到…** → 选择目标 Series（列表排除自身；仅匹配相同 `title` 与 `season`）。
2. 确认：「将把**本条目下全部分集**移至「{目标标题}」；源条目将被删除。」
3. 引擎在单事务内：迁移分集 `item_id`、按 `(item_id, idx)` 去重（冲突时保留目标分集、丢弃源分集记录但可选删源文件）、删除空源条目。

### 2.5 海报（9c）

1. **入库时**：`register_completed_*` 路径上，若任务上下文携带 `poster_url`（解析阶段从页面 `og:image` 提取，失败时不阻塞入库），则下载至 `media_dir/.posters/{item_id}.{ext}`，经 `canonicalize` 后写入 `library_items.poster_path`（**与 `file_path` 相同：存绝对路径**，读取时用 `ensure_path_in_media_dir` 校验）。
2. **展示**：片库列表 `LibraryCard`、详情 AppBar 左侧显示封面缩略图；无海报时沿用首字母/占位图标。
3. **删除条目时**：若 `poster_path` 在 `media_dir` 下且 `delete_files=true`，一并删除海报文件。

### 2.6 设置与任务（9d）

- **缓存目录**：设置页 `media_dir` 旁增加「选择文件夹」；选中的目录名须通过 `validate_media_dir`（单层相对名）。实现策略：用户选物理路径后，若不在 `data_dir` 下则 **仅取其目录名** 写入 `media_dir` 并在 `data_dir` 下 `create_dir_all`——**不搬移已有文件**（搬移归 v0.3 迁移向导）。
- **失败任务重试**：任务详情/失败行展示「重试」；调用 `Engine::retry_task(task_id, new_url: Option<&str>)` 将任务置 `Queued` 并可选更新 URL（覆盖测试辅助 `requeue_failed_task` 直写 TaskStore 的反模式）。

---

## 3. Engine API（9a–9b）

### 3.1 删除

```rust
/// 删除片库条目及其全部分集。
/// delete_files: 是否删除 media_dir 内已校验的媒体文件与海报。
pub fn remove_library_item(&mut self, item_id: &str, delete_files: bool) -> Result<(), EngineError>;

/// 删除单个分集。若为该条目最后一集，等价于 remove_library_item(item_id, delete_files)。
pub fn remove_episode(&mut self, episode_id: &str, delete_files: bool) -> Result<(), EngineError>;
```

**内部算法（`delete_files = true`）**：

1. 加载条目与全部分集；若不存在 → `NotFound`。
2. 对每个 `episode_id` 调用 LAN 清理（见下「LAN 清理」）。
3. 收集待删路径：各分集 `file_path` + 条目 `poster_path`（均经 `ensure_path_in_media_dir` 校验）；任一越界 → `InvalidArg`，**不修改 DB**。
4. 对每个路径执行 `std::fs::remove_file`；文件不存在视为成功；权限/占用错误 → 返回错误，**不修改 DB**。
5. `BEGIN` → `DELETE FROM library_items WHERE id=?`（`ON DELETE CASCADE` 清理分集）→ `COMMIT`。

**内部算法（`delete_files = false`）**：

1. 加载条目与分集。
2. LAN 清理（同上）。
3. **跳过**路径校验与磁盘删除。
4. 步骤 5 提交 DB 事务。

**`remove_episode`（非最后一集）**：步骤 2 仅针对该分集；步骤 3–4 仅该分集 `file_path`（`delete_files=true` 时）；步骤 5 为 `DELETE FROM library_episodes WHERE id=?`（单语句，隐式原子）。

**LAN 清理**（`LanService` 已初始化时，`ensure_lan` 后）：

1. `StreamTokenStore::revoke_for_episode(episode_id)`。
2. 若 `active_cast.episode_id == episode_id`（`ActiveCastSession` **新增** `episode_id` 字段，`cast_episode` 时写入），调用 `stop_cast_internal(false)` 清理发送端会话（**不**向 TV 发 HTTP stop；TV 侧拉流失败后自行结束）。

### 3.2 重命名（9b）

```rust
pub fn rename_library_item(&self, item_id: &str, title: &str) -> Result<(), EngineError>;
pub fn rename_episode(&self, episode_id: &str, title: &str) -> Result<(), EngineError>;
```

非空、`title.len() <= 512`；trim 首尾空白。

### 3.3 合并（9b）

```rust
pub fn merge_library_items(&mut self, source_item_id: &str, target_item_id: &str, delete_orphan_files: bool) -> Result<(), EngineError>;
```

- 校验：两者 `kind == Series`、相同 `title` 与 `season`（`season=None` 与 `None` 相等）。
- 分集 `item_id` 批量更新；`idx` 冲突时保留 **目标** 分集进度与文件，源分集记录删除；若 `delete_orphan_files` 则删被丢弃的源文件。
- 源条目无分集后删除源 `library_items` 行。

### 3.4 LibraryStore 增量

- `remove_episode(&self, episode_id: &str)` — 按 id 删除单行；`remove_item` 保留。
- `update_item_title` / `update_episode_title` — 单行 UPDATE（9b）。
- `count_episodes(item_id)` — 供「删最后一集」判断。

---

## 4. FFI（9a–9b）

| C API | 说明 |
|-------|------|
| `engine_remove_library_item(handle, item_id, delete_files)` | JSON 响应 `{ "ok": true }` |
| `engine_remove_episode(handle, episode_id, delete_files)` | 同上 |
| `engine_rename_library_item` / `engine_rename_episode` | 9b |
| `engine_merge_library_items` | 9b |

参数 `delete_files` 为 `bool`（FFI `uint8_t` 0/1）。错误经既有 `ffi_call` / `EngineException` 路径返回 Flutter。

---

## 5. Flutter UI（9a–9d）

### 5.1 删除入口

| 位置 | 控件 |
|------|------|
| `LibraryDetailScreen` AppBar | `PopupMenuButton`（可聚焦）→ 删除 /（9b）重命名、合并 |
| `LibraryDetailScreen`（TV） | 同上 + `Shortcuts` 打开删除菜单 |
| `EpisodeTile`（Series，≥2 集） | 行尾 `PopupMenuButton` → 删除此分集 |

确认对话框组件：`ConfirmDeleteDialog`（`features/library/widgets/`），复用 `delete_files` 默认值 `true`。对外 API：`showConfirmDeleteDialogResult` 返回 `({bool confirmed, bool deleteFiles})?`。

删除后：`ref.invalidate(libraryProvider)`；若当前在详情页且条目已删，`context.pop()`。

### 5.2 海报（9c）

- `LibraryCard`：左侧 `ClipRRect` 80×120；`posterPath != null` 时 `Image.file`。
- 解析管线：`ResolveOutcome` / 下载完成回调向 `register_completed_*` 传入可选 `poster_url`（Engine 侧拉取，非 Flutter 直写）。

### 5.3 设置（9d）

- `file_picker` 或平台目录选择（优先各端已有依赖；无则 macOS/Windows 用 `file_selector`，移动端用 SAF/DocumentPicker 选 **子目录名** 策略见 §2.6）。
- 失败任务：`TaskTile` 在 `status == failed` 时显示「重试」；可选对话框编辑 URL。

---

## 6. 错误处理

| 场景 | 行为 |
|------|------|
| 条目/分集不存在 | `EngineError::NotFound` |
| `delete_files=true` 且 `file_path` 不在 `media_dir` | `InvalidArg`，不删 DB |
| `delete_files=false` 且路径越界/脏数据 | **允许**删 DB，磁盘不动 |
| 文件删除权限/占用 | 错误信息含路径；DB 不变 |
| 正在播放的分集被删 | 允许；播放器 `getEpisode` 返回空时已有「找不到分集」页 |
| 正在投送的分集被删 | 撤销 token + 清理 `active_cast`；接收端拉流 404 |
| 重命名空标题 | `InvalidArg` |
| 合并 title/season 不匹配 | `InvalidArg` |

---

## 7. 测试

### 7.1 Engine（9a）

| ID | 场景 |
|----|------|
| L9-1 | `remove_library_item(delete_files=true)` 删 DB + 文件 |
| L9-2 | `delete_files=false` 仅删 DB，文件仍在 |
| L9-3 | `delete_files=true` 且 DB 含越界 `file_path`（经 `LibraryStore` 注入脏数据）→ `InvalidArg`，DB 不变 |
| L9-3b | `delete_files=false` 且越界 `file_path` → DB 删除成功，磁盘不动 |
| L9-4 | `remove_episode` 删最后一集后条目消失 |
| L9-5 | 文件不存在时 `delete_files=true` 仍成功提交 DB |
| L9-6 | loopback 投送后 `remove_episode` → `active_cast` 清空，可对**新分集**再次 `cast_episode` |

### 7.2 Flutter Widget（9a）

| ID | 场景 |
|----|------|
| W9-1 | 确认框默认勾选删文件 |
| W9-2 | 取消不改变片库 |
| W9-3 | 删除整条目成功 invalidate 列表 |
| W9-4 | Series（≥2 集）删除单分集调用 `removeEpisode` |

### 7.3 集成（9a）

| ID | 场景 |
|----|------|
| U11 | 下载 → 片库可见 → 删除 → 列表为空；`delete_files=true` 时媒体文件不存在 |

9b–9d 各增加对应用例（合并冲突、海报展示、目录选择器、失败重试）。

---

## 8. 与 AGENTS.md 对齐检查

- [x] 经 `Engine` 写入片库
- [x] 多行持久化在单事务
- [x] `media_dir` 校验与 `canonicalize`（仅 `delete_files=true` 删文件路径）
- [x] 分集唯一键与合并语义
- [x] LAN 元数据不含 `source_url`
- [x] 不提供站外破解能力

---

## 9. 不在本 Plan 范围

- 批量多选删除
- 缓存目录整体搬迁与 SQLite 路径批量重写
- 自动清理「仅移出片库」产生的孤儿文件
- TV 片库网格项直接删除（仅详情页）
- Windows / 桌面 Universal Links（Plan 6c）
- 片库全文搜索

---

## 10. PM 定稿摘要

| 决策项 | 结论 |
|--------|------|
| v0.2 主线 | **Plan 9 片库管理优先** |
| 第一实现切片 | **9a 删除** |
| 删除默认行为 | **默认删文件**；可勾选「仅移出片库」 |
| `delete_files=false` | **跳过路径校验**，允许移除脏数据 |
| 投送中删除 | **revoke token + `stop_cast_internal(false)`** |
| 9a 发版 | **`v0.1.1` tag** |
| 完整 v0.2 | **`v0.2.0` = 9a–9d 全部完成** |
| v0.1.0 发版 | 与 9a **并行**，不阻塞开发 |
| 磁盘文件名 | 重命名 **不改** 文件名 |
| 空 Series | 删最后一集 **自动删条目壳** |
