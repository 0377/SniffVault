# Plan 6a 受控 WebView + 嗅探闭环 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 交付受控内置浏览、Cookie 注入解析、嗅探候选入队，以及任务级 Cookie/Referer 快照，使登录后的下载在重启后仍能带鉴权头完成。

**Architecture:** 引擎先扩展 `DownloadTask` 鉴权列与 worker HTTP 头；Flutter 经 `EngineRepository` 入队时传入 `DownloadAuth`。浏览 UI 用 `webview_flutter`；`platforms/webview_sniff` 只把请求观察为 `HookRequest`，Dart 按规格映射 `SniffEvent` 后调用现有 `sniffUrls`。不在原生层写库或入队。

**Tech Stack:** 现有 Rust engine / JSON FFI / Cargokit、flutter_riverpod、go_router、webview_flutter、新建 path 插件 `platforms/webview_sniff`

**规格:** `docs/superpowers/specs/2026-09-07-webview-sniff-design.md`

## Global Constraints

- 产品：受控 WebView，**不是**片源导航浏览器；无站点目录、无官方收藏夹、无 Share / LAN / TV Leanback / 系统抓包 / DRM 绕过
- 导航顺序固定：**片库 | 浏览 | 任务 | 添加 | 设置**；窄屏 `NavigationBar`，宽屏 `NavigationRail`（断点 `kAppShellBreakpoint == 600`）
- Feature 禁止 `EngineHost.open()`；写任务/片库必须经 `Engine`
- Cookie：**不得**出现在 `list_tasks` / `task_updated` JSON、路由 query、SnackBar、日志全文；调试日志只打 host + initiator
- 浏览 Cookie 仓 = 平台 `CookieManager`；引擎只存 **入队瞬间** `cookie_header` / `referer` 快照
- `engine_enqueue_single` 增加最后可选 `opts_json`；`engine_enqueue_episodes` 把 `cookies`/`referer` 并入现有 JSON
- Android TV（`isTelevisionProvider == true`）：无浏览 tab；`NeedsBrowser` 无「打开内置浏览」；`/browse` 仅说明页
- iOS/macOS 嗅探允许弱于 Android/Windows；**解析本页闭环必须通**
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
| `engine/tests/download_integration.rs` | T1/T3 worker 带头 |
| `engine/tests/engine_facade.rs`（或新建 `engine/tests/enqueue_auth.rs`） | T4 |
| `app/lib/engine/models/download_auth.dart` | Dart `DownloadAuth` |
| `app/lib/engine/native_bindings.dart` / `engine_host.dart` | FFI 入队 + `sniffUrls` 已有则接到 repository |
| `app/lib/providers/engine_repository.dart` | `sniffUrls`、`enqueue*` + `DownloadAuth?` |
| `app/lib/features/browse/browse_url.dart` | 地址栏校验 |
| `app/lib/features/browse/hook_to_sniff.dart` | Hook → `SniffEvent` |
| `app/lib/features/browse/sniff_accumulator.dart` | 500 条 + 顶层导航清空 |
| `app/lib/features/browse/browse_chrome.dart` | 地址栏/导航按钮 |
| `app/lib/features/browse/sniff_candidate_list.dart` | 候选列表 |
| `app/lib/features/browse/browse_screen.dart` | WebView + 解析本页 |
| `app/lib/features/browse/browse_unavailable_screen.dart` | TV 说明 |
| `app/lib/providers/browse_resolve_provider.dart` | 向导入参（outcome + auth） |
| `app/lib/providers/device_profile.dart` | `isTelevisionProvider` |
| `app/lib/shell/app_shell.dart` / `router.dart` | 五栏 + `/browse` + `/browse/wizard` |
| `app/lib/features/add/resolve_wizard.dart` | NeedsBrowser CTA |
| `app/lib/features/settings/settings_screen.dart` | 清除浏览 Cookie |
| `platforms/webview_sniff/` | 四端请求观察插件 |
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
            for stmt in [
                "ALTER TABLE download_tasks ADD COLUMN cookie_header TEXT;",
                "ALTER TABLE download_tasks ADD COLUMN referer TEXT;",
            ] {
                if let Err(e) = tx.execute(stmt, []) {
                    if !e.to_string().contains("duplicate column name") {
                        return Err(EngineError::Db(e));
                    }
                }
            }
            tx.execute("PRAGMA user_version = 3", [])?;
            tx.commit()?;
        }
        Ok(())
