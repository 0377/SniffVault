# Plan 6a 受控 WebView + 嗅探闭环 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 交付受控内置浏览、Cookie 注入解析、嗅探候选入队，以及任务级 Cookie/Referer 快照，使登录后的下载在重启后仍能带鉴权头完成。

**Architecture:** 引擎先扩展任务鉴权列与 worker HTTP 头。Flutter 用 `webview_flutter` 的 `NavigationDelegate` + 注入脚本观察 `HookRequest`，Dart 映射 `SniffEvent`。`platforms/webview_sniff` 只做 Cookie 仓读写与 Android `isTelevision`。C ABI 与 Dart `EngineHost` 必须同一提交更新。

**Tech Stack:** 现有 Rust engine / JSON FFI / Cargokit、flutter_riverpod、go_router、webview_flutter、新建 path 插件 `platforms/webview_sniff`

**规格:** `docs/superpowers/specs/2026-09-07-webview-sniff-design.md`

**修订:** 2026-09-07 plan review（钩子策略 B、生产 TV 检测、生产 Cookie 导出、FFI Dart 同提交、T1/U6 门禁边界）

## Global Constraints

- 产品：受控 WebView，**不是**片源导航浏览器；无站点目录、无官方收藏夹、无 Share / LAN / TV Leanback / 系统抓包 / DRM 绕过
- 嗅探策略 **B**：四端 `NavigationDelegate` + 注入脚本，覆盖为「中」；**禁止** `shouldInterceptRequest` 与遍历 PlatformView 找 WebView
- 导航顺序固定：**片库 | 浏览 | 任务 | 添加 | 设置**；窄屏 `NavigationBar`，宽屏 `NavigationRail`（断点 `kAppShellBreakpoint == 600`）
- Feature 禁止 `EngineHost.open()`；写任务/片库必须经 `Engine`
- Cookie：**不得**出现在 `list_tasks` / `task_updated` JSON、路由 query、SnackBar、日志全文；调试日志只打 host + initiator
- FFI JSON / `DownloadAuth` 字段名 **`cookies`**；SQLite / `DownloadTask` 字段名 **`cookie_header`**
- 浏览 Cookie 仓 = 平台 CookieManager（插件 `cookieHeaderFor` / `clearCookies`）；引擎只存入队瞬间快照
- 生产 `isTelevision` 必须走 Android `UI_MODE_TYPE_TELEVISION` Channel；测试可 override，**默认实现不得写死 false 并跳过 Channel**
- `engine_enqueue_single` 增加 `opts_json` 的提交 **必须同时** 改 Dart `native_bindings` / `EngineHost`
- 进度更新禁止整行 `upsert` 任务（以免鉴权列被覆盖成 NULL）
- U6 门禁 = 解析本页 + 真引擎；U7 嗅探非门禁
- `docs/` 在 `.gitignore`；计划/规格用 `git add -f`
- 验证：
  ```bash
  cargo fmt --manifest-path engine/Cargo.toml --all -- --check
  cargo test --manifest-path engine/Cargo.toml --workspace
  cargo clippy --manifest-path engine/Cargo.toml --all-targets --all-features -- -D warnings
  cd app && flutter pub get && flutter test
  ```
- 提交信息中文；不提交 `GeneratedPluginRegistrant.*` 除非本任务新增插件后确需登记

---

## File Map

| 路径 | 职责 |
|------|------|
| `engine/src/types.rs` | `DownloadAuth`；`DownloadTask.cookie_header` / `referer`（序列化跳过） |
| `engine/src/tasks/schema.rs` | `TASK_SCHEMA_VERSION=3`；`TASK_MIGRATION_V3` |
| `engine/src/tasks/store.rs` | 迁移 v2/v3 分条件执行；读写新列 |
| `engine/src/engine.rs` | `enqueue_*` 写入鉴权快照 |
| `engine/src/download/http.rs` | 客户端级 Cookie/Referer 用于下载 GET |
| `engine/src/download/worker.rs` | 按任务快照 `with_auth` |
| `engine/ffi/src/sync_dispatch.rs` | enqueue FFI |
| `engine/tests/task_store.rs` | 重启持久化 |
| `engine/tests/types_serde.rs` | T2 脱敏 |
| `engine/tests/download_integration.rs` | T1 入队后下载带头（Task 3 起） |
| `engine/tests/enqueue_auth.rs` | T4 入队鉴权与回滚 |
| `app/lib/engine/models/download_auth.dart` | Dart `DownloadAuth` |
| `app/lib/engine/native_bindings.dart` / `engine_host.dart` | **与 FFI 同提交** 增加 opts_json |
| `platforms/webview_sniff/` | Cookie 仓 + `isTelevision` |
| `app/lib/providers/engine_repository.dart` | `sniffUrls`、`enqueue*` + `DownloadAuth?` |
| `app/lib/features/browse/browse_url.dart` | 地址栏校验 |
| `app/lib/features/browse/hook_to_sniff.dart` | Hook → `SniffEvent` |
| `app/lib/features/browse/sniff_accumulator.dart` | 500 条 + 顶层导航清空 |
| `app/lib/features/browse/browse_chrome.dart` | 地址栏/导航按钮 |
| `app/lib/features/browse/sniff_candidate_list.dart` | 候选列表 |
| `app/lib/features/browse/browse_screen.dart` | WebView + 解析本页 |
| `app/lib/features/browse/browse_unavailable_screen.dart` | TV 说明 |
| `app/lib/providers/browse_resolve_provider.dart` | 向导入参（outcome + auth） |
| `app/lib/providers/browse_session.dart` | 解析本页 / 嗅探 debounce（无 WebView） |
| `app/lib/features/browse/cookie_store.dart` | CookieExporter + 清除 |
| `app/lib/providers/device_profile.dart` | `isTelevisionProvider` |
| `app/lib/shell/app_shell.dart` / `router.dart` | 五栏 + `/browse` + `/browse/wizard` |
| `app/lib/features/add/resolve_wizard.dart` | NeedsBrowser CTA |
| `app/lib/features/settings/settings_screen.dart` | 清除浏览 Cookie |
| `app/pubspec.yaml` | `webview_flutter` + path 插件 |
| `README.md` / `platforms/README.md` | 使用说明 |
| `app/test/` | W3'、W5–W9 |
| `app/integration_test/` | U6 |

