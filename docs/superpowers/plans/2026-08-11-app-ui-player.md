# App UI + Player Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 在 Plan 4 `EngineHost` 基础上交付四端可用的 Flutter 应用：片库/任务/添加/设置 UI、粘贴 URL 解析入队、本地播放器与进度回写，并通过 W1–W4 + U1–U3 测试。

**Architecture:** `flutter_riverpod` 管理 `EngineHost` 单例与列表 Provider；`DownloadCoordinator` 订阅 `taskEvents` 并 `invalidate` 刷新；`go_router` + 自适应 `AppShell`；`media_kit` 播放 `LibraryEpisode.file_path`；Feature 层经 `EngineRepository` 抽象访问引擎（Widget 测试可 mock）。

**Tech Stack:** Flutter stable、flutter_riverpod ^2.6、go_router ^14、media_kit ^1.2、media_kit_video、media_kit_libs_video、path_provider、现有 Cargokit/EngineHost

**规格:** `docs/superpowers/specs/2026-08-11-app-ui-player-design.md`

**修订:** 2026-08-11 plan review（引擎门闩、NavigationBar、DownloadCoordinator eager watch、W1/W3/W4/U1–U3 测试修正、Candidates resolveQualities、播放器进度持久化、HLS 验收）

## Global Constraints

- Plan 5：**B+**；**不修改** `engine/` 与 FFI 边界（除非播放器集成发现阻断性缺口）
- 状态管理：**flutter_riverpod**；路由：**go_router**；播放器：**media_kit** + media_kit_video + media_kit_libs_video
- `main()` 调用 `WidgetsFlutterBinding.ensureInitialized()`、`MediaKit.ensureInitialized()`、`ProviderScope`
- Feature 禁止直接 `EngineHost.open()`；经 `EngineRepository` / Provider
- **`engineHostProvider` 未 `data` 前不得渲染 Feature**；`VideoSniffingApp` 显示 loading/error 门闩
- **`VideoSniffingApp` 在 `data` 态必须 `ref.watch(downloadCoordinatorProvider)`**（eager 订阅 taskEvents）
- 窄屏 `width < 600` → Material 3 **`NavigationBar`**；宽屏 `width >= 600` → **`NavigationRail`**
- 下载：`ensureDownloads()` 在入队后调用；`downloads already running` 静默忽略
- `total_bytes == null` 时进度条用 indeterminate，**禁止除零**
- 播放器离开页时 `setEpisodePosition`；`file_path` 为引擎 canonicalize 绝对路径
- **不实现** WebView、Share、LAN、TV UI、字幕、PiP
- **不暴露** `register_completed_*`、`drain_downloads_for_test`
- 验证：
  ```bash
  cargo fmt --manifest-path engine/Cargo.toml --all -- --check
  cargo test --manifest-path engine/Cargo.toml --workspace
  cargo clippy --manifest-path engine/Cargo.toml --all-targets --all-features -- -D warnings
  cd app && flutter pub get && flutter test
  cd app && flutter test integration_test/smoke_test.dart -d macos
  cd app && flutter test integration_test/ui_test.dart -d macos
  ```
- `docs/` 在 `.gitignore`；计划/规格用 `git add -f` 提交

---

## File Map

| 路径 | 职责 |
|------|------|
| `app/pubspec.yaml` | 新增 riverpod、go_router、media_kit 依赖 |
| `app/lib/main.dart` | bootstrap：MediaKit + ProviderScope + `VideoSniffingApp` |
| `app/lib/app.dart` | 引擎就绪门闩 + `MaterialApp.router` + eager `downloadCoordinator` |
| `app/lib/router.dart` | go_router 路由表 |
| `app/lib/shell/app_shell.dart` | NavigationBar / NavigationRail |
| `app/lib/providers/engine_repository.dart` | `EngineRepository` 抽象 + `EngineHostRepository` |
| `app/lib/providers/engine_host_provider.dart` | `engineHostProvider` FutureProvider |
| `app/lib/providers/download_coordinator.dart` | taskEvents 订阅、worker 启停 |
| `app/lib/providers/library_provider.dart` | `libraryProvider` |
| `app/lib/providers/tasks_provider.dart` | `tasksProvider` |
| `app/lib/providers/settings_provider.dart` | `settingsProvider` |
| `app/lib/ui/error_presenter.dart` | `EngineException` → 用户文案 |
| `app/lib/ui/loading_overlay.dart` | 全屏 loading |
| `app/lib/features/library/` | 片库列表 + 详情 |
| `app/lib/features/tasks/` | 任务列表 + TaskTile |
| `app/lib/features/add/` | 添加 + ResolveWizard |
| `app/lib/features/settings/` | 设置表单 |
| `app/lib/features/player/` | media_kit 播放器 |
| `app/test/` | W1–W4 widget 测试 |
| `app/integration_test/ui_test.dart` | U1–U3 UI 集成测试 |
| `app/integration_test/smoke_test.dart` | F1–F5（保留） |
| `.github/workflows/ci.yml` | flutter-smoke 增加 ui_test |
| `README.md` | Plan 5 主流程说明 |

---

### Task 1: 依赖与应用 Bootstrap

**Files:**
- Modify: `app/pubspec.yaml`
- Modify: `app/lib/main.dart`
- Create: `app/lib/app.dart`

**Interfaces:**
- Consumes: 现有 `EngineHost`、`openNativeLibrary()`
- Produces: `VideoSniffingApp` widget；`main()` 不再在启动时 open/dispose 做 smoke

- [ ] **Step 1: 更新 `app/pubspec.yaml` dependencies**

在 `dependencies:` 追加：

```yaml
  flutter_riverpod: ^2.6.1
  go_router: ^14.6.2
  media_kit: ^1.2.0
  media_kit_video: ^1.3.0
  media_kit_libs_video: ^1.0.5
```

- [ ] **Step 2: 创建 `app/lib/app.dart`（含引擎门闩 + eager DownloadCoordinator）**

```dart
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'providers/download_coordinator.dart';
import 'providers/engine_host_provider.dart';
import 'router.dart';

class VideoSniffingApp extends ConsumerWidget {
  const VideoSniffingApp({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final hostAsync = ref.watch(engineHostProvider);

    return hostAsync.when(
      loading: () => const MaterialApp(
        home: Scaffold(
          body: Center(
            child: Column(
              mainAxisSize: MainAxisSize.min,
              children: [
                CircularProgressIndicator(),
                SizedBox(height: 16),
                Text('正在初始化引擎…'),
              ],
            ),
          ),
        ),
      ),
      error: (error, _) => MaterialApp(
        home: Scaffold(
          body: Center(
            child: Column(
              mainAxisSize: MainAxisSize.min,
              children: [
                Text('引擎初始化失败：$error'),
                const SizedBox(height: 16),
                FilledButton(
                  onPressed: () => ref.invalidate(engineHostProvider),
                  child: const Text('重试'),
                ),
              ],
            ),
          ),
        ),
      ),
      data: (_) {
        // Eager：应用生命周期内订阅 taskEvents
        ref.watch(downloadCoordinatorProvider);
        final router = ref.watch(appRouterProvider);
        return MaterialApp.router(
          title: 'Video Sniffing',
          theme: ThemeData(
            colorScheme: ColorScheme.fromSeed(seedColor: Colors.deepPurple),
            useMaterial3: true,
          ),
          routerConfig: router,
        );
      },
    );
  }
}
```