```

`row_to_task`：SELECT 在 `updated_at_ms` 后加 `cookie_header, referer`（下标 15、16）。**所有** `SELECT id, parent_id, ... updated_at_ms FROM download_tasks` 必须同步加这两列（`get` / `list_all` / `list_children` / `list_runnable_tasks`）。

`upsert_conn` INSERT/UPDATE 增加 `cookie_header, referer` 绑定 `task.cookie_header`、`task.referer`。

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

在 `engine/tests/download_integration.rs` 追加 T1（fixture 要求 Cookie 否则 403）：

```rust
#[test]
fn mp4_download_sends_enqueued_cookie() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        // 本测试在 Task 3 接上 enqueue auth 后才会绿；若本任务尚未改 enqueue，
        // 先用 TaskStore 直接 upsert 带 cookie_header 的任务再 start_downloads。
        let mut fx = EngineFixture::open();
        let (addr, _guard) = serve_cookie_mp4().await;
        let url = format!("http://{addr}/secret.mp4");
        let store = TaskStore::open(&fx.data_dir().join("tasks.db")).unwrap();
        let mut task = store
            .get(
                // 先 enqueue_single 无 auth 得到 id，再 upsert 补 cookie
                &{
                    fx.engine.enqueue_single("authed", &url, None).unwrap()
                },
            )
            .unwrap();
        task.cookie_header = Some("sid=ok".into());
        task.referer = Some(format!("http://{addr}/page"));
        store.upsert(&task).unwrap();
        fx.engine.start_downloads().unwrap();
        wait_for_task(&fx.engine, &task.id, TaskStatus::Completed, Duration::from_secs(30)).await;
    });
}
```

**不要**在 Step 1 依赖尚未存在的 `enqueue_single(..., Some(auth))`。T3：无 cookie 的公开 `sample.mp4` 既有 `mp4_download_registers` 必须保持绿色。

在 `engine/tests/download_integration.rs` 增加：

```rust
async fn serve_cookie_mp4() -> (std::net::SocketAddr, fixture_server::ServerGuard) {
    use axum::{
        body::Body,
        http::{header, HeaderMap, StatusCode},
        response::Response,
        routing::get,
        Router,
    };
    use tokio::net::TcpListener;

    let bytes = std::fs::read(fixture_server::fixtures_dir().join("sample.mp4")).unwrap();
    let router = Router::new().route(
        "/secret.mp4",
        get(move |headers: HeaderMap| {
            let bytes = bytes.clone();
            async move {
                let ok = headers
                    .get(header::COOKIE)
                    .and_then(|v| v.to_str().ok())
                    .is_some_and(|v| v.contains("sid=ok"));
                if !ok {
                    return Response::builder()
                        .status(StatusCode::FORBIDDEN)
                        .body(Body::empty())
                        .unwrap();
                }
                Response::builder()
                    .status(StatusCode::OK)
                    .header(header::CONTENT_TYPE, "video/mp4")
                    .body(Body::from(bytes))
                    .unwrap()
            }
        }),
    );
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let handle = tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    (addr, fixture_server::ServerGuard(handle))
}
```

若 `ServerGuard` 的内部 `JoinHandle` 为私有，把该函数放到 `engine/tests/support/fixture_server.rs` 并公开构造。无 Cookie 的 `mp4_download_registers` 必须保持绿色（T3）。

- [ ] **Step 2: 运行确认 HTTP 单测失败**

```bash
cargo test --manifest-path engine/Cargo.toml -p video_sniffing_engine get_stream_sends_client_auth -- --nocapture
```

Expected: `with_auth` 未定义 或 断言失败。

- [ ] **Step 3: 实现 `HttpClient` 鉴权并让 worker 使用**

`HttpClient` 增加字段 `cookies: Option<String>`、`referer: Option<String>`，`new` 里置 `None`。

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

Expected: PASS，含 `get_stream_sends_client_auth`、`mp4_download_sends_enqueued_cookie`、`mp4_download_registers`。

- [ ] **Step 5: Commit**

```bash
git add engine/src/download/http.rs engine/src/download/worker.rs engine/tests
git commit -m "$(cat <<'EOF'
feat(engine): 下载请求附带任务 Cookie 与 Referer 快照