---

### Task 1: 任务库鉴权列 + JSON 脱敏

**Files:**
- Modify: `engine/src/types.rs`
- Modify: `engine/src/tasks/schema.rs`
- Modify: `engine/src/tasks/store.rs`
- Modify: 所有 `DownloadTask {` 字面量（`engine/src/engine.rs`、`engine/src/download/worker.rs`、`engine/src/download/scheduler.rs`、`engine/src/download/checkpoint.rs`、`engine/tests/**`）
- Test: `engine/tests/task_store.rs`、`engine/tests/types_serde.rs`

**Interfaces:**
- Consumes: 现有 `TaskStore::open` / `upsert` / `get`
- Produces: `DownloadTask { cookie_header: Option<String>, referer: Option<String> }`；serde 序列化 **省略** 这两键；`DownloadAuth { cookies: Option<String>, referer: Option<String> }`

- [ ] **Step 1: 写失败测试（T2 + 重启读列）**

在 `engine/tests/types_serde.rs` 追加：

```rust
#[test]
fn download_task_json_omits_auth_snapshot() {
    let task = DownloadTask {
        id: "t1".into(),
        parent_id: None,
        season: None,
        title: "ep".into(),
        source_url: "https://example.com/v.mp4".into(),
        quality_label: None,
        status: TaskStatus::Queued,
        progress_bytes: 0,
        total_bytes: None,
        error_message: None,
        output_path: None,
        library_item_id: None,
        episode_index: None,
        created_at_ms: 0,
        updated_at_ms: 0,
        cookie_header: Some("sid=secret".into()),
        referer: Some("https://example.com/page".into()),
    };
    let value = serde_json::to_value(&task).unwrap();
    assert!(value.get("cookie_header").is_none());
    assert!(value.get("referer").is_none());
    assert_eq!(value["source_url"], "https://example.com/v.mp4");
    assert!(!serde_json::to_string(&task).unwrap().contains("sid=secret"));
}
```

在 `engine/tests/task_store.rs` 的 `sample` 补 `cookie_header: None, referer: None`（编译需要），并追加：

```rust
#[test]
fn auth_snapshot_survives_reopen() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("tasks.db");
    {
        let store = TaskStore::open(&path).unwrap();
        let mut task = sample("a", None, TaskStatus::Queued);
        task.cookie_header = Some("sid=ok".into());
        task.referer = Some("http://x/page".into());
        store.upsert(&task).unwrap();
    }
    let store = TaskStore::open(&path).unwrap();
    let got = store.get("a").unwrap();
    assert_eq!(got.cookie_header.as_deref(), Some("sid=ok"));
    assert_eq!(got.referer.as_deref(), Some("http://x/page"));
}
```

- [ ] **Step 2: 运行测试确认失败**

```bash
cargo test --manifest-path engine/Cargo.toml --test types_serde --test task_store
```

Expected: 编译失败（无字段）或测试失败（读出 None）。

- [ ] **Step 3: 实现类型、迁移、store**

`engine/src/types.rs` 在 `DownloadTask` 末尾（`updated_at_ms` 之后）增加：

```rust
    /// 入队瞬间 Cookie 快照；仅本机 store/worker 使用。
    #[serde(default, skip_serializing)]
    pub cookie_header: Option<String>,
    #[serde(default, skip_serializing)]
    pub referer: Option<String>,
```

同文件增加：

```rust
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DownloadAuth {
    pub cookies: Option<String>,
    pub referer: Option<String>,
}
```

并在 `engine/src/lib.rs` 若有类型 re-export 则导出 `DownloadAuth`。

`engine/src/tasks/schema.rs`：

```rust
pub const TASK_SCHEMA_VERSION: i64 = 3;

pub const TASK_MIGRATION_V2: &str = "ALTER TABLE download_tasks ADD COLUMN checkpoint_json TEXT;";

pub const TASK_MIGRATION_V3: &str = r#"
ALTER TABLE download_tasks ADD COLUMN cookie_header TEXT;
ALTER TABLE download_tasks ADD COLUMN referer TEXT;
"#;
```

**重写** `TaskStore::migrate`：禁止在升到 v3 时只跑 V2。

```rust
        if version > TASK_SCHEMA_VERSION {
            return Err(EngineError::Message(format!(
                "unsupported tasks.db schema version {version}, expected <= {TASK_SCHEMA_VERSION}"
            )));
        }
        if version == 0 {
            Self::ensure_v1_columns(conn)?;
        }
        if version < 2 {
            let tx = conn.unchecked_transaction()?;
            if let Err(e) = tx.execute_batch(TASK_MIGRATION_V2) {
                if !e.to_string().contains("duplicate column name") {
                    return Err(EngineError::Db(e));
                }
            }
            tx.execute("PRAGMA user_version = 2", [])?;
            tx.commit()?;
        }
        let version: i64 = conn.query_row("PRAGMA user_version", [], |row| row.get(0))?;
        if version < 3 {
            let tx = conn.unchecked_transaction()?;
            if let Err(e) = tx.execute_batch(TASK_MIGRATION_V3) {
                if !e.to_string().contains("duplicate column name") {
                    return Err(EngineError::Db(e));
                }
            }
            tx.execute("PRAGMA user_version = 3", [])?;
            tx.commit()?;
        }
        Ok(())
```

SQLite 多句 `ALTER` 的 `execute_batch` 若第一条成功第二条 duplicate，需按句执行并忽略 duplicate（与现网 V2 相同策略）。**只使用** `TASK_MIGRATION_V3` 常量作为语句来源，不要另写一份列名。