> Task 1 时 `engine_host_provider.dart` / `download_coordinator.dart` 尚未存在，可先提交占位 `app.dart` 在 Task 3 再切换为上述完整版；或 Task 1–3 合并为一次提交。推荐 **Task 3 完成后** 将 `app.dart` 更新为本版本。

- [ ] **Step 3: 替换 `app/lib/main.dart`**

```dart
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:media_kit/media_kit.dart';

import 'app.dart';
import 'engine/native_bindings.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  MediaKit.ensureInitialized();
  openNativeLibrary();
  runApp(const ProviderScope(child: VideoSniffingApp()));
}
```

- [ ] **Step 4: 创建占位 `app/lib/router.dart`（Task 4 完善）**

```dart
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

final appRouterProvider = Provider<GoRouter>((ref) {
  return GoRouter(
    initialLocation: '/library',
    routes: [
      GoRoute(
        path: '/library',
        builder: (context, state) =>
            const Scaffold(body: Center(child: Text('bootstrap ok'))),
      ),
    ],
  );
});
```

- [ ] **Step 5: Run**

```bash
cd app && flutter pub get && flutter analyze
```

Expected: 无 error（router 占位可 analyze 通过）

- [ ] **Step 6: Commit**

```bash
git add app/pubspec.yaml app/pubspec.lock app/lib/main.dart app/lib/app.dart app/lib/router.dart
git commit -m "feat(app): 添加 Plan 5 依赖与 bootstrap 脚手架"
```

---

### Task 2: ErrorPresenter 与 EngineRepository

**Files:**
- Create: `app/lib/ui/error_presenter.dart`
- Create: `app/lib/providers/engine_repository.dart`
- Create: `app/lib/providers/engine_host_provider.dart`
- Test: `app/test/error_presenter_test.dart`
- Test: `app/test/fakes/fake_engine_repository.dart`

**Interfaces:**
- Consumes: `EngineHost`、`EngineException`、`FfiError`、`ResolveOutcome*`、`EnqueueEpisodesResult`
- Produces:
  - `String? presentEngineError(EngineException e)`（`null` = 不向用户展示）
  - `abstract class EngineRepository`（完整方法列表见 Step 3）
  - `class EngineHostRepository implements EngineRepository`
  - `final engineHostProvider = FutureProvider<EngineHost>(...)`
  - `final engineRepositoryProvider = Provider<EngineRepository>(...)`

- [ ] **Step 1: 写失败测试 `app/test/error_presenter_test.dart`**

```dart
import 'package:flutter_test/flutter_test.dart';
import 'package:video_sniffing/engine/engine_host.dart';
import 'package:video_sniffing/engine/models/ffi_response.dart';
import 'package:video_sniffing/ui/error_presenter.dart';

void main() {
  test('maps http kind to network message', () {
    final message = presentEngineError(
      EngineException(const FfiError(kind: 'http', message: 'timeout')),
    );
    expect(message, contains('网络'));
  });

  test('maps invalid_arg to raw message', () {
    final message = presentEngineError(
      EngineException(
        const FfiError(kind: 'invalid_arg', message: 'media_dir invalid'),
      ),
    );
    expect(message, 'media_dir invalid');
  });

  test('suppresses downloads already running', () {
    expect(
      presentEngineError(
        EngineException(
          const FfiError(
            kind: 'message',
            message: 'invalid argument: downloads already running',
          ),
        ),
      ),
      isNull,
    );
  });
}
```

- [ ] **Step 2: Run 确认 FAIL**

```bash
cd app && flutter test test/error_presenter_test.dart
```

Expected: FAIL `presentEngineError` not found

- [ ] **Step 3: 实现 `app/lib/ui/error_presenter.dart`**

```dart
import 'package:video_sniffing/engine/engine_host.dart';

/// 返回 `null` 表示无需向用户展示（如 downloads already running）。
String? presentEngineError(EngineException exception) {
  final error = exception.error;
  final lower = error.message.toLowerCase();
  if (lower.contains('downloads already running')) {
    return null;
  }
  switch (error.kind) {
    case 'http':
      return '网络请求失败，请检查连接后重试';
    case 'not_found':
      return '找不到对应内容';
    case 'invalid_arg':
      return error.message;
    case 'db':
    case 'io':
      return '本地存储异常：${error.message}';
    default:
      return error.message;
  }
}
```

- [ ] **Step 4: Run 测试 PASS**

```bash
cd app && flutter test test/error_presenter_test.dart
```

- [ ] **Step 5: 创建 `app/lib/providers/engine_repository.dart`**

```dart
import 'package:video_sniffing/engine/engine_host.dart';
import 'package:video_sniffing/engine/models/download_task.dart';
import 'package:video_sniffing/engine/models/engine_settings.dart';
import 'package:video_sniffing/engine/models/library_episode.dart';
import 'package:video_sniffing/engine/models/library_item.dart';
import 'package:video_sniffing/engine/models/resolve_types.dart';
import 'package:video_sniffing/engine/models/sniff_types.dart';
import 'package:video_sniffing/engine/models/task_event.dart';

abstract class EngineRepository {
  Stream<TaskEvent> get taskEvents;

  EngineSettings settings();
  void saveSettings(EngineSettings settings);
  List<LibraryItem> listLibrary();
  List<LibraryEpisode> listEpisodes(String itemId);
  List<DownloadTask> listTasks();

  String enqueueSingle({
    required String title,
    required String url,
    String? qualityLabel,
  });

  EnqueueEpisodesResult enqueueEpisodes({
    required String listTitle,
    int? season,
    required List<(int index, String title, String url)> episodes,
    String? qualityLabel,
  });

  void startDownloads();
  void pauseTask(String taskId);
  void resumeTask(String taskId);
  void cancelTask(String taskId);
  void setEpisodePosition(String episodeId, int positionMs);

  Future<ResolveOutcome> resolveUrl(String url, {ResolveOptions? opts});
  Future<List<Quality>> resolveQualities(String mediaUrl, {ResolveOptions? opts});
}

class EngineHostRepository implements EngineRepository {
  EngineHostRepository(this._host);

  final EngineHost _host;

  @override
  Stream<TaskEvent> get taskEvents => _host.taskEvents;

  @override
  EngineSettings settings() => _host.settings();

  @override
  void saveSettings(EngineSettings settings) => _host.saveSettings(settings);

  @override
  List<LibraryItem> listLibrary() => _host.listLibrary();

  @override
  List<LibraryEpisode> listEpisodes(String itemId) => _host.listEpisodes(itemId);

  @override
  List<DownloadTask> listTasks() => _host.listTasks();

  @override
  String enqueueSingle({
    required String title,
    required String url,
    String? qualityLabel,
  }) =>
      _host.enqueueSingle(title: title, url: url, qualityLabel: qualityLabel);

  @override
  EnqueueEpisodesResult enqueueEpisodes({
    required String listTitle,
    int? season,
    required List<(int index, String title, String url)> episodes,
    String? qualityLabel,
  }) =>
      _host.enqueueEpisodes(
        listTitle: listTitle,
        season: season,
        episodes: episodes,
        qualityLabel: qualityLabel,
      );

  @override
  void startDownloads() => _host.startDownloads();

  @override
  void pauseTask(String taskId) => _host.pauseTask(taskId);

  @override
  void resumeTask(String taskId) => _host.resumeTask(taskId);

  @override
  void cancelTask(String taskId) => _host.cancelTask(taskId);

  @override
  void setEpisodePosition(String episodeId, int positionMs) =>
      _host.setEpisodePosition(episodeId, positionMs);

  @override
  Future<ResolveOutcome> resolveUrl(String url, {ResolveOptions? opts}) =>
      _host.resolveUrl(url, opts: opts);

  @override
  Future<List<Quality>> resolveQualities(
    String mediaUrl, {
    ResolveOptions? opts,
  }) =>
      _host.resolveQualities(mediaUrl, opts: opts);
}
```