EOF
)"
```

---

### Task 3: enqueue API + FFI（T4 + 接上入队鉴权）

**Files:**
- Modify: `engine/src/engine.rs`
- Modify: `engine/ffi/src/sync_dispatch.rs`
- Modify: `engine/ffi/tests/sync_tasks_test.rs`、`engine/ffi/tests/start_downloads_prepare_test.rs`（`engine_enqueue_single` 多一个 `ptr::null()`）
- Modify: 所有 `enqueue_single(` / `enqueue_episodes(` Rust 调用点加 `None` auth
- Test: `engine/tests/enqueue_auth.rs`（新建）

**Interfaces:**
- Consumes: `DownloadAuth`
- Produces:
  ```rust
  pub fn enqueue_single(&mut self, title: &str, url: &str, quality_label: Option<&str>, auth: Option<&DownloadAuth>) -> Result<String, EngineError>;
  pub fn enqueue_episodes(&mut self, list_title: &str, season: Option<u32>, episodes: &[(u32, String, String)], quality_label: Option<&str>, auth: Option<&DownloadAuth>) -> Result<(String, Vec<String>), EngineError>;
  ```
  FFI：`engine_enqueue_single(..., opts_json: *const c_char)`；`EnqueueEpisodesArgs` 增加 `cookies`/`referer`

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
```

FFI 测试（可放同文件或 `engine/ffi/tests`）：`engine_list_tasks` 返回 JSON `data` 数组元素 **不含** `sid=ok` 字符串。

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

父任务与子任务使用 **同一** `auth` 克隆。继续走 `upsert_parent_with_children` 单事务。

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

把旧调用点第四个/第五个参数补 `None`。FFI 测试调用增加 `std::ptr::null()`。

将 Task 2 的 `mp4_download_sends_enqueued_cookie` 改为直接：

```rust
        let auth = DownloadAuth {
            cookies: Some("sid=ok".into()),
            referer: Some(format!("http://{addr}/page")),
        };
        let id = fx
            .engine
            .enqueue_single("authed", &url, None, Some(&auth))
            .unwrap();
```

删除「先无 auth enqueue 再 TaskStore upsert」的过渡写法。

- [ ] **Step 4: 跑 workspace 测试**

```bash
cargo test --manifest-path engine/Cargo.toml --workspace
cargo clippy --manifest-path engine/Cargo.toml --all-targets --all-features -- -D warnings
```

Expected: PASS，clippy 无警告。

- [ ] **Step 5: Commit**

```bash
git add engine
git commit -m "$(cat <<'EOF'
feat(engine): 入队 API 与 FFI 接受可选下载鉴权

EOF
)"
```

---

### Task 4: Dart FFI、Repository、Fake

**Files:**
- Create: `app/lib/engine/models/download_auth.dart`
- Modify: `app/lib/engine/native_bindings.dart`
- Modify: `app/lib/engine/engine_host.dart`
- Modify: `app/lib/providers/engine_repository.dart`
- Modify: `app/test/fakes/fake_engine_repository.dart`
- Modify: 所有调用 `enqueueSingle` / `enqueueEpisodes` 的 Dart（添加页回调签名）

**Interfaces:**
- Produces:
  ```dart
  class DownloadAuth {
    const DownloadAuth({this.cookies, this.referer});
    final String? cookies;
    final String? referer;
    Map<String, dynamic> toJson() => {
      if (cookies != null) 'cookies': cookies,
      if (referer != null) 'referer': referer,
    };
  }
  ```
  ```dart
  String enqueueSingle({required String title, required String url, String? qualityLabel, DownloadAuth? auth});
  EnqueueEpisodesResult enqueueEpisodes({..., DownloadAuth? auth});
  List<ResourceCandidate> sniffUrls(List<SniffEvent> events, {String? pageUrl});
  ```

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

- [ ] **Step 3: 实现绑定**

`EngineEnqueueSingleNative` 增加第 5 个 `Pointer<Utf8> optsJson`（可为 nullptr）。

`EngineHost.enqueueSingle`：`auth == null` 传 nullptr，否则 `jsonEncode(auth.toJson()).toNativeUtf8()` 并 `malloc.free`。

`enqueueEpisodes` 现有 args map 增加：

```dart
    if (auth?.cookies != null) 'cookies': auth!.cookies,
    if (auth?.referer != null) 'referer': auth!.referer,
```

`EngineRepository` / `EngineHostRepository`：`sniffUrls` 转调已有 `EngineHost.sniffUrls`。

`ResolveWizard` 的 typedef 为 `enqueueSingle` / `enqueueEpisodes` 增加可选命名参数 `DownloadAuth? auth`（默认 `null`）。**添加页不传 auth**。

Fake：实现新方法；`lastEnqueueAuth`；`sniffUrls` 默认返回 `const []`，可在测试里赋值 `sniffResult`。

- [ ] **Step 4: `cd app && flutter test`**

Expected: 既有测试全绿 + 新测试 PASS。

- [ ] **Step 5: Commit**

```bash
git add app/lib app/test
git commit -m "$(cat <<'EOF'
feat(app): EngineRepository 支持嗅探与入队鉴权

EOF
)"
```

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
- Create: `app/lib/providers/device_profile.dart`
- Modify: `app/lib/shell/app_shell.dart`
- Modify: `app/lib/router.dart`
- Create: `app/lib/features/browse/browse_unavailable_screen.dart`
- Create: `app/lib/features/browse/browse_screen.dart`（本任务可先占位 Scaffold「浏览」，WebView 在 Task 9/11 接入）
- Test: `app/test/app_shell_test.dart`

**Interfaces:**
- Produces: `isTelevisionProvider = Provider<bool>((ref) => false);` 测试 override 为 `true`
- Destinations 顺序文案：`片库`,`浏览`,`任务`,`添加`,`设置`
- Branch 顺序与 destination **一致**
- `/browse`、`/browse/wizard`（wizard 用 `parentNavigatorKey: _rootNavigatorKey`）

- [ ] **Step 1: 改 `app_shell_test.dart`**

五个 `StatefulShellBranch`（library, browse, tasks, add, settings）。断言 `find.text('浏览')`。再写一条：`ProviderScope(overrides: [isTelevisionProvider.overrideWithValue(true)])` 下 **找不到** 文案 `浏览`。

若 AppShell 目前无 Riverpod，改为 `ConsumerWidget` 并 `ref.watch(isTelevisionProvider)`。

- [ ] **Step 2: `cd app && flutter test test/app_shell_test.dart`** → FAIL（仍四栏）

- [ ] **Step 3: 实现五栏；TV 时 destinations 与 `goBranch` 索引去掉浏览（4 项：片库/任务/添加/设置），`/browse` 仍指向 `BrowseUnavailableScreen`（文案：`请在手机或电脑使用内置浏览`）**

占位 `BrowseScreen`：`Scaffold(appBar: AppBar(title: Text('浏览')), body: BrowseChrome(...))` 即可，WebView 用 `SizedBox.shrink()` 占位。

`/browse/wizard`：读 `browseResolveProvider`，为 null 则 `pop`；否则嵌 `ResolveWizard`。本任务可用空 provider。

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
- Test: `app/test/browse_resolve_test.dart`

**Interfaces:**
- `browseResolveProvider`：`StateProvider<BrowseResolveArgs?>`  
  `class BrowseResolveArgs { ResolveOutcome outcome; DownloadAuth auth; ResolveOptions resolveOpts; }`
- `CookieExporter`：`Future<String?> cookieHeaderFor(Uri page)`
- 解析本页：`page_url = referer = currentUrl`；`resolveUrl(currentUrl, opts)`；成功 `browseResolveProvider.notifier.state = args` 然后 `context.push('/browse/wizard')`
- 向导入队：`enqueueSingle(..., auth: args.auth)`；HLS `resolveQualities(url, opts: args.resolveOpts)`
- 添加页路径：auth 仍为 null（W9 后半）

- [ ] **Step 1: W8/W9 widget 测试**

使用 `ProviderScope(overrides: [engineRepositoryProvider.overrideWithValue(fake)])`。`BrowseScreen` 构造注入 `CookieExporter`（测试返回 `'sid=ok'`）和假 `WebView` 导航状态（`currentUrl` 设为 `http://x/page`）。点 `Key('browse_resolve_page')` 后：

```dart
expect(fake.lastResolveOpts?.cookies, 'sid=ok');
```

入队：让 fake `resolveUrl` 返回 Single mp4，向导点下载后：

```dart
expect(fake.lastEnqueueAuth?.cookies, 'sid=ok');
```

另写：直接泵 `AddScreen` 流程（可只调 fake.enqueueSingle 不经浏览）`lastEnqueueAuth == null`。若 Add 集成过重，则单测 `EngineHostRepository` 不强制；W9 用 Fake 在「模拟添加页回调」`enqueueSingle(auth: null)`。

- [ ] **Step 2: 测试失败**

- [ ] **Step 3: 实现会话 Notifier**

顶层 URL 变化：`accumulator.onTopLevelNavigation()`。钩子事件 debounce **300ms** 后 `repo.sniffUrls(acc.events, pageUrl: currentUrl)` 更新 `candidates`。

`/browse?url=`：`initState`/`didChangeDependencies` 里 `parseBrowseUrl` 成功则加载（Task 11 真正 loadRequest）。

Wizard 页：`NeedsBrowser` **不**传 `onOpenBrowser`。

- [ ] **Step 4: `cd app && flutter test`** PASS

- [ ] **Step 5: Commit** `feat(app): 解析本页并带着鉴权快照入队`

---

### Task 10: 设置清除浏览 Cookie

**Files:**
- Modify: `app/lib/features/settings/settings_screen.dart`
- Create: `app/lib/features/browse/cookie_store.dart`（对 `WebViewCookieManager.clearCookies` 的薄封装，测试可 fake）
- Test: `app/test/settings_clear_cookies_test.dart`

**Interfaces:**
- `abstract class BrowseCookieStore { Future<void> clearAll(); }`
- `browseCookieStoreProvider`
- 按钮 `Key('settings_clear_browse_cookies')` 文案 **清除浏览 Cookie**；成功 SnackBar **已清除浏览 Cookie**
- **不**清任务库快照；无 Cookie 文本框

- [ ] **Step 1: 测试 tap 后 fake.clearAll 被调用且 SnackBar 文案正确**

- [ ] **Step 2: FAIL**

- [ ] **Step 3: 实现**；生产实现调用 `WebViewCookieManager().clearCookies()`（依赖 Task 11 的 webview_flutter 时：若本任务先于插件，先提供 in-memory fake 生产实现，Task 11 再换成真实 CookieManager）

- [ ] **Step 4: PASS**

- [ ] **Step 5: Commit** `feat(app): 设置页可清除浏览 Cookie`

---

### Task 11: webview_flutter + Android/iOS 钩子插件

**Files:**
- Modify: `app/pubspec.yaml` 增加：
  ```yaml
    webview_flutter: ^4.10.0
    webview_sniff:
      path: ../platforms/webview_sniff
  ```
- Create: `platforms/webview_sniff/` 标准 Flutter 插件（Android + iOS；macOS/Windows 先 method 未实现返回空流）
- Create: `platforms/webview_sniff/lib/webview_sniff.dart`
- Modify: `app/lib/features/browse/browse_screen.dart` 接入 `WebViewWidget`
- Test: `platforms/webview_sniff/test/hook_to_sniff_test.dart` 可省略（映射已在 app 测）；app 侧用 fake stream

**Interfaces:**
- ```dart
  class WebViewSniff {
    static Stream<HookRequest> attach(WebViewController controller);
  }
  ```
- Android：对 **同一** WebView `shouldInterceptRequest` 观察后原样放行（返回 `null` 让系统继续）。EventChannel 事件 JSON：`url`,`page_url`,`is_main_frame`,`mime`。**禁止**把 Cookie 头放进 payload。
- iOS：`WKNavigationDelegate` + 注入脚本监听 `fetch`/`XHR`/`HTMLMediaElement.src`，同样 JSON。不保证 MSE。
- UA：`EngineSettings.user_agent` 非空则 `controller.setUserAgent`

- [ ] **Step 1: 用 `flutter create --template=plugin --platforms=android,ios,macos,windows platforms/webview_sniff` 后立刻改名为现有目录结构（若目录已存在则手写 `pubspec.yaml` `android/` `ios/`）**

插件 `pubspec.yaml` `name: webview_sniff`。

- [ ] **Step 2: Dart API + Android Kotlin 观察**

Kotlin 伪实现要点：`FlutterPlugin` + `EventChannel("webview_sniff/events")`；`attach` 通过 `webview_flutter_android` 拿到 `WebView` 或 `WebViewClient` 包装。若官方 API 无法挂钩，使用 `WebViewCompat`/`WebViewClientCompat` 在 Activity 的 platform view 创建后查找 `WebView` 实例。

iOS：`WKUserScript` atDocumentStart：

```javascript
(function() {
  const post = (url) => {
    try { webkit.messageHandlers.sniff.postMessage({url: String(url)}); } catch (e) {}
  };
  const origFetch = window.fetch;
  window.fetch = function() { try { post(arguments[0]); } catch (e) {} return origFetch.apply(this, arguments); };
})();
```

原生把 message 转 `HookRequest`（`is_main_frame: false`，mime 未知则空）。导航回调 `is_main_frame: true`。

- [ ] **Step 3: `BrowseScreen` `WebViewController` + `loadRequest`；订阅 `WebViewSniff.attach` → accumulator**

加载失败：body 显示错误 + 刷新按钮，不崩溃。

钩子未实现：列表空，「解析本页」仍可用。首次可用 `SnackBar` 一次：**本端嗅探能力有限**（用 `shared_preferences` 过重则 `static bool _hinted` 进程内一次即可）。

- [ ] **Step 4: `cd app && flutter test`；`cd app && flutter build apk --debug` 或至少 `flutter build ios --debug --no-codesign`（环境缺一则记录，不得伪称通过）**

- [ ] **Step 5: Commit** `feat(app): 接入 WebView 与 Android/iOS 请求钩子`

---

### Task 12: macOS / Windows 钩子

**Files:**
- Modify: `platforms/webview_sniff/macos/`
- Modify: `platforms/webview_sniff/windows/`
- `app/macos` / `app/windows` 插件登记（`flutter pub get` 生成文件可提交 **本插件相关** 行）

**Interfaces:** 与 Task 11 相同 `HookRequest` 流。Windows：WebView2 `WebResourceRequested` 只观察。macOS：同 iOS WK 脚本 + navigation delegate。

- [ ] **Step 1: 为 Windows 写一个原生侧单元难以自动化时，在 Dart 增加 `HookRequest.fromJson` 测试（已有则跳过）；Windows 实现后用本地 `flutter run -d windows` 手工打开 `https://example.com` 确认无崩溃**

- [ ] **Step 2: 实现两平台；未实现时 attach 返回 empty stream，不抛错**

- [ ] **Step 3: `cd app && flutter test`**

- [ ] **Step 4: Commit** `feat(app): macOS/Windows WebView 嗅探钩子`

---

### Task 13: U6 集成、文档、全量验证

**Files:**
- Modify: `app/integration_test/ui_test.dart` 或新建 `app/integration_test/browse_test.dart`
- Modify: `README.md`、`platforms/README.md`
- 规格状态行可改为「已完成」仅在全部绿之后（本任务文档）

**Interfaces:** U6：应用内浏览打开需 Cookie 的本地 HTML，点解析本页，得到非 `NeedsBrowser` **或** 明确 `Single`/`Candidates`（与 fixture 设计一致）。U7 可选：页面 `<video src=".../sample.mp4">`；macOS 钩不到则 skip，注释写明。

- [ ] **Step 1: fixture HTTP**（可复用引擎 fixture 思路，在 Dart `HttpServer.bind`）：

  - `GET /gate` 无 Cookie → 200 HTML 无媒体
  - `GET /gate` 有 `sid=ok` → 200 HTML 含 `http://127.0.0.1:port/clip.mp4` 或直接 mp4 链接
  - 测试里无法方便种 WebView Cookie 时：先 `javascript` 禁止；改为 gate **不**需 Cookie、仅验证「解析本页」对本地 HTML 调 `resolveUrl` 且 fake 已被集成测试里的真引擎替代。

  **U6 真引擎路径：** HTML 含直接 `http://127.0.0.1:port/sample.mp4`（与 Plan 5 U1 同源文件）。浏览 load 该 HTML → 解析本页 → 向导出现「下载」。Cookie fixture 作为加分：通过 `CookieManager.setCookie` 种 `sid=ok` 再解析需 Cookie 的页。

- [ ] **Step 2: 本地运行**

```bash
cd app && flutter test integration_test/browse_test.dart -d macos
```

CI 无 GUI 不稳定则与 U3 一样提供 `INTEGRATION_SKIP_BROWSE`；**默认文档要求本地 U6 绿**。

- [ ] **Step 3: README** 主流程增加：添加失败 `NeedsBrowser` → 浏览登录 → 解析本页 / 候选。`platforms/README.md` 改为指向 `webview_sniff` 插件，并写明 Share 仍未实现。

- [ ] **Step 4: 全量**

```bash
cargo fmt --manifest-path engine/Cargo.toml --all -- --check
cargo test --manifest-path engine/Cargo.toml --workspace
cargo clippy --manifest-path engine/Cargo.toml --all-targets --all-features -- -D warnings
cd app && flutter test
```

Expected: 全部 PASS。

- [ ] **Step 5: Commit** `docs: 补充 Plan 6a 浏览主路径与 U6 集成测试`

---

## 任务依赖

```
1 schema/脱敏 → 2 worker HTTP → 3 enqueue/FFI → 4 Dart repository
4 → 5 chrome → 6 shell → 7 NeedsBrowser
4 → 8 sniff UI
6 + 8 + 7 → 9 resolve/enqueue UI
9 → 10 settings（可与 11 并行）
9 → 11 WebView Android/iOS → 12 desktop hooks → 13 U6/docs
```

---

## Self-review（对照规格）

| 规格 | 任务 |
|------|------|
| 五栏浏览、TV 隐藏 | 6 |
| 地址栏 http(s)、拒绝 javascript/file/data | 5 W6 |
| NeedsBrowser CTA | 7 W5 |
| 解析本页 Cookie/Referer/page_url | 9 W8 |
| 向导 Riverpod 不传 Cookie query | 9 |
| 嗅探 500/300ms/顶层清空 | 8+9 |
| 任务快照 + JSON 脱敏 | 1 T2 |
| worker 带头、无 auth 旧行为 | 2 T1/T3 |
| enqueue FFI / episodes JSON | 3 T4 |
| 清除浏览 Cookie | 10 |
| 四端钩子、不改 body、无 Cookie payload | 11–12 |
| U6、README | 13 |
| 无 Share/LAN/TV 浏览/抓包 | Global Constraints |