`row_to_task`：SELECT 在 `updated_at_ms` 后加 `cookie_header, referer`（下标 15、16）。**所有** `SELECT id, parent_id, ... updated_at_ms FROM download_tasks` 必须同步加这两列（`get` / `list_all` / `list_children` / `list_runnable_tasks`）。

`upsert_conn` INSERT/UPDATE 增加 `cookie_header, referer`。`update_progress` / checkpoint / `set_task_status` **保持列更新**，禁止改成整行 `upsert`。

给每个 `DownloadTask {` 字面量补：

```rust
        cookie_header: None,
        referer: None,
```

- [ ] **Step 4: 再跑测试**

```bash
cargo test --manifest-path engine/Cargo.toml --workspace
```

Expected: PASS（含既有 task_store / types_serde）。

- [ ] **Step 5: Commit**

```bash
git add engine/src/types.rs engine/src/lib.rs engine/src/tasks/schema.rs engine/src/tasks/store.rs engine/src/engine.rs engine/src/download engine/tests
git commit -m "$(cat <<'EOF'
feat(engine): 任务库持久化 Cookie/Referer 快照并在 JSON 中脱敏

EOF
)"
```

---

### Task 2: 下载 HTTP 带任务鉴权（T1 / T3）

**Files:**
- Modify: `engine/src/download/http.rs`
- Modify: `engine/src/download/worker.rs`（创建 `HttpClient` 处）
- Test: `engine/src/download/http.rs` 既有 `#[cfg(test)]`；`engine/tests/download_integration.rs`

**Interfaces:**
- Consumes: Task 1 的 `DownloadTask.cookie_header` / `referer`
- Produces: `HttpClient::with_auth(self, cookies: Option<String>, referer: Option<String>) -> Self`；`get_text` / `get_bytes` / `get_range` / `head_size_and_ranges` / `get_stream` / `get_stream_range` 均带上客户端鉴权头。`get_page_text(..., &PageFetchOptions)` **仍只使用 opts**（解析路径不变）。

- [ ] **Step 1: 写失败测试**

在 `engine/src/download/http.rs` 测试模块追加（复用文件内 `spawn_server`）：

```rust
    async fn cookie_handler(headers: HeaderMap) -> impl IntoResponse {
        let cookie = headers
            .get("cookie")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();
        (StatusCode::OK, cookie)
    }

    #[tokio::test]
    async fn get_stream_sends_client_auth() {
        let (base_url, _guard) =
            spawn_server(Router::new().route("/v", get(cookie_handler))).await;
        let client = HttpClient::new(None)
            .unwrap()
            .with_auth(Some("sid=ok".into()), Some("http://ref.example/page".into()));
        let response = client.get_stream(&format!("{base_url}/v")).await.unwrap();
        let body = response.text().await.unwrap();
        assert_eq!(body, "sid=ok");
    }

    #[tokio::test]
    async fn get_stream_without_auth_sends_no_cookie() {
        let (base_url, _guard) =
            spawn_server(Router::new().route("/v", get(cookie_handler))).await;
        let client = HttpClient::new(None).unwrap();
        let response = client.get_stream(&format!("{base_url}/v")).await.unwrap();
        let body = response.text().await.unwrap();
        assert_eq!(body, "");
    }
```

在 `engine/src/download/http.rs` 测试模块追加（复用文件内 `spawn_server`）。**本任务不要**写 `mp4_download_sends_enqueued_cookie`（双 `TaskStore` 竞态）；T1 集成放到 Task 3，用 `enqueue_single(..., Some(&auth))`。

T3：无 cookie 的既有 `mp4_download_registers` 必须保持绿色。

- [ ] **Step 2: 运行确认 HTTP 单测失败**

```bash
cargo test --manifest-path engine/Cargo.toml -p video_sniffing_engine get_stream_sends_client_auth -- --nocapture
```

Expected: `with_auth` 未定义 或 断言失败。

- [ ] **Step 3: 实现 `HttpClient` 鉴权并让 worker 使用**

`HttpClient` 增加字段 `cookies: Option<String>`、`referer: Option<String>`。`new` 的 `Ok(Self { ... })` **必须**同时初始化这两字段为 `None`（以及现有 `client`/`cancel`）。

```rust
    pub fn with_auth(mut self, cookies: Option<String>, referer: Option<String>) -> Self {
        self.cookies = cookies;
        self.referer = referer;
        self
    }

    fn apply_client_auth(&self, mut request: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        if let Some(cookies) = &self.cookies {
            request = request.header(reqwest::header::COOKIE, cookies);
        }
        if let Some(referer) = &self.referer {
            request = request.header(reqwest::header::REFERER, referer);
        }
        request
    }
```

所有 **非** `get_page_text` 的请求构建改为 `self.apply_client_auth(self.client.get(url))`。

`worker.rs` 在 `HttpClient::new(...)` 成功后：

```rust
    let http = match HttpClient::new(config.user_agent.as_deref()) {
        Ok(c) => c
            .with_cancellation(cancel.clone())
            .with_auth(task.cookie_header.clone(), task.referer.clone()),
        Err(e) => return TaskRunOutcome::Failed(e),
    };
```

（若 `with_cancellation` 已存在，保持链式顺序：`new` → `with_cancellation` → `with_auth`。）

- [ ] **Step 4: 跑测试**

```bash
cargo test --manifest-path engine/Cargo.toml --workspace
```

Expected: PASS，含 `get_stream_sends_client_auth`、`get_stream_without_auth_sends_no_cookie`、`mp4_download_registers`。不含入队鉴权集成测试（Task 3）。

- [ ] **Step 5: Commit**

```bash
git add engine/src/download/http.rs engine/src/download/worker.rs engine/tests
git commit -m "$(cat <<'EOF'
feat(engine): 下载请求附带任务 Cookie 与 Referer 快照

EOF
)"
```