- [ ] **Step 6: 创建 `app/lib/providers/engine_host_provider.dart`**

```dart
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:path_provider/path_provider.dart';
import 'package:video_sniffing/engine/engine_host.dart';
import 'package:video_sniffing/providers/engine_repository.dart';

final engineHostProvider = FutureProvider<EngineHost>((ref) async {
  final dir = await getApplicationSupportDirectory();
  final host = await EngineHost.open(dir.path);
  ref.onDispose(host.dispose);
  return host;
});

final engineRepositoryProvider = Provider<EngineRepository>((ref) {
  final host = ref.watch(engineHostProvider).requireValue;
  return EngineHostRepository(host);
});
```

- [ ] **Step 7: 创建 `app/test/fakes/fake_engine_repository.dart`**

`FakeEngineRepository.saveSettings` **必须复刻** `engine/src/settings.rs` 的 `validate_media_dir` 规则（供 W4 使用）：

```dart
import 'dart:async';

import 'package:video_sniffing/engine/engine_host.dart';
import 'package:video_sniffing/engine/models/ffi_response.dart';
import 'package:video_sniffing/engine/models/download_task.dart';
import 'package:video_sniffing/engine/models/engine_settings.dart';
import 'package:video_sniffing/engine/models/library_episode.dart';
import 'package:video_sniffing/engine/models/library_item.dart';
import 'package:video_sniffing/engine/models/resolve_types.dart';
import 'package:video_sniffing/engine/models/task_event.dart';
import 'package:video_sniffing/providers/engine_repository.dart';

void validateMediaDirForTest(String name) {
  if (name.isEmpty) {
    throw EngineException(
      const FfiError(kind: 'invalid_arg', message: 'media_dir must not be empty'),
    );
  }
  if (name == '.' ||
      name.startsWith('/') ||
      name.contains('..') ||
      name.contains('/') ||
      name.contains('\\')) {
    throw EngineException(
      const FfiError(
        kind: 'invalid_arg',
        message: 'media_dir must be a single relative directory name',
      ),
    );
  }
}

class FakeEngineRepository implements EngineRepository {
  FakeEngineRepository({
    this.settingsValue = EngineSettings.defaults,
    this.libraryItems = const [],
    this.tasks = const [],
  });

  EngineSettings settingsValue;
  List<LibraryItem> libraryItems;
  List<DownloadTask> tasks;
  final _events = StreamController<TaskEvent>.broadcast();

  @override
  Stream<TaskEvent> get taskEvents => _events.stream;

  @override
  EngineSettings settings() => settingsValue;

  @override
  void saveSettings(EngineSettings settings) {
    validateMediaDirForTest(settings.mediaDir);
    settingsValue = settings;
  }

  @override
  List<LibraryItem> listLibrary() => libraryItems;

  @override
  List<LibraryEpisode> listEpisodes(String itemId) => [];

  @override
  List<DownloadTask> listTasks() => tasks;

  @override
  String enqueueSingle({
    required String title,
    required String url,
    String? qualityLabel,
  }) =>
      'fake-task-id';

  @override
  EnqueueEpisodesResult enqueueEpisodes({
    required String listTitle,
    int? season,
    required List<(int index, String title, String url)> episodes,
    String? qualityLabel,
  }) =>
      const EnqueueEpisodesResult(parentId: 'parent', childIds: ['c1']);

  @override
  void startDownloads() {}

  @override
  void pauseTask(String taskId) {}

  @override
  void resumeTask(String taskId) {}

  @override
  void cancelTask(String taskId) {}

  @override
  void setEpisodePosition(String episodeId, int positionMs) {}

  @override
  Future<ResolveOutcome> resolveUrl(String url, {ResolveOptions? opts}) async {
    return ResolveOutcomeSingle(
      ResourceCandidate(
        id: '1',
        url: url,
        kind: MediaKind.mp4,
      ),
    );
  }

  @override
  Future<List<Quality>> resolveQualities(
    String mediaUrl, {
    ResolveOptions? opts,
  }) async =>
      [const Quality(label: '1080p')];

  void dispose() => _events.close();
}
```

（删除计划中旧的 `noSuchMethod` 空壳版本。）

- [ ] **Step 8: Commit**

```bash
git add app/lib/ui/error_presenter.dart app/lib/providers/ app/test/
git commit -m "feat(app): 添加 EngineRepository 与错误文案映射"
```

---

### Task 3: DownloadCoordinator 与数据 Provider

**Files:**
- Create: `app/lib/providers/download_coordinator.dart`
- Create: `app/lib/providers/library_provider.dart`
- Create: `app/lib/providers/tasks_provider.dart`
- Create: `app/lib/providers/settings_provider.dart`
- Test: `app/test/download_coordinator_test.dart`

**Interfaces:**
- Consumes: `EngineRepository`、`Ref`、`TaskStatus`、`TaskEventKind`
- Produces:
  - `class DownloadCoordinator { void ensureDownloads(); void dispose(); }`
  - `final downloadCoordinatorProvider`
  - `final libraryProvider = Provider<List<LibraryItem>>(...)`
  - `final tasksProvider = Provider<List<DownloadTask>>(...)`
  - `final settingsProvider = Provider<EngineSettings>(...)`

- [ ] **Step 1: 写失败测试 `app/test/download_coordinator_test.dart`**