---

### Task 3: enqueue API + FFI + Dart EngineHost（T1 / T4）

**Files:**
- Modify: `engine/src/engine.rs`
- Modify: `engine/src/tasks/store.rs`（`upsert_parent_with_children` 在父任务写入后校验子任务 `id` 非空，失败不 commit）
- Modify: `engine/ffi/src/sync_dispatch.rs`
- Modify: `engine/ffi/tests/sync_tasks_test.rs`、`engine/ffi/tests/start_downloads_prepare_test.rs`
- Modify: `engine/tests/engine_facade.rs`、`engine/tests/download_integration.rs`、`engine/tests/task_events.rs` 以及所有 `enqueue_single(` / `enqueue_episodes(`
- Modify: `app/lib/engine/native_bindings.dart`、`app/lib/engine/engine_host.dart`、`app/integration_test/engine_smoke_test.dart`（`enqueueSingle` 增加可选 `auth`，默认 null，C 侧多传 `nullptr`）
- Create: `app/lib/engine/models/download_auth.dart`（供 EngineHost JSON；Repository 封装可留 Task 4）
- Test: `engine/tests/enqueue_auth.rs`；`engine/tests/download_integration.rs` 的 T1；`engine/tests/task_store.rs` 回滚

**Interfaces:**
- Consumes: `DownloadAuth`
- Produces: 见原 `enqueue_single` / `enqueue_episodes` 五参数签名；FFI `opts_json`；Dart `EngineHost.enqueueSingle({DownloadAuth? auth})` 与 C 五指针 **同提交**

- [ ] **Step 1: 写失败测试 `engine/tests/enqueue_auth.rs`**

```rust
use tempfile::tempdir;
use video_sniffing_engine::tasks::TaskStore;
use video_sniffing_engine::{DownloadAuth, Engine};

#[test]
fn enqueue_single_persists_auth_on_task() {
    let dir = tempdir().unwrap();
    let mut engine = Engine::open(dir.path()).unwrap();
    let auth = DownloadAuth {
        cookies: Some("sid=ok".into()),
        referer: Some("https://x/page".into()),
    };
    let id = engine
        .enqueue_single("t", "https://x/v.mp4", None, Some(&auth))
        .unwrap();
    drop(engine);
    let store = TaskStore::open(&dir.path().join("tasks.db")).unwrap();
    let task = store.get(&id).unwrap();
    assert_eq!(task.cookie_header.as_deref(), Some("sid=ok"));
    assert_eq!(task.referer.as_deref(), Some("https://x/page"));
}

#[test]
fn enqueue_episodes_copies_auth_to_parent_and_children() {
    let dir = tempdir().unwrap();
    let mut engine = Engine::open(dir.path()).unwrap();
    let auth = DownloadAuth {
        cookies: Some("sid=ok".into()),
        referer: Some("https://x/page".into()),
    };
    let (parent_id, child_ids) = engine
        .enqueue_episodes(
            "show",
            Some(1),
            &[(1, "e1".into(), "https://x/1.mp4".into())],
            None,
            Some(&auth),
        )
        .unwrap();
    let store = TaskStore::open(&dir.path().join("tasks.db")).unwrap();
    let parent = store.get(&parent_id).unwrap();
    let child = store.get(&child_ids[0]).unwrap();
    assert_eq!(parent.cookie_header.as_deref(), Some("sid=ok"));
    assert_eq!(child.cookie_header.as_deref(), Some("sid=ok"));
    assert_eq!(child.referer.as_deref(), Some("https://x/page"));
}

#[test]
fn enqueue_episodes_empty_writes_nothing() {
    let dir = tempdir().unwrap();
    let mut engine = Engine::open(dir.path()).unwrap();
    assert!(engine
        .enqueue_episodes("show", None, &[], None, None)
        .is_err());
    assert!(engine.list_tasks().unwrap().is_empty());
}

#[test]
fn upsert_parent_rolls_back_when_child_id_empty() {
    let dir = tempdir().unwrap();
    let store = TaskStore::open(&dir.path().join("tasks.db")).unwrap();
    let parent = sample_task("p"); // 与 task_store::sample 相同形状，Queued，cookie 可 None
    let mut child = sample_task("c1");
    child.parent_id = Some("p".into());
    child.id.clear();
    assert!(store.upsert_parent_with_children(&parent, &[child]).is_err());
    assert!(store.get("p").is_err());
}
```

`sample_task` 可复用 `task_store.rs` 的 `sample`（测试可见性：把 `sample` 留在 `task_store.rs` 并在本文件复制最小字面量，禁止依赖未导出私有函数）。

在 `engine/tests/support/fixture_server.rs` **新增** `serve_cookie_mp4() -> (SocketAddr, ServerGuard)`（在**该模块内**构造 `ServerGuard`，测试 crate 不要用 `ServerGuard(handle)`）。`GET /secret.mp4`：Cookie 含 `sid=ok` 则返回 `sample.mp4` 字节，否则 403。

`engine/tests/download_integration.rs`：

```rust
#[test]
fn mp4_download_sends_enqueued_cookie() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let mut fx = EngineFixture::open();
        let (addr, _guard) = fixture_server::serve_cookie_mp4().await;
        let url = format!("http://{addr}/secret.mp4");
        let auth = DownloadAuth {
            cookies: Some("sid=ok".into()),
            referer: Some(format!("http://{addr}/page")),
        };
        let id = fx
            .engine
            .enqueue_single("authed", &url, None, Some(&auth))
            .unwrap();
        fx.engine.start_downloads().unwrap();
        wait_for_task(&fx.engine, &id, TaskStatus::Completed, Duration::from_secs(30)).await;
    });
}
```

FFI：`engine_enqueue_single` 带 auth 入队后 `engine_list_tasks` JSON **不含** `sid=ok`。

- [ ] **Step 2: 运行确认失败**