```dart
import 'package:flutter_test/flutter_test.dart';
import '../fakes/fake_engine_repository.dart';
import 'package:video_sniffing/providers/download_coordinator.dart';

class _RecordingRepo extends FakeEngineRepository {
  int startCalls = 0;
  @override
  void startDownloads() => startCalls++;
}

void main() {
  test('ensureDownloads calls startDownloads when worker inactive', () {
    final repo = _RecordingRepo();
    final coordinator = DownloadCoordinator.forTest(
      repo,
      onInvalidateTasks: () {},
      onInvalidateLibrary: () {},
    );
    coordinator.ensureDownloads();
    expect(repo.startCalls, 1);
    coordinator.dispose();
  });
}
```

- [ ] **Step 2: Run FAIL**

```bash
cd app && flutter test test/download_coordinator_test.dart
```

- [ ] **Step 3: 实现 `app/lib/providers/download_coordinator.dart`**

`DownloadCoordinator` 提供 **生产构造** `DownloadCoordinator(Ref ref, EngineRepository repo)` 与 **测试构造** `DownloadCoordinator.forTest(repo, {required onInvalidateTasks, required onInvalidateLibrary})`，测试构造不依赖 `Ref`：

```dart
import 'dart:async';

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:video_sniffing/engine/engine_host.dart';
import 'package:video_sniffing/engine/models/task_event.dart';
import 'package:video_sniffing/engine/models/task_status.dart';
import 'package:video_sniffing/providers/engine_host_provider.dart';
import 'package:video_sniffing/providers/engine_repository.dart';
import 'package:video_sniffing/providers/library_provider.dart';
import 'package:video_sniffing/providers/tasks_provider.dart';

typedef _Invalidate = void Function();

class DownloadCoordinator {
  DownloadCoordinator._(
    this._repo, {
    required _Invalidate onInvalidateTasks,
    required _Invalidate onInvalidateLibrary,
  })  : _onInvalidateTasks = onInvalidateTasks,
        _onInvalidateLibrary = onInvalidateLibrary {
    _subscription = _repo.taskEvents.listen(_onEvent);
  }

  factory DownloadCoordinator(Ref ref, EngineRepository repo) {
    return DownloadCoordinator._(
      repo,
      onInvalidateTasks: () => ref.invalidate(tasksProvider),
      onInvalidateLibrary: () => ref.invalidate(libraryProvider),
    );
  }

  factory DownloadCoordinator.forTest(
    EngineRepository repo, {
    required _Invalidate onInvalidateTasks,
    required _Invalidate onInvalidateLibrary,
  }) {
    return DownloadCoordinator._(
      repo,
      onInvalidateTasks: onInvalidateTasks,
      onInvalidateLibrary: onInvalidateLibrary,
    );
  }

  final EngineRepository _repo;
  final _Invalidate _onInvalidateTasks;
  final _Invalidate _onInvalidateLibrary;
  StreamSubscription<TaskEvent>? _subscription;
  var _workerActive = false;

  void ensureDownloads() {
    if (_workerActive) {
      return;
    }
    try {
      _repo.startDownloads();
      _workerActive = true;
    } on EngineException catch (e) {
      final msg = e.error.message.toLowerCase();
      if (!msg.contains('downloads already running')) {
        rethrow;
      }
      _workerActive = true;
    }
  }

  void _onEvent(TaskEvent event) {
    switch (event.kind) {
      case TaskEventKind.workerStopped:
        _workerActive = false;
        _onInvalidateTasks();
        final queued = _repo
            .listTasks()
            .any((task) => task.status == TaskStatus.queued);
        if (queued) {
          ensureDownloads();
        }
      case TaskEventKind.taskUpdated:
        _onInvalidateTasks();
        final task = event.task;
        if (task != null && task.status == TaskStatus.completed) {
          _onInvalidateLibrary();
        }
    }
  }

  void dispose() {
    _subscription?.cancel();
  }
}

final downloadCoordinatorProvider = Provider<DownloadCoordinator>((ref) {
  final repo = ref.watch(engineRepositoryProvider);
  final coordinator = DownloadCoordinator(ref, repo);
  ref.onDispose(coordinator.dispose);
  return coordinator;
});
```

- [ ] **Step 4: 实现列表 Provider**

`app/lib/providers/library_provider.dart`:

```dart
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:video_sniffing/engine/models/library_item.dart';
import 'package:video_sniffing/providers/engine_host_provider.dart';
import 'package:video_sniffing/providers/engine_repository.dart';

final libraryProvider = Provider<List<LibraryItem>>((ref) {
  ref.watch(engineHostProvider);
  final repo = ref.watch(engineRepositoryProvider);
  final items = [...repo.listLibrary()]
    ..sort((a, b) => b.createdAtMs.compareTo(a.createdAtMs));
  return items;
});
```

`app/lib/providers/tasks_provider.dart`:

```dart
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:video_sniffing/engine/models/download_task.dart';
import 'package:video_sniffing/providers/engine_host_provider.dart';
import 'package:video_sniffing/providers/engine_repository.dart';

final tasksProvider = Provider<List<DownloadTask>>((ref) {
  ref.watch(engineHostProvider);
  final repo = ref.watch(engineRepositoryProvider);
  return repo.listTasks();
});
```

`app/lib/providers/settings_provider.dart`:

```dart
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:video_sniffing/engine/models/engine_settings.dart';
import 'package:video_sniffing/providers/engine_host_provider.dart';
import 'package:video_sniffing/providers/engine_repository.dart';

final settingsProvider = Provider<EngineSettings>((ref) {
  ref.watch(engineHostProvider);
  return ref.watch(engineRepositoryProvider).settings();
});
```

- [ ] **Step 4b: 将 `app/lib/app.dart` 更新为 Task 1 Step 2 完整版（引擎门闩 + eager `downloadCoordinatorProvider`）**

- [ ] **Step 5: Run PASS**

```bash
cd app && flutter test test/download_coordinator_test.dart
```

- [ ] **Step 6: Commit**

```bash
git add app/lib/providers/ app/lib/app.dart app/test/download_coordinator_test.dart
git commit -m "feat(app): 添加 DownloadCoordinator 与列表 Provider"
```

---

### Task 4: Router 与 AppShell（W3）

**Files:**
- Modify: `app/lib/router.dart`
- Create: `app/lib/shell/app_shell.dart`
- Create: `app/lib/features/library/library_screen.dart`（占位）
- Create: `app/lib/features/tasks/tasks_screen.dart`（占位）
- Create: `app/lib/features/add/add_screen.dart`（占位）
- Create: `app/lib/features/settings/settings_screen.dart`（占位）
- Test: `app/test/app_shell_test.dart`

**Interfaces:**
- Consumes: `go_router`、`AppShell`
- Produces: `appRouterProvider` 含 ShellRoute；`AppShell` 暴露 `navigationShell`；`kAppShellBreakpoint = 600.0`

- [ ] **Step 1: 写失败测试 W3 `app/test/app_shell_test.dart`**