```bash
cargo test --manifest-path engine/Cargo.toml --test enqueue_auth
```

Expected: 参数个数不匹配。

- [ ] **Step 3: 实现 enqueue + FFI**

构造 `DownloadTask` 时：

```rust
        cookie_header: auth.and_then(|a| a.cookies.clone()),
        referer: auth.and_then(|a| a.referer.clone()),
```

父任务与子任务使用 **同一** `auth` 克隆。`upsert_parent_with_children`：

```rust
        Self::upsert_conn(&tx, parent)?;
        for child in children {
            if child.id.is_empty() {
                return Err(EngineError::InvalidArg("task id must not be empty".into()));
            }
            Self::upsert_conn(&tx, child)?;
        }
        tx.commit()?;
```

空 `id` 必须在 **父任务 upsert 之后** 检查，以便 T4 回滚测试有效。

Dart **本任务同步**：

```dart
typedef EngineEnqueueSingleNative = Pointer<Char> Function(
  Pointer<Void> handle,
  Pointer<Utf8> title,
  Pointer<Utf8> url,
  Pointer<Utf8> qualityLabel,
  Pointer<Utf8> optsJson,
);
```

`EngineHost.enqueueSingle` / `enqueueEpisodes` 增加 `DownloadAuth? auth`，`null` → 不传 cookies 键 / `opts_json` 为 `nullptr`。`engine_smoke_test` 不传 auth。

`sync_dispatch.rs`：

```rust
#[derive(Debug, Deserialize)]
struct EnqueueAuthJson {
    cookies: Option<String>,
    referer: Option<String>,
}

fn parse_enqueue_auth(opts_json: *const c_char) -> Result<Option<DownloadAuth>, EngineError> {
    if opts_json.is_null() {
        return Ok(None);
    }
    let parsed: EnqueueAuthJson = parse_json_c_str(opts_json, "opts_json")?;
    Ok(Some(DownloadAuth {
        cookies: parsed.cookies,
        referer: parsed.referer,
    }))
}
```

`engine_enqueue_single` 增加参数 `opts_json`，`parse_enqueue_auth` 后传入 `enqueue_single`。

`EnqueueEpisodesArgs` 增加 `#[serde(default)] cookies: Option<String>`、`referer: Option<String>`，拼成 `DownloadAuth` 传入 `enqueue_episodes`。

把 Rust 旧调用点补 `None`（**包括** `engine/tests/engine_facade.rs`）。FFI 测试增加 `std::ptr::null()` 作为 `opts_json`。

T1 使用 `enqueue_single(..., Some(&auth))`，不要第二套 `TaskStore` upsert。

- [ ] **Step 4: 跑 workspace 测试 + `cd app && flutter test`**（Dart FFI 签名已改，未加载原生库的 widget 测试应仍绿）

```bash
cargo test --manifest-path engine/Cargo.toml --workspace
cargo clippy --manifest-path engine/Cargo.toml --all-targets --all-features -- -D warnings
cd app && flutter test
```

- [ ] **Step 5: Commit**（**同一 commit** 含 engine FFI 与 `app/lib/engine/native_bindings.dart` `engine_host.dart` `download_auth.dart`）

```bash
git add engine app/lib/engine app/integration_test/engine_smoke_test.dart
git commit -m "$(cat <<'EOF'
feat(engine): 入队鉴权快照并同步 Dart FFI 签名

EOF
)"
```

---

### Task 4: EngineRepository、Fake、向导回调

**Files:**
- Modify: `app/lib/providers/engine_repository.dart`
- Modify: `app/test/fakes/fake_engine_repository.dart`
- Modify: `app/lib/features/add/resolve_wizard.dart` typedef（可选 `DownloadAuth? auth`）
- Test: `app/test/engine_repository_auth_test.dart`

**Interfaces:**
- Consumes: Task 3 的 `EngineHost.enqueueSingle(..., auth:)` 与已有 `sniffUrls`
- Produces: `EngineRepository` 上 `sniffUrls`、`enqueue*` 的 `DownloadAuth?`；Fake `lastEnqueueAuth` / `lastResolveOpts`

Dart `DownloadAuth` 已在 Task 3 创建则本任务只引用，勿重复定义。

**不要**再改 `native_bindings` C 签名（已在 Task 3）。

- [ ] **Step 1: 扩展 Fake 并写一个纯 Dart 测试**

Create `app/test/engine_repository_auth_test.dart`：用 Fake 记录最后一次 `enqueueSingle` 的 `auth`。若 Fake 尚未有字段，本步先写测试期望 `FakeEngineRepository.lastAuth`。

```dart
void main() {
  test('Fake enqueueSingle stores DownloadAuth', () {
    final fake = FakeEngineRepository();
    fake.enqueueSingle(
      title: 't',
      url: 'https://x/v.mp4',
      auth: const DownloadAuth(cookies: 'sid=ok', referer: 'https://x/page'),
    );
    expect(fake.lastEnqueueAuth?.cookies, 'sid=ok');
  });
}
```

- [ ] **Step 2: `cd app && flutter test test/engine_repository_auth_test.dart`**

Expected: FAIL（无 `auth` 参数 / 无 `lastEnqueueAuth`）。

- [ ] **Step 3: 实现 Repository / Fake / typedef**

`EngineHostRepository.enqueueSingle` 把 `auth` 传给 Task 3 的 `EngineHost`。`sniffUrls` 转调已有 `EngineHost.sniffUrls`。

`ResolveWizard` typedef 增加可选 `DownloadAuth? auth`。**添加页不传 auth**。

Fake：`lastEnqueueAuth`；`sniffUrls` 默认 `const []`。

- [ ] **Step 4: `cd app && flutter test`** PASS

- [ ] **Step 5: Commit** `feat(app): EngineRepository 支持嗅探与入队鉴权`

---

### Task 5: 地址栏校验 + BrowseChrome（W6）