使用完整 `GoRouter` + `StatefulShellRoute`（**不要** fake `StatefulNavigationShell`）：

```dart
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:go_router/go_router.dart';
import 'package:video_sniffing/shell/app_shell.dart';

GoRouter _testRouter() {
  return GoRouter(
    initialLocation: '/library',
    routes: [
      StatefulShellRoute.indexedStack(
        builder: (context, state, navigationShell) =>
            AppShell(navigationShell: navigationShell),
        branches: [
          StatefulShellBranch(
            routes: [
              GoRoute(
                path: '/library',
                builder: (_, __) => const Text('library'),
              ),
            ],
          ),
          StatefulShellBranch(
            routes: [
              GoRoute(path: '/tasks', builder: (_, __) => const Text('tasks')),
            ],
          ),
          StatefulShellBranch(
            routes: [
              GoRoute(path: '/add', builder: (_, __) => const Text('add')),
            ],
          ),
          StatefulShellBranch(
            routes: [
              GoRoute(
                path: '/settings',
                builder: (_, __) => const Text('settings'),
              ),
            ],
          ),
        ],
      ),
    ],
  );
}

void main() {
  testWidgets('W3 uses NavigationBar when width < 600', (tester) async {
    await tester.pumpWidget(
      MediaQuery(
        data: const MediaQueryData(size: Size(400, 800)),
        child: MaterialApp.router(routerConfig: _testRouter()),
      ),
    );
    await tester.pumpAndSettle();
    expect(find.byType(NavigationBar), findsOneWidget);
    expect(find.byType(NavigationRail), findsNothing);
  });

  testWidgets('W3 uses NavigationRail when width >= 600', (tester) async {
    await tester.pumpWidget(
      MediaQuery(
        data: const MediaQueryData(size: Size(800, 800)),
        child: MaterialApp.router(routerConfig: _testRouter()),
      ),
    );
    await tester.pumpAndSettle();
    expect(find.byType(NavigationRail), findsOneWidget);
    expect(find.byType(NavigationBar), findsNothing);
  });
}
```

- [ ] **Step 2: Run FAIL**

```bash
cd app && flutter test test/app_shell_test.dart
```

- [ ] **Step 3: 实现 `app/lib/shell/app_shell.dart`**

```dart
import 'package:flutter/material.dart';
import 'package:go_router/go_router.dart';

const kAppShellBreakpoint = 600.0;

class AppShell extends StatelessWidget {
  const AppShell({super.key, required this.navigationShell});

  final StatefulNavigationShell navigationShell;

  static const _destinations = [
    NavigationDestination(icon: Icon(Icons.video_library), label: '片库'),
    NavigationDestination(icon: Icon(Icons.download), label: '任务'),
    NavigationDestination(icon: Icon(Icons.add_link), label: '添加'),
    NavigationDestination(icon: Icon(Icons.settings), label: '设置'),
  ];

  void _onTap(int index) {
    navigationShell.goBranch(index, initialLocation: index == navigationShell.currentIndex);
  }

  @override
  Widget build(BuildContext context) {
    final useRail = MediaQuery.sizeOf(context).width >= kAppShellBreakpoint;
    final body = navigationShell;
    if (useRail) {
      return Scaffold(
        body: Row(
          children: [
            NavigationRail(
              selectedIndex: navigationShell.currentIndex,
              onDestinationSelected: _onTap,
              labelType: NavigationRailLabelType.all,
              destinations: _destinations
                  .map(
                    (d) => NavigationRailDestination(
                      icon: d.icon,
                      label: Text(d.label),
                    ),
                  )
                  .toList(),
            ),
            const VerticalDivider(width: 1),
            Expanded(child: body),
          ],
        ),
      );
    }
    return Scaffold(
      body: body,
      bottomNavigationBar: NavigationBar(
        selectedIndex: navigationShell.currentIndex,
        onDestinationSelected: _onTap,
        destinations: _destinations,
      ),
    );
  }
}
```

- [ ] **Step 4: 创建四个占位 Screen（各约 10 行）**

例 `library_screen.dart`:

```dart
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

class LibraryScreen extends ConsumerWidget {
  const LibraryScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    return const Scaffold(
      body: Center(child: Text('片库')),
    );
  }
}
```

`tasks_screen.dart`、`add_screen.dart`、`settings_screen.dart` 同理改标题。

- [ ] **Step 5: 完善 `app/lib/router.dart`**

```dart
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:video_sniffing/features/add/add_screen.dart';
import 'package:video_sniffing/features/library/library_detail_screen.dart';
import 'package:video_sniffing/features/library/library_screen.dart';
import 'package:video_sniffing/features/player/player_screen.dart';
import 'package:video_sniffing/features/settings/settings_screen.dart';
import 'package:video_sniffing/features/tasks/tasks_screen.dart';
import 'package:video_sniffing/shell/app_shell.dart';

final _rootNavigatorKey = GlobalKey<NavigatorState>();

final appRouterProvider = Provider<GoRouter>((ref) {
  return GoRouter(
    navigatorKey: _rootNavigatorKey,
    initialLocation: '/library',
    routes: [
      StatefulShellRoute.indexedStack(
        builder: (context, state, navigationShell) =>
            AppShell(navigationShell: navigationShell),
        branches: [
          StatefulShellBranch(
            routes: [
              GoRoute(
                path: '/library',
                builder: (_, __) => const LibraryScreen(),
                routes: [
                  GoRoute(
                    path: ':itemId',
                    builder: (_, state) => LibraryDetailScreen(
                      itemId: state.pathParameters['itemId']!,
                    ),
                  ),
                ],
              ),
            ],
          ),
          StatefulShellBranch(
            routes: [
              GoRoute(path: '/tasks', builder: (_, __) => const TasksScreen()),
            ],
          ),
          StatefulShellBranch(
            routes: [
              GoRoute(
                path: '/add',
                builder: (_, state) => AddScreen(
                  initialUrl: state.uri.queryParameters['url'],
                ),
              ),
            ],
          ),
          StatefulShellBranch(
            routes: [
              GoRoute(
                path: '/settings',
                builder: (_, __) => const SettingsScreen(),
              ),
            ],
          ),
        ],
      ),
      GoRoute(
        parentNavigatorKey: _rootNavigatorKey,
        path: '/play/:episodeId',
        builder: (_, state) => PlayerScreen(
          episodeId: state.pathParameters['episodeId']!,
        ),
      ),
    ],
  );
});
```

同时创建占位 `library_detail_screen.dart`、`player_screen.dart`（`PlayerScreen` 暂显示 `Text('player')`）。

- [ ] **Step 6: Run W3 PASS**

```bash
cd app && flutter test test/app_shell_test.dart
```

- [ ] **Step 7: Commit**

```bash
git add app/lib/router.dart app/lib/shell/ app/lib/features/ app/test/app_shell_test.dart
git commit -m "feat(app): 添加 go_router 与自适应 AppShell"
```

---