**Files:**
- Create: `app/lib/features/browse/browse_url.dart`
- Create: `app/lib/features/browse/browse_chrome.dart`
- Test: `app/test/browse_url_test.dart`、`app/test/browse_chrome_test.dart`

**Interfaces:**
- Produces:
  ```dart
  String? browseUrlError(String raw);
  Uri? parseBrowseUrl(String raw); // 合法 http/https 则返回 trimmed Uri
  String cookieHeaderFrom(Iterable<CookiePair> cookies);
  ```
  `browseUrlError`：空 → `'请输入地址'`；scheme 为 `javascript`/`file`/`data` 或非 `http`/`https` → `'非法地址'`；其余 `null`。

- [ ] **Step 1: 失败测试**

`app/test/browse_url_test.dart`：

```dart
import 'package:flutter_test/flutter_test.dart';
import 'package:video_sniffing/features/browse/browse_url.dart';

void main() {
  test('W6 javascript rejected', () {
    expect(browseUrlError('javascript:alert(1)'), '非法地址');
    expect(parseBrowseUrl('javascript:alert(1)'), isNull);
  });
  test('http accepted', () {
    expect(browseUrlError('http://127.0.0.1:1/'), isNull);
    expect(parseBrowseUrl('https://example.com/a')?.host, 'example.com');
  });
}
```

`browse_chrome_test.dart`：泵 `BrowseChrome`，在 `Key('browse_url_field')` 输入 `javascript:alert(1)` 并提交，`onSubmit` 的 mock **不得**被调用；屏幕出现 `非法地址`。

- [ ] **Step 2: `cd app && flutter test test/browse_url_test.dart test/browse_chrome_test.dart`**

Expected: FAIL。

- [ ] **Step 3: 最小实现**

`browse_url.dart` 用 `Uri.tryParse`。只允许 `http`/`https`（大小写不敏感）。

`cookieHeaderFrom`：`name=value` 用 `'; '` 连接（规格与 Plan 3 R6 一致）。

`BrowseChrome`：`TextField` + 后退/前进/刷新/`解析本页`（`Key('browse_resolve_page')`）。`canGoBack`/`canGoForward`/`resolveEnabled` 控制按钮。`resolveEnabled` 在 url 空或 `about:blank` 时为 false。

- [ ] **Step 4: 再跑上述测试 PASS**

- [ ] **Step 5: Commit** `feat(app): 受控浏览地址栏校验与工具栏`

---

### Task 6: 五栏 Shell、路由、TV 开关（W3'）

**Files:**
- Create: `app/lib/platform/television.dart`（`Future<bool> detectIsTelevision()` → MethodChannel `webview_sniff/device` 方法 `isTelevision`；非 Android 直接 `false`）
- Create: `app/lib/providers/device_profile.dart`（`isTelevisionProvider` 以 `detectIsTelevision` 为源；测试 override）
- Modify: `platforms/webview_sniff` 或 `app/android/.../MainActivity.kt`：实现 Channel，`uiMode & UI_MODE_TYPE_MASK == UI_MODE_TYPE_TELEVISION`
- Test: `app/test/app_shell_test.dart` **必须** `ProviderScope`；TV 用例 `overrideWithValue(true)`

**Interfaces:**
- 生产路径：`isTelevisionProvider` **读取检测结果**，禁止 `Provider((ref) => false)` 作为唯一实现
- Destinations：`片库`,`浏览`,`任务`,`添加`,`设置`
- TV：无「浏览」destination；`/browse` → `BrowseUnavailableScreen`（无 WebView）

- [ ] **Step 1: 改 `app_shell_test.dart`**

五个 `StatefulShellBranch`。断言 `find.text('浏览')`。`ProviderScope(overrides: [isTelevisionProvider.overrideWith((ref) async => true)])`（若为 `FutureProvider`）或 `overrideWithValue(true)` 下找不到 `浏览`。

若 AppShell 无 Riverpod，改为 `ConsumerWidget`。测试路由器与生产 `router.dart` 分支顺序一致。

- [ ] **Step 2: `cd app && flutter test test/app_shell_test.dart`** → FAIL（仍四栏）

- [ ] **Step 3: 实现五栏 + 真实 `detectIsTelevision`。** 本任务创建最小 `platforms/webview_sniff`（可先只有 `isTelevision`）。占位 `BrowseScreen` 可无 WebView。`/browse/wizard` 用根 Navigator。

Channel 未绑定时 Android 返回 `false`，不得 crash。测试用 override，不必 mock Channel。

- [ ] **Step 4: `cd app && flutter test`** PASS

- [ ] **Step 5: Commit** `feat(app): 主导航增加浏览并在 TV 上隐藏`

---

### Task 7: NeedsBrowser CTA（W5）与向导不循环

**Files:**
- Modify: `app/lib/features/add/resolve_wizard.dart`
- Modify: `app/test/resolve_wizard_test.dart`
- Modify: `app/lib/features/add/add_screen.dart`

**Interfaces:**
- Produces: `ResolveWizard({ VoidCallback? onOpenBrowser })`
- `onOpenBrowser == null`：无「打开内置浏览」按钮（TV / 纯展示）
- 主文案：**此站点需要在内置浏览中打开并登录后再解析。**
- 按钮：**打开内置浏览**、**返回**
- 从浏览向导再次 `NeedsBrowser`：**不要**再传 `onOpenBrowser`（或传入空），避免死循环

- [ ] **Step 1: 更新 W1 文案断言 + W5**

```dart
  testWidgets('W1 NeedsBrowser shows browser hint', (tester) async {
    await tester.pumpWidget(_wrap(const ResolveOutcomeNeedsBrowser(reason: 'auth_required')));
    expect(find.textContaining('需要在内置浏览中打开'), findsOneWidget);
  });

  testWidgets('W5 open browse button when callback set', (tester) async {
    var opened = false;
    await tester.pumpWidget(MaterialApp(
      home: Scaffold(
        body: ResolveWizard(
          outcome: const ResolveOutcomeNeedsBrowser(reason: 'auth_required'),
          onEnqueue: (_) async {},
          onOpenBrowser: () => opened = true,
        ),
      ),
    ));
    await tester.tap(find.text('打开内置浏览'));
    expect(opened, isTrue);
  });
```

- [ ] **Step 2: flutter test 该文件** → FAIL

- [ ] **Step 3: 实现 CTA；`AddScreen` 在非 TV 时 `onOpenBrowser: () => context.go('/browse?url=${Uri.encodeQueryComponent(url)}')`**

TV：`ref.watch(isTelevisionProvider)` 为 true 则不传 `onOpenBrowser`。

- [ ] **Step 4: `cd app && flutter test`** PASS

- [ ] **Step 5: Commit** `feat(app): NeedsBrowser 跳转内置浏览`

---

### Task 8: 嗅探缓冲 + 候选列表（W7）

**Files:**
- Create: `app/lib/features/browse/hook_to_sniff.dart`
- Create: `app/lib/features/browse/sniff_accumulator.dart`
- Create: `app/lib/features/browse/sniff_candidate_list.dart`
- Test: `app/test/sniff_accumulator_test.dart`、`app/test/sniff_candidate_list_test.dart`

**Interfaces:**
- `class HookRequest { url, pageUrl, isMainFrame, mime }`
- `SniffEvent hookToSniffEvent(HookRequest r)` 按规格：
  - `isMainFrame` → `navigation`
  - mime 含 `video`/`audio` 或 url 含 `.m3u8` / `.mp4`（忽略 query）→ `media`
  - 否则若非主框架 → `sub_resource`
  - 其余 → `other`
- `SniffAccumulator`：`add`；`onTopLevelNavigation` 清空；长度 > 500 丢最旧；`events` getter
- `SniffCandidateList({ required List<ResourceCandidate> candidates, required ValueChanged<ResourceCandidate> onSelect })`
  - 空列表时 `SizedBox.shrink()`
  - 非空显示数量与 `ListTile`（title 为 `candidate.title ?? candidate.url`）

- [ ] **Step 1: 单测** accumulator 501 条只留 500；navigation 清空；`hookToSniffEvent` 三条映射。W7：pump 列表 fake 候选，tap 后 `onSelect` 收到该 url。

- [ ] **Step 2: flutter test 失败**

- [ ] **Step 3: 实现**

- [ ] **Step 4: PASS**

- [ ] **Step 5: Commit** `feat(app): 嗅探事件映射与候选列表`

---

### Task 9: 解析本页、向导路由、入队带 DownloadAuth（W8 / W9）

**Files:**
- Create: `app/lib/providers/browse_session.dart`
- Create: `app/lib/providers/browse_resolve_provider.dart`
- Modify: `app/lib/features/browse/browse_screen.dart`
- Modify: `app/lib/router.dart` `/browse/wizard`
- Modify: `app/lib/features/add/resolve_wizard.dart` 入队调用处传 `auth`
- Test: `app/test/browse_resolve_test.dart`（**禁止** pump 真实 `WebViewWidget`）

**Interfaces:**
- `CookieExporter`：`Future<String?> cookieHeaderFor(Uri page)`
- 生产实现类名 `PluginCookieExporter`，内部调 `WebViewSniff.cookieHeaderFor`（插件可在 Task 10 落地；本任务测试只用 Fake）
- W8：只测 `BrowseSession.resolveThisPage` / Notifier：给定 `currentUrl` + Fake exporter + Fake repo

- [ ] **Step 1: W8/W9 不依赖 WebView**

```dart
  test('W8 resolveThisPage passes cookies', () async {
    final fake = FakeEngineRepository();
    final session = BrowseSession(
      repo: fake,
      cookies: FakeCookieExporter('sid=ok'),
    );
    session.currentUrl = Uri.parse('http://x/page');
    await session.resolveThisPage();
    expect(fake.lastResolveOpts?.cookies, 'sid=ok');
  });
```

入队 W9：`session` 设置 outcome 后调用与向导相同的 `enqueueSingle(..., auth: session.auth)` 断言 `lastEnqueueAuth`。添加页：`enqueueSingle` 不传 auth。

- [ ] **Step 2: 测试失败**

- [ ] **Step 3: 实现会话 Notifier**

顶层 URL 变化：`accumulator.onTopLevelNavigation()`。钩子事件 debounce **300ms** 后 `repo.sniffUrls(acc.events, pageUrl: currentUrl)` 更新 `candidates`。

`/browse?url=`：`initState`/`didChangeDependencies` 里 `parseBrowseUrl` 成功则加载（Task 11 真正 loadRequest）。

Wizard 页：`NeedsBrowser` **不**传 `onOpenBrowser`。

- [ ] **Step 4: `cd app && flutter test`** PASS

- [ ] **Step 5: Commit** `feat(app): 解析本页并带着鉴权快照入队`

---

### Task 10: 插件 Cookie 仓 + 设置清除

**Files:**
- Modify: `platforms/webview_sniff/`（Task 6 已有 `isTelevision`）增加：
  - `cookieHeaderFor(url: String) -> String?`（Android `CookieManager.getCookie`；iOS/macOS `WKHTTPCookieStore` 拼 `name=value; ...`；Windows WebView2 cookie manager）
  - `clearCookies()`
- Create: `app/lib/features/browse/cookie_store.dart`（`CookieExporter` + `BrowseCookieStore`；生产 `PluginCookieExporter`）
- Modify: `app/lib/features/settings/settings_screen.dart`
- Test: `app/test/settings_clear_cookies_test.dart`

**Interfaces:**
- 按钮 `Key('settings_clear_browse_cookies')`；SnackBar **已清除浏览 Cookie**
- **不**清任务库快照；无 Cookie 文本框
- 钩子 payload 禁止带 Cookie