### Task 5: 片库 Feature

**Files:**
- Modify: `app/lib/features/library/library_screen.dart`
- Create: `app/lib/features/library/library_detail_screen.dart`
- Create: `app/lib/features/library/widgets/library_card.dart`
- Create: `app/lib/features/library/widgets/episode_tile.dart`

**Interfaces:**
- Consumes: `libraryProvider`、`engineRepositoryProvider.listEpisodes`、`LibraryItemKind`
- Produces: 可点击跳转 `/library/:itemId`；Series 分集行显示续播角标；点击分集 `context.push('/play/$episodeId')`

- [ ] **Step 1: 实现 `library_card.dart`**

列表项：`title`、Series 显示 `第 N 季`、`position_ms > 0` 时 `ResumeBadge`（小 Chip「续播」——在 detail 的 episode 行判断）。

- [ ] **Step 2: 实现 `library_screen.dart`**

```dart
// 核心逻辑
final items = ref.watch(libraryProvider);
if (items.isEmpty) return const Center(child: Text('片库为空，去「添加」粘贴 URL'));
return ListView.builder(
  itemCount: items.length,
  itemBuilder: (context, index) {
    final item = items[index];
    return LibraryCard(
      item: item,
      onTap: () => context.push('/library/${item.id}'),
    );
  },
);
```

- [ ] **Step 3: 实现 `library_detail_screen.dart`**

```dart
final repo = ref.read(engineRepositoryProvider);
final episodes = repo.listEpisodes(itemId);
// Single：若仅 1 集，大按钮「播放」→ push /play/{id}
// Series：ListView EpisodeTile，onTap → push /play/{id}
// EpisodeTile：subtitle 显示 file_path basename；有 duration 时 LinearProgressIndicator(value: position/duration)
```

- [ ] **Step 4: 手动验证**

```bash
cd app && flutter run -d macos
```

完成下载后片库应出现条目（可用集成测试 Task 10 自动化）。

- [ ] **Step 5: Commit**

```bash
git add app/lib/features/library/
git commit -m "feat(app): 实现片库列表与详情页"
```

---

### Task 6: 任务列表与 TaskTile（W2）

**Files:**
- Modify: `app/lib/features/tasks/tasks_screen.dart`
- Create: `app/lib/features/tasks/widgets/task_tile.dart`
- Create: `app/lib/features/tasks/widgets/parent_task_group.dart`
- Test: `app/test/task_tile_test.dart`

**Interfaces:**
- Produces: `TaskTile(task: DownloadTask, onPause, onResume, onCancel)`；`taskProgressFraction` 返回 `null` 当 `totalBytes == null`

- [ ] **Step 1: 写失败测试 W2**

```dart
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:video_sniffing/engine/models/download_task.dart';
import 'package:video_sniffing/engine/models/task_status.dart';
import 'package:video_sniffing/features/tasks/widgets/task_tile.dart';

void main() {
  testWidgets('W2 shows indeterminate progress when totalBytes is null', (
    tester,
  ) async {
    const task = DownloadTask(
      id: 't1',
      title: 'running',
      sourceUrl: 'https://example/x.mp4',
      status: TaskStatus.running,
      progressBytes: 100,
      totalBytes: null,
      createdAtMs: 1,
      updatedAtMs: 1,
    );
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: TaskTile(task: task, onPause: () {}, onResume: () {}, onCancel: () {}),
        ),
      ),
    );
    expect(find.byType(LinearProgressIndicator), findsOneWidget);
    final indicator = tester.widget<LinearProgressIndicator>(
      find.byType(LinearProgressIndicator),
    );
    expect(indicator.value, isNull);
  });
}
```

- [ ] **Step 2: Run FAIL → 实现 `task_tile.dart`**

```dart
double? taskProgressFraction(DownloadTask task) {
  final total = task.totalBytes;
  if (total == null || total == 0) return null;
  return task.progressBytes / total;
}

class TaskTile extends StatelessWidget {
  // ...
  @override
  Widget build(BuildContext context) {
    final fraction = taskProgressFraction(task);
    return ListTile(
      title: Text(task.title),
      subtitle: task.status == TaskStatus.running
          ? LinearProgressIndicator(value: fraction)
          : Text(task.status.jsonValue),
      trailing: _buildActions(context),
    );
  }
}
```

- [ ] **Step 3: 实现 `parent_task_group.dart` 与 `tasks_screen.dart`**

```dart
// tasks_screen.dart
final tasks = ref.watch(tasksProvider);
final roots = tasks.where((t) => t.parentId == null).toList();
// ParentTaskGroup: ExpansionTile 父任务 + children 子 TaskTile
// 操作调用 repo.pauseTask / resumeTask / cancelTask
```

- [ ] **Step 4: Run W2 PASS**

```bash
cd app && flutter test test/task_tile_test.dart
```

- [ ] **Step 5: Commit**

```bash
git add app/lib/features/tasks/ app/test/task_tile_test.dart
git commit -m "feat(app): 实现任务列表与 TaskTile 进度条"
```

---

### Task 7: 设置页（W4）

**Files:**
- Modify: `app/lib/features/settings/settings_screen.dart`
- Test: `app/test/settings_screen_test.dart`

**Interfaces:**
- Consumes: `settingsProvider`、`engineRepositoryProvider.saveSettings`、`presentEngineError`
- Produces: `SettingsScreen` 表单字段 key：`settings_media_dir` 等

- [ ] **Step 1: 写失败测试 W4**

```dart
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:video_sniffing/features/settings/settings_screen.dart';
import 'package:video_sniffing/providers/engine_repository.dart';
import '../fakes/fake_engine_repository.dart';

void main() {
  testWidgets('W4 shows error when media_dir contains slash', (tester) async {
    final fake = FakeEngineRepository();
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          engineRepositoryProvider.overrideWithValue(fake),
        ],
        child: const MaterialApp(home: SettingsScreen()),
      ),
    );
    await tester.enterText(find.byKey(const Key('settings_media_dir')), 'bad/dir');
    await tester.tap(find.byKey(const Key('settings_save')));
    await tester.pump();
    expect(
      find.textContaining('media_dir must be a single relative directory name'),
      findsOneWidget,
    );
  });
}
```

实现时：`saveSettings` 在 `media_dir` 含 `/` 时由引擎抛 `EngineException`；UI catch 后 `presentEngineError` 显示在 `settings_error` Text。

- [ ] **Step 2: 实现 `settings_screen.dart`**

五个字段 + 保存按钮；`copyWith` 更新本地 `_draft`；保存成功 `ref.invalidate(settingsProvider)` + SnackBar。

- [ ] **Step 3: Run W4 PASS**

```bash
cd app && flutter test test/settings_screen_test.dart
```

- [ ] **Step 4: Commit**

```bash
git add app/lib/features/settings/ app/test/settings_screen_test.dart
git commit -m "feat(app): 实现设置页与校验错误展示"
```

---

### Task 8: 添加流与 ResolveWizard（W1）

**Files:**
- Modify: `app/lib/features/add/add_screen.dart`
- Create: `app/lib/features/add/resolve_wizard.dart`
- Create: `app/lib/features/add/widgets/quality_picker.dart`
- Create: `app/lib/features/add/widgets/episode_multi_select.dart`
- Create: `app/lib/ui/loading_overlay.dart`
- Test: `app/test/resolve_wizard_test.dart`

**Interfaces:**
- Consumes: `ResolveOutcome*`、`downloadCoordinatorProvider.ensureDownloads`、`settingsProvider`
- Produces: `ResolveWizard(outcome, onEnqueueComplete)` 渲染四分支 UI

- [ ] **Step 1: 写失败测试 W1 `app/test/resolve_wizard_test.dart`（四条独立 test）**

```dart
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:video_sniffing/engine/models/resolve_types.dart';
import 'package:video_sniffing/features/add/resolve_wizard.dart';

Widget _wrap(ResolveOutcome outcome) {
  return MaterialApp(
    home: Scaffold(
      body: ResolveWizard(
        outcome: outcome,
        onEnqueue: (_) async {},
      ),
    ),
  );
}

void main() {
  testWidgets('W1 Single shows download button', (tester) async {
    await tester.pumpWidget(
      _wrap(
        ResolveOutcomeSingle(
          ResourceCandidate(id: '1', url: 'https://x/y.mp4', kind: MediaKind.mp4),
        ),
      ),
    );
    expect(find.text('下载'), findsOneWidget);
  });

  testWidgets('W1 Candidates shows quality picker', (tester) async {
    await tester.pumpWidget(
      _wrap(
        ResolveOutcomeCandidates([
          ResourceCandidate(id: '1', url: 'https://x/a.m3u8', kind: MediaKind.hls),
        ]),
      ),
    );
    expect(find.textContaining('清晰度'), findsOneWidget);
  });

  testWidgets('W1 EpisodeList shows multi select', (tester) async {
    await tester.pumpWidget(
      _wrap(
        ResolveOutcomeEpisodeList(
          EpisodeList(
            title: 'Series',
            episodes: [
              Episode(index: 1, title: 'E1', url: 'https://x/1', qualityOptions: []),
            ],
          ),
        ),
      ),
    );
    expect(find.textContaining('选择分集'), findsOneWidget);
  });

  testWidgets('W1 NeedsBrowser shows browser hint', (tester) async {
    await tester.pumpWidget(
      _wrap(const ResolveOutcomeNeedsBrowser(reason: 'auth_required')),
    );
    expect(find.textContaining('内置浏览器'), findsOneWidget);
  });
}
```

- [ ] **Step 2: 实现 `resolve_wizard.dart`**

| Outcome | UI |
|---------|-----|
| `Single` | 标题 `Key('resolve_title_field')` +「下载」 |
| `Candidates` | 选候选 → 若 `quality == null` 且 `kind == hls` 则 `resolveQualities(url)` → `QualityPicker` → `enqueueSingle` |
| `EpisodeList` | `EpisodeMultiSelect` 默认全选 → `enqueueEpisodes` |
| `NeedsBrowser` | 说明文案 +「返回」 |

入队后：

```dart
ref.read(downloadCoordinatorProvider).ensureDownloads();
if (context.mounted) {
  ScaffoldMessenger.of(context).showSnackBar(
    const SnackBar(content: Text('已加入下载队列')),
  );
  context.go('/tasks');
}
```

- [ ] **Step 3: 实现 `add_screen.dart`**

```dart
// TextField key: add_url_field
// FilledButton key: add_resolve_button
// onPressed: show LoadingOverlay → repo.resolveUrl(url) → Navigator push ResolveWizard 或同页切换
// 支持 initialUrl query 预填（Plan 6 预留）
```

- [ ] **Step 4: Run W1 PASS**

```bash
cd app && flutter test test/resolve_wizard_test.dart
```

- [ ] **Step 5: Commit**

```bash
git add app/lib/features/add/ app/lib/ui/loading_overlay.dart app/test/resolve_wizard_test.dart
git commit -m "feat(app): 实现粘贴 URL 解析向导与入队"
```

---

### Task 9: 播放器（media_kit）

**Files:**
- Modify: `app/lib/features/player/player_screen.dart`
- Create: `app/lib/features/player/player_controller.dart`

**Interfaces:**
- Consumes: `engineRepositoryProvider`、`LibraryEpisode.filePath`、`media_kit` `Player`/`VideoController`
- Produces: `PlayerScreen(episodeId)` 全屏播放；dispose 时 `setEpisodePosition`

- [ ] **Step 1: 实现 `player_controller.dart`**

```dart
import 'package:media_kit/media_kit.dart';
import 'package:media_kit_video/media_kit_video.dart';

class LocalPlayerController {
  LocalPlayerController(String filePath) : player = Player() {
    controller = VideoController(player);
    player.open(Media(filePath));
  }

  final Player player;
  late final VideoController controller;
  int lastPositionMs = 0;

  Future<void> seekTo(Duration position) => player.seek(position);

  void updateCachedPosition() {
    lastPositionMs = player.state.position.inMilliseconds;
  }

  Future<void> dispose() async {
    updateCachedPosition();
    await player.dispose();
  }
}
```

- [ ] **Step 2: 实现 `player_screen.dart`**

要点：
- `WidgetsBindingObserver`：`paused` 时 `_persistPosition()`
- 订阅 `player.stream.position`，debounce 5s 更新 `lastPositionMs`
- `_init`：`player.stream.duration` 首帧非零后 `seek(positionMs)`（**不要** `player.stream.first`）
- 返回：`await _persistPosition()` 再 `context.pop()`
- `dispose`：用 `lastPositionMs` 同步调用 `setEpisodePosition`（不 await 未完成 Future）

- [ ] **Step 3: 本地验证播放（MP4 + HLS）**

```bash
cd app && flutter run -d macos
```

1. 下载 fixture MP4 → 片库播放 → 退出再进确认续播  
2. 下载 HLS fixture（引擎 ffmpeg 合并输出）→ 片库播放确认可播

- [ ] **Step 4: Commit**

```bash
git add app/lib/features/player/
git commit -m "feat(app): 集成 media_kit 本地播放器与进度回写"
```

---

### Task 10: UI 集成测试 U1–U3 与 CI

**Files:**
- Create: `app/integration_test/ui_test.dart`
- Modify: `.github/workflows/ci.yml`
- Create: `app/lib/bootstrap.dart`（可选：导出测试用 `pumpApp`）

**Interfaces:**
- Consumes: 完整 `VideoSniffingApp`、widget Key：`add_url_field`、`add_resolve_button`、`tasks_list`
- Produces: U1–U3 通过；CI 跑 `ui_test.dart`

- [ ] **Step 1: 为关键 Widget 添加 Key（若 Task 5–8 未加）**