- [ ] **Step 1: 设置页 tap 后 fake `clearAll` + SnackBar；另测 `cookieHeaderFrom` 拼接（已在 Task 5 则可复用）**

- [ ] **Step 2: FAIL**

- [ ] **Step 3: 插件四端实现 Cookie 读写。某端读不到时 `cookieHeaderFor` 返回 null，「解析本页」仍可调用 `resolveUrl`（opts.cookies 为空），不得抛未捕获异常。**

- [ ] **Step 4: PASS `cd app && flutter test`**

- [ ] **Step 5: Commit** `feat(app): 浏览 Cookie 导出与设置清除`

---

### Task 11: webview_flutter 浏览页 + 策略 B 嗅探（Android/iOS）

**Files:**
- Modify: `app/pubspec.yaml`：`webview_flutter: ^4.10.0`（若未加）
- Modify: `app/lib/features/browse/browse_screen.dart`
- Create: `app/lib/features/browse/sniff_script.dart`（注入脚本字符串，Dart 单测可断言包含 `fetch`）
- Test: 不强制真实 WebView widget 测试

**Interfaces:**
- 使用 **同一** `WebViewController`：`NavigationDelegate`（主框架 → `HookRequest(isMainFrame: true)`）+ `runJavaScript` 注入 + `JavaScriptChannel`（子资源）
- **禁止** `shouldInterceptRequest`、禁止搜 PlatformView 里的 `WebView`
- UA：`settings.user_agent` 非空则 `setUserAgent`
- 加载失败：页内错误 + 刷新；不崩溃
- 「解析本页」走 `BrowseSession` + `PluginCookieExporter`

注入脚本（`sniff_script.dart` 常量）：

```javascript
(function() {
  const post = (url, mime) => {
    try {
      SniffChannel.postMessage(JSON.stringify({url: String(url), mime: mime || '', is_main_frame: false}));
    } catch (e) {}
  };
  const origFetch = window.fetch;
  window.fetch = function() { try { post(arguments[0], ''); } catch (e) {} return origFetch.apply(this, arguments); };
  const origOpen = XMLHttpRequest.prototype.open;
  XMLHttpRequest.prototype.open = function(method, url) { try { post(url, ''); } catch (e) {} return origOpen.apply(this, arguments); };
})();
```

Channel 名与 `JavaScriptChannel` 注册名必须一致。`page_url` 由 Dart 填当前主框架 URL。

- [ ] **Step 1: `sniff_script.dart` 含 `fetch` / `XMLHttpRequest` 的单测**

- [ ] **Step 2: FAIL 然后实现 BrowseScreen `WebViewWidget`**

- [ ] **Step 3: `cd app && flutter test`；能则 `flutter build apk --debug` 或 `flutter build ios --debug --no-codesign`（缺环境如实记录）**

- [ ] **Step 4: Commit** `feat(app): 内置浏览 WebView 与脚本嗅探`

---

### Task 12: 桌面 WebView 同一套 Dart 观察

**Files:**
- `app/macos` / `app/windows`：启用 `webview_flutter` 所需权限/Entitlements（若构建报错再补，不要无故改无关 plist）
- 嗅探逻辑复用 Task 11 Dart，**不要**实现 WebView2 `WebResourceRequested`

- [ ] **Step 1: `flutter build macos --debug` 与/或 `flutter build windows --debug`（当前 OS 能编哪个编哪个）**

- [ ] **Step 2: 桌面 `NavigationDelegate` + 同一注入脚本；失败不崩溃；解析本页可用**

- [ ] **Step 3: `cd app && flutter test`**

- [ ] **Step 4: Commit** `feat(app): 桌面浏览使用同一套 NavigationDelegate 嗅探`

---

### Task 13: U6 门禁、U7 可选、文档

**Files:**
- Create: `app/integration_test/browse_test.dart`
- Modify: `README.md`、`platforms/README.md`

**U6（必须绿）：** 本地 `HttpServer` 提供 HTML，内含 `http://127.0.0.1:port/clip.mp4`（与 U1 同源字节）。浏览加载 HTML → 点解析本页 → 向导出现「下载」。**不**要求嗅探列表有条目。

**U7（skip 合法）：** `<video src=".../clip.mp4">` 后候选出现；`markTestSkipped` 若 5s 内没有。

可选加分：种 Cookie 后再解析需 Cookie 页；失败不挡 U6。

```bash
cd app && flutter test integration_test/browse_test.dart -d macos
```

CI 可用 `INTEGRATION_SKIP_BROWSE`；文档写明本地应交 U6。

README：NeedsBrowser → 浏览 → 解析本页。`platforms/README.md`：插件职责为 Cookie + `isTelevision`，Share 未做。

全量 fmt / test / clippy / `flutter test`。Commit：`docs: 补充 Plan 6a 浏览主路径与 U6`

---

## 任务依赖

```
1 schema → 2 HTTP with_auth → 3 enqueue+FFI+Dart Host → 4 Repository
4 → 5 chrome → 6 shell+isTelevision → 7 NeedsBrowser
4 → 8 sniff 映射 UI
6+7+8 → 9 BrowseSession
6 → 10 Cookie 插件 + 设置
9+10 → 11 WebView+脚本（Android/iOS）→ 12 桌面同一套 → 13 U6
```

---

## Self-review（对照规格）

| 规格 | 任务 |
|------|------|
| 五栏、生产 TV 检测 | 6 |
| 地址栏校验 | 5 W6 |
| NeedsBrowser CTA | 7 |
| 解析本页 + 生产 Cookie 导出接口 | 9 + 10 W8 |
| HookRequest → SniffEvent | 8 |
| 策略 B NavigationDelegate+脚本 | 11–12 |
| 任务快照脱敏 / worker 头 / FFI 同提交 | 1–3 |
| T4 回滚 | 3 |
| 清除 Cookie | 10 |
| U6 门禁 / U7 非门禁 | 13 |