- `AddScreen`: `Key('add_url_field')`, `Key('add_resolve_button')`
- `TasksScreen`: `Key('tasks_list')`
- `LibraryScreen`: `Key('library_list')`

- [ ] **Step 2: 创建 `app/integration_test/ui_test.dart`**

```dart
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';
import 'package:path_provider/path_provider.dart';
import 'package:video_sniffing/app.dart';
import 'package:video_sniffing/engine/engine_host.dart';
import 'package:video_sniffing/engine/models/task_status.dart';

import 'smoke_test.dart' show pumpEngineEvents;

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();

  testWidgets('U1 paste url creates task entry', (tester) async {
    await tester.pumpWidget(const ProviderScope(child: VideoSniffingApp()));
    await tester.pumpAndSettle(const Duration(seconds: 15));

    await tester.tap(find.text('添加'));
    await tester.pumpAndSettle();
    await tester.enterText(
      find.byKey(const Key('add_url_field')),
      'https://cdn.example/clip.mp4',
    );
    await tester.tap(find.byKey(const Key('add_resolve_button')));
    await pumpEngineEvents(tester);
    await tester.pumpAndSettle(const Duration(seconds: 15));

    await tester.enterText(
      find.byKey(const Key('resolve_title_field')),
      'ui-smoke-clip',
    );
    await tester.tap(find.text('下载'));
    await tester.pumpAndSettle(const Duration(seconds: 5));

    expect(find.text('ui-smoke-clip'), findsWidgets);
  });

  testWidgets('U2 completed task appears in library', (tester) async {
    await tester.pumpWidget(const ProviderScope(child: VideoSniffingApp()));
    await tester.pumpAndSettle(const Duration(seconds: 15));

    // 复用 U1 入队流程（可抽 helper enqueueFixtureMp4）
    // ... 同上粘贴 URL、标题 ui-smoke-clip、点下载 ...

    final end = DateTime.now().add(const Duration(seconds: 60));
    while (DateTime.now().isBefore(end)) {
      await tester.tap(find.text('任务'));
      await pumpEngineEvents(tester);
      await tester.pump(const Duration(seconds: 1));
      if (find.textContaining('completed').evaluate().isNotEmpty) {
        break;
      }
    }

    await tester.tap(find.text('片库'));
    await tester.pumpAndSettle();
    expect(find.byKey(const Key('library_list')), findsOneWidget);
    expect(find.textContaining('ui-smoke-clip'), findsWidgets);
  });

  testWidgets('U3 resume position persists', (tester) async {
    final supportDir = await getApplicationSupportDirectory();
    // 先完成 U2 流程使片库有条目可播 …
    await tester.pumpWidget(const ProviderScope(child: VideoSniffingApp()));
    await tester.pumpAndSettle(const Duration(seconds: 15));
    // 进入片库 → 点播放 → pump 5s → 返回
    await tester.tap(find.text('片库'));
    await tester.pumpAndSettle();
    await tester.tap(find.textContaining('ui-smoke-clip'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('播放'));
    await tester.pump(const Duration(seconds: 5));
    await tester.tap(find.byIcon(Icons.arrow_back));
    await tester.pumpAndSettle();

    final host = await EngineHost.open(supportDir.path);
    try {
      final items = host.listLibrary();
      expect(items, isNotEmpty);
      final episodes = host.listEpisodes(items.first.id);
      expect(episodes.first.positionMs, greaterThan(0));
    } finally {
      host.dispose();
    }
  });
}
```

> U2/U3 实施时将入队 helper 抽为 `enqueueFixtureMp4(tester)` 避免重复。`VideoSniffingApp` 引擎门闩需 `pumpAndSettle` 足够长等待 `engineHostProvider.data`。

- [ ] **Step 3: 更新 `.github/workflows/ci.yml`**

在 `Flutter test and integration smoke` 步骤追加：

```yaml
          flutter test integration_test/ui_test.dart -d macos
```

- [ ] **Step 4: Run 本地**

```bash
cd app
flutter test
flutter test integration_test/smoke_test.dart -d macos
flutter test integration_test/ui_test.dart -d macos
```

- [ ] **Step 5: Commit**

```bash
git add app/integration_test/ui_test.dart .github/workflows/ci.yml
git commit -m "test(app): 添加 U1-U3 UI 集成测试并接入 CI"
```

---

### Task 11: README 与最终验证

**Files:**
- Modify: `README.md`

- [ ] **Step 1: README 追加 Plan 5 小节**

在「## Flutter + FFI」之后插入：

```markdown
## 应用 UI（Plan 5）

主流程：启动应用 →「添加」粘贴 URL → 解析并入队 →「任务」查看进度 →「片库」播放已缓存内容。

```bash
cd app
flutter pub get
flutter run -d macos   # 或 android / ios / windows
flutter test
flutter test integration_test/ui_test.dart -d macos
```

规格见 `docs/superpowers/specs/2026-08-11-app-ui-player-design.md`。
```

- [ ] **Step 2: 完整验证清单**

```bash
cd "$(git rev-parse --show-toplevel)"
cargo fmt --manifest-path engine/Cargo.toml --all -- --check
cargo test --manifest-path engine/Cargo.toml --workspace
cargo clippy --manifest-path engine/Cargo.toml --all-targets --all-features -- -D warnings
cd app && flutter pub get && flutter test
cd app && flutter test integration_test/smoke_test.dart -d macos
cd app && flutter test integration_test/ui_test.dart -d macos
cd app && flutter build macos --debug
cd app && flutter build windows --debug
```

（iOS/Android 构建在对应环境交付前验证。）

- [ ] **Step 3: 提交 README**

```bash
git add README.md
git commit -m "docs: 补充 Plan 5 应用 UI 使用说明"
```

---

## Self-Review

| 规格要求 | 对应任务 |
|----------|----------|
| Riverpod + go_router + media_kit | Task 1, 4, 9 |
| 四端 UI 路由 | Task 4–8 |
| 粘贴 URL 四分支解析 | Task 8 |
| 清晰度 / 分集批量 | Task 8 |
| 片库续播标记 | Task 5 |
| 任务父子折叠 + 进度 | Task 6 |
| 设置全字段 | Task 7 |
| 播放器 + position 回写 | Task 9 |
| 引擎启动门闩 + eager coordinator | Task 1 Step 2、Task 3 Step 4b |
| Candidates `resolveQualities` | Task 8 |
| HLS 播放验收 | Task 9 Step 3 |
| error_presenter | Task 2 |
| W1–W4 | Task 4, 6, 7, 8 |
| U1–U3 + F1–F5 保留 | Task 10 |
| README | Task 11 |
| 不改 engine/FFI | 全计划无 engine 修改 |

无 TBD / 占位符；各 Task 含路径、接口、测试命令与提交信息。

## 仓库设置（实施后不纳入 commit）

U4/U5 在合并前本地执行；CI 仅强制 U1–U3 + F1–F5。
