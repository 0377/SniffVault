# Plan 6b 系统分享与深链入口 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 在 Android / iOS 通过系统分享或 `sniffvault://add?url=` 深链打开添加页并预填 `http(s)` URL，复用现有解析向导，不改 Engine/FFI。

**Architecture:** 原生层只生成 `sniffvault://add?url=<encoded raw>`（Android `Uri.Builder.appendQueryParameter`；iOS `URLComponents`）。Flutter 用 `app_links` + `DeepLinkHost`（去重、`pendingIngressUriProvider` 排队）解析后 `GoRouter.go('/add?url=')`，**不**手动 `goBranch`。`platforms/share_ingress/` 仅托管 iOS Share Extension 源文件，无 Dart API。

**Tech Stack:** Flutter + Riverpod + go_router、`app_links`、`platforms/share_ingress`（空壳）、Kotlin `MainActivity`

**规格:** `docs/superpowers/specs/2026-09-08-system-share-design.md`

**修订:** 2026-09-08 plan review（去重、排队、SnackBar 分类、Android/iOS 编码、CI U8、移除 goBranch）

## Global Constraints

- 分享 **只预填添加页**，**禁止**自动 `resolveUrl`；**禁止** Universal Links、桌面协议、Engine/FFI 变更
- 深链：**`sniffvault://add?url=<encoded>`**；导航仅用 **`router.go('/add?url=...')`**，**禁止** `findAncestorStateOfType<StatefulNavigationShellState>` / 手动 `goBranch`
- SnackBar：`noHttpUrl`/`missingPayload` →「未能识别有效链接」；`invalidScheme` →「仅支持 http/https 链接」；`payloadTooLong` →「链接过长」
- 冷启动：**仅** `main()` 中 `getInitialLink()` → `pendingIngressUriProvider`；`DeepLinkHost` **禁止**再收 `initialUri` prop；**必须** `_lastHandledUri` 去重
- 引擎 loading：`pendingIngressUriProvider` 排队，进入 `data` 后消费
- Android query：**`Uri.Builder.appendQueryParameter("url", text)`**；**禁止** `Uri.encode(text)` 手拼
- iOS Extension：**`URLComponents` + `URLQueryItem`**；只传 raw，不在 Extension 内 `extractHttpUrl`
- `ingressUriStreamProvider` 类型 **`Provider<Stream<Uri>>`**（非 `StreamProvider`）
- `share_ingress`：**无 Dart import 要求**；仅为 iOS Extension 目录与插件注册
- `docs/` 在 `.gitignore`；`git add -f` 文档
- 验证：`cd app && flutter test`；`flutter test integration_test/deep_link_test.dart -d macos`
- 提交信息中文

---

## File Map

| 路径 | 职责 |
|------|------|
| `app/lib/deep_link/share_url_extractor.dart` | `extractHttpUrl`、`isIngressPayloadTooLong` |
| `app/lib/deep_link/ingress_uri.dart` | `parseSniffVaultIngress`、`IngressFailure`（含 `invalidScheme`） |
| `app/lib/deep_link/deep_link_providers.dart` | `appLinksProvider`、`ingressUriStreamProvider`、`pendingIngressUriProvider` |
| `app/lib/deep_link/deep_link_host.dart` | 去重、排队消费、导航、SnackBar |
| `app/lib/shell/app_shell.dart` | `kAddShellBranchIndex = 3`（文档常量，导航不调用） |
| `app/lib/features/add/add_screen.dart` | `didUpdateWidget` |
| `app/lib/main.dart` | `getInitialLink` → `pendingIngressUriProvider` |
| `app/lib/app.dart` | `builder: DeepLinkHost` |
| `app/android/.../MainActivity.kt` | SEND → sniffvault URI |
| `platforms/share_ingress/ios/ShareExtension/` | Extension 源 |
| `app/integration_test/deep_link_test.dart` | U8 |
| `.github/workflows/ci.yml` | `deeplink` integration suite |

---

### Task 1: URL 提取器 `extractHttpUrl`

**Files:**
- Create: `app/lib/deep_link/share_url_extractor.dart`
- Test: `app/test/share_url_extractor_test.dart`

**Interfaces:**
- Consumes: `browseUrlError` from `app/lib/features/browse/browse_url.dart`
- Produces: `extractHttpUrl(String raw) → String?`；`isIngressPayloadTooLong(String) → bool`；`kMaxIngressPayloadLength = 8192`

- [ ] **Step 1: 写失败测试（W12 等）**

```dart
import 'package:flutter_test/flutter_test.dart';
import 'package:video_sniffing/deep_link/share_url_extractor.dart';

void main() {
  test('W12 extracts first http url from surrounding text', () {
    expect(
      extractHttpUrl('看这里 https://a.com/x 和 https://b.com'),
      'https://a.com/x',
    );
  });

  test('accepts whole string when already valid url', () {
    expect(
      extractHttpUrl('https://example.com/watch?v=1'),
      'https://example.com/watch?v=1',
    );
  });

  test('rejects javascript scheme', () {
    expect(extractHttpUrl('javascript:alert(1)'), isNull);
  });

  test('rejects empty and plain text without url', () {
    expect(extractHttpUrl(''), isNull);
    expect(extractHttpUrl('只是文字'), isNull);
  });

  test('isIngressPayloadTooLong at boundary', () {
    expect(isIngressPayloadTooLong('x' * 8192), isFalse);
    expect(isIngressPayloadTooLong('x' * 8193), isTrue);
  });
}
```

- [ ] **Step 2: 运行确认失败** — `cd app && flutter test test/share_url_extractor_test.dart`

- [ ] **Step 3: 实现** `share_url_extractor.dart`（与 review 前计划相同：`browseUrlError` + 正则 `_httpUrlInText`）

- [ ] **Step 4: 运行确认通过**

- [ ] **Step 5: Commit** — `feat(app): 分享文本 URL 提取与长度校验`

---

### Task 2: 深链 URI 解析（含 `invalidScheme`）

**Files:**
- Create: `app/lib/deep_link/ingress_uri.dart`
- Test: `app/test/ingress_uri_test.dart`

**Interfaces:**
- Produces:
  ```dart
  enum IngressFailure { missingPayload, payloadTooLong, noHttpUrl, invalidScheme }

  sealed class IngressNavigateTarget {}
  class IngressNavigateSuccess extends IngressNavigateTarget { final String url; }
  class IngressNavigateFailure extends IngressNavigateTarget { final IngressFailure reason; }

  IngressNavigateTarget? parseSniffVaultIngress(Uri uri);
  String snackBarMessageFor(IngressFailure failure);
  ```

- [ ] **Step 1: 写失败测试**

```dart
import 'package:flutter_test/flutter_test.dart';
import 'package:video_sniffing/deep_link/ingress_uri.dart';

void main() {
  test('parses sniffvault add url query', () {
    final result = parseSniffVaultIngress(
      Uri.parse('sniffvault://add?url=https%3A%2F%2Fexample.com'),
    );
    expect(result, isA<IngressNavigateSuccess>());
    expect((result! as IngressNavigateSuccess).url, 'https://example.com');
  });

  test('W11 javascript payload is invalidScheme', () {
    final result = parseSniffVaultIngress(
      Uri.parse(
        'sniffvault://add?url=${Uri.encodeComponent('javascript:alert(1)')}',
      ),
    );
    expect(result, isA<IngressNavigateFailure>());
    expect(
      (result! as IngressNavigateFailure).reason,
      IngressFailure.invalidScheme,
    );
    expect(
      snackBarMessageFor(IngressFailure.invalidScheme),
      '仅支持 http/https 链接',
    );
  });

  test('plain text without url is noHttpUrl', () {
    final result = parseSniffVaultIngress(
      Uri.parse('sniffvault://add?url=${Uri.encodeComponent('只是文字')}'),
    );
    expect((result! as IngressNavigateFailure).reason, IngressFailure.noHttpUrl);
  });

  test('ignores non sniffvault scheme', () {
    expect(parseSniffVaultIngress(Uri.parse('https://example.com')), isNull);
  });
}
```

- [ ] **Step 2: 运行确认失败**

- [ ] **Step 3: 实现**

```dart
import 'package:video_sniffing/deep_link/share_url_extractor.dart';

enum IngressFailure { missingPayload, payloadTooLong, noHttpUrl, invalidScheme }

sealed class IngressNavigateTarget {}

class IngressNavigateSuccess extends IngressNavigateTarget {
  IngressNavigateSuccess(this.url);
  final String url;
}

class IngressNavigateFailure extends IngressNavigateTarget {
  IngressNavigateFailure(this.reason);
  final IngressFailure reason;
}

final _schemePrefix = RegExp(r'[a-z][a-z0-9+.-]*://', caseSensitive: false);

bool _hasDisallowedScheme(String raw) {
  final trimmed = raw.trim();
  final direct = Uri.tryParse(trimmed);
  if (direct != null && direct.hasScheme) {
    final scheme = direct.scheme.toLowerCase();
    if (scheme != 'http' && scheme != 'https') {
      return true;
    }
  }
  for (final match in _schemePrefix.allMatches(trimmed)) {
    final prefix = match.group(0)!.toLowerCase();
    if (!prefix.startsWith('http://') && !prefix.startsWith('https://')) {
      return true;
    }
  }
  return false;
}

String snackBarMessageFor(IngressFailure failure) {
  return switch (failure) {
    IngressFailure.invalidScheme => '仅支持 http/https 链接',
    IngressFailure.payloadTooLong => '链接过长',
    IngressFailure.missingPayload ||
    IngressFailure.noHttpUrl =>
      '未能识别有效链接',
  };
}

IngressNavigateTarget? parseSniffVaultIngress(Uri uri) {
  if (uri.scheme != 'sniffvault' || uri.host != 'add') {
    return null;
  }
  final payload = uri.queryParameters['url'];
  if (payload == null || payload.trim().isEmpty) {
    return IngressNavigateFailure(IngressFailure.missingPayload);
  }
  if (isIngressPayloadTooLong(payload)) {
    return IngressNavigateFailure(IngressFailure.payloadTooLong);
  }
  if (_hasDisallowedScheme(payload)) {
    return IngressNavigateFailure(IngressFailure.invalidScheme);
  }
  final target = extractHttpUrl(payload);
  if (target == null) {
    return IngressNavigateFailure(IngressFailure.noHttpUrl);
  }
  return IngressNavigateSuccess(target);
}
```

- [ ] **Step 4: 运行确认通过** — `flutter test test/ingress_uri_test.dart`

- [ ] **Step 5: Commit** — `feat(app): 解析 sniffvault 深链并区分失败类型`

---

### Task 3: `DeepLinkHost`（W10、去重、排队）

**Files:**
- Create: `app/lib/deep_link/deep_link_providers.dart`
- Create: `app/lib/deep_link/deep_link_host.dart`
- Modify: `app/lib/shell/app_shell.dart`
- Modify: `app/lib/app.dart`
- Modify: `app/lib/main.dart`
- Modify: `app/pubspec.yaml`（`app_links: ^6.3.2`）
- Test: `app/test/deep_link_host_test.dart`

**Interfaces:**
- Produces:
  ```dart
  final appLinksProvider = Provider<AppLinks>((ref) => AppLinks());
  final ingressUriStreamProvider = Provider<Stream<Uri>>((ref) {
    return ref.watch(appLinksProvider).uriLinkStream;
  });
  final pendingIngressUriProvider = StateProvider<Uri?>((ref) => null);

  class DeepLinkHost extends ConsumerStatefulWidget {
    const DeepLinkHost({super.key, required this.child});
    final Widget child;
  }
  ```

- [ ] **Step 1:** 在 `app_shell.dart` 增加 `const kAddShellBranchIndex = 3;`（仅文档/测试引用，**不在 DeepLinkHost 调用 goBranch**）

- [ ] **Step 2: 写失败测试 W10**

W10 测试要点：
- `ProviderScope` overrides：`ingressUriStreamProvider.overrideWithValue(Stream.empty())`、`pendingIngressUriProvider.overrideWith(...)` **不要**传 `initialUri` 给 `DeepLinkHost`
- 通过 `pendingIngressUriProvider` 注入 `sniffvault://add?url=https%3A%2F%2Fexample.com`
- 断言 `router.state.uri.path == '/add'` 且 `AddScreen` 预填 `https://example.com`

`DeepLinkHost` 构造：**仅** `child` 参数。

- [ ] **Step 3: 实现 `deep_link_providers.dart`**

```dart
import 'package:app_links/app_links.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

final appLinksProvider = Provider<AppLinks>((ref) => AppLinks());

final ingressUriStreamProvider = Provider<Stream<Uri>>((ref) {
  return ref.watch(appLinksProvider).uriLinkStream;
});

final pendingIngressUriProvider = StateProvider<Uri?>((ref) => null);
```

- [ ] **Step 4: 实现 `deep_link_host.dart`**

```dart
import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:video_sniffing/deep_link/deep_link_providers.dart';
import 'package:video_sniffing/deep_link/ingress_uri.dart';

class DeepLinkHost extends ConsumerStatefulWidget {
  const DeepLinkHost({super.key, required this.child});
  final Widget child;

  @override
  ConsumerState<DeepLinkHost> createState() => _DeepLinkHostState();
}

class _DeepLinkHostState extends ConsumerState<DeepLinkHost> {
  String? _lastHandledUri;
  StreamSubscription<Uri>? _subscription;

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addPostFrameCallback((_) {
      _flushPending();
      _subscribeStream();
    });
  }

  @override
  void dispose() {
    _subscription?.cancel();
    super.dispose();
  }

  void _flushPending() {
    final pending = ref.read(pendingIngressUriProvider);
    if (pending != null) {
      ref.read(pendingIngressUriProvider.notifier).state = null;
      _handleUri(pending);
    }
  }

  void _subscribeStream() {
    final stream = ref.read(ingressUriStreamProvider);
    _subscription = stream.listen(_handleUri);
  }

  void _handleUri(Uri uri) {
    final key = uri.toString();
    if (_lastHandledUri == key) {
      return;
    }
    final parsed = parseSniffVaultIngress(uri);
    if (parsed == null) {
      return;
    }
    _lastHandledUri = key;
    final router = GoRouter.of(context);
    final messenger = ScaffoldMessenger.maybeOf(context);

    if (parsed is IngressNavigateSuccess) {
      final encoded = Uri.encodeQueryComponent(parsed.url);
      router.go('/add?url=$encoded');
      return;
    }

    final failure = (parsed as IngressNavigateFailure).reason;
    router.go('/add');
    messenger?.showSnackBar(
      SnackBar(content: Text(snackBarMessageFor(failure))),
    );
  }

  @override
  Widget build(BuildContext context) => widget.child;
}
```

- [ ] **Step 5: 修改 `deep_link_providers.dart` 与 `main.dart`**

`deep_link_providers.dart` 增加 bootstrap 辅助：

```dart
Uri? _bootstrapIngressUri;

/// 在 runApp 之前调用，写入冷启动 initial link。
void setBootstrapIngressUri(Uri? uri) => _bootstrapIngressUri = uri;

final pendingIngressUriProvider = StateProvider<Uri?>((ref) => _bootstrapIngressUri);
```

`main.dart`：

```dart
import 'package:app_links/app_links.dart';
import 'package:video_sniffing/deep_link/deep_link_providers.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  MediaKit.ensureInitialized();
  openNativeLibrary();
  setBootstrapIngressUri(await AppLinks().getInitialLink());
  runApp(
    const ProviderScope(
      child: IngressUriListener(
        child: VideoSniffingApp(),
      ),
    ),
  );
}
```

- [ ] **Step 6: 实现 `IngressUriListener`**（与 `deep_link_host.dart` 同文件或 `ingress_uri_listener.dart`）

引擎 loading 期间亦将热启动 URI 写入 `pendingIngressUriProvider`：

```dart
class IngressUriListener extends ConsumerStatefulWidget {
  const IngressUriListener({super.key, required this.child});
  final Widget child;

  @override
  ConsumerState<IngressUriListener> createState() => _IngressUriListenerState();
}

class _IngressUriListenerState extends ConsumerState<IngressUriListener> {
  StreamSubscription<Uri>? _subscription;

  @override
  void initState() {
    super.initState();
    final stream = ref.read(ingressUriStreamProvider);
    _subscription = stream.listen((uri) {
      ref.read(pendingIngressUriProvider.notifier).state = uri;
    });
  }

  @override
  void dispose() {
    _subscription?.cancel();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => widget.child;
}
```

- [ ] **Step 7: 修改 `app.dart`**

```dart
builder: (context, child) => DeepLinkHost(
  child: child ?? const SizedBox.shrink(),
),
```

仅挂在 `engineHostProvider.data` 分支。`DeepLinkHost.initState` 的 `addPostFrameCallback` 调用 `_flushPending()` 消费排队 URI。

- [ ] **Step 8: 写 W10 测试**

`DeepLinkHost` **无** `initialUri` 参数。测试在 pump 前：

```dart
setBootstrapIngressUri(
  Uri.parse('sniffvault://add?url=https%3A%2F%2Fexample.com'),
);
```

并 override `ingressUriStreamProvider.overrideWithValue(const Stream.empty())`。

- [ ] **Step 9: 运行** `flutter test test/deep_link_host_test.dart`

- [ ] **Step 10: Commit** — `feat(app): DeepLinkHost 深链导航与排队去重`

---

### Task 4: `AddScreen` 热更新（W13）

**Files:**
- Modify: `app/lib/features/add/add_screen.dart`
- Test: `app/test/add_screen_deep_link_test.dart`

- [ ] **Step 1: 写失败测试**（**必须** `ProviderScope` + `isTelevisionProvider` override）

```dart
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:video_sniffing/features/add/add_screen.dart';
import 'package:video_sniffing/providers/device_profile.dart';

void main() {
  testWidgets('W13 updates url field when initialUrl changes', (tester) async {
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          isTelevisionProvider.overrideWith((ref) async => false),
        ],
        child: const MaterialApp(
          home: AddScreen(initialUrl: 'https://first.example'),
        ),
      ),
    );
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          isTelevisionProvider.overrideWith((ref) async => false),
        ],
        child: const MaterialApp(
          home: AddScreen(initialUrl: 'https://second.example'),
        ),
      ),
    );
    final field = tester.widget<TextField>(find.byKey(const Key('add_url_field')));
    expect(field.controller?.text, 'https://second.example');
  });
}
```

- [ ] **Step 2–5:** 实现 `didUpdateWidget`（与 review 前相同）→ 测试 → Commit `fix(app): 深链热启动时更新添加页 URL 预填`

---

### Task 5: `share_ingress` 脚手架（紧挨 Task 7 亦可）

**Files:** `platforms/share_ingress/`、`app/pubspec.yaml`

- [ ] **Step 1:** `flutter create --template=plugin --platforms=android,ios share_ingress` 于 `platforms/`
- [ ] **Step 2:** `lib/share_ingress.dart` 仅 `library;` 注释说明 **无 Dart API**
- [ ] **Step 3:** `app/pubspec.yaml` path 依赖；`flutter pub get`
- [ ] **Step 4: Commit** — `feat(platforms): share_ingress 插件壳（iOS Extension 托管）`

---

### Task 6: Android 分享与深链

**Files:** `AndroidManifest.xml`、`MainActivity.kt`

- [ ] **Step 1:** Manifest 增加 SEND + VIEW intent-filter（与规格相同）

- [ ] **Step 2: MainActivity**

```kotlin
private fun toIngressUri(intent: Intent?): Uri? {
    if (intent == null) return null
    when (intent.action) {
        Intent.ACTION_SEND -> {
            if (intent.type != "text/plain") return null
            val text = intent.getStringExtra(Intent.EXTRA_TEXT)?.trim()
                ?: return null
            if (text.isEmpty()) return null
            return Uri.Builder()
                .scheme("sniffvault")
                .authority("add")
                .appendQueryParameter("url", text)
                .build()
        }
        Intent.ACTION_VIEW -> {
            val data = intent.data
            if (data?.scheme == "sniffvault" && data.host == "add") {
                return data
            }
        }
    }
    return null
}
```

- [ ] **Step 3:** `flutter build apk --debug`
- [ ] **Step 4: Commit** — `feat(android): 分享文本转 sniffvault 深链`

---

### Task 7: iOS URL Scheme + Share Extension

**Files:** `Runner/Info.plist`、`platforms/share_ingress/ios/ShareExtension/`、`project.pbxproj`

- [ ] **Step 1:** `CFBundleURLTypes` → `sniffvault`

- [ ] **Step 2: ShareViewController** — `openMainApp` 使用 `URLComponents`：

```swift
private func openMainApp(with raw: String) {
    let trimmed = raw.trimmingCharacters(in: .whitespacesAndNewlines)
    guard !trimmed.isEmpty else {
        showFailure()
        return
    }
    var components = URLComponents()
    components.scheme = "sniffvault"
    components.host = "add"
    components.queryItems = [URLQueryItem(name: "url", value: trimmed)]
    guard let url = components.url else {
        showFailure()
        return
    }
    extensionContext?.open(url) { _ in
        self.extensionContext?.completeRequest(returningItems: nil)
    }
}
```

- [ ] **Step 3:** Xcode 嵌入 ShareExtension target（Bundle Id `com.videosniffing.videoSniffing.ShareExtension`）
- [ ] **Step 4:** `flutter build ios --no-codesign --debug`
- [ ] **Step 5: Commit** — `feat(ios): Share Extension 与 sniffvault URL Scheme`

---

### Task 8: 集成测试 U8 + W11 widget

**Files:**
- Create: `app/integration_test/deep_link_test.dart`
- Create: `app/integration_test/support/deep_link_flow.dart`
- Test: `app/test/deep_link_host_test.dart` 追加 W11 SnackBar 用例（可选与 W10 同文件）

- [ ] **Step 1:** `deep_link_flow.dart` 通过 `setBootstrapIngressUri` + `VideoSniffingApp` 注入深链

- [ ] **Step 2:** U8 断言 `add_url_field` 非空、`example.com` 可见

- [ ] **Step 3:** `flutter test integration_test/deep_link_test.dart -d macos`

- [ ] **Step 4:** `flutter test` 全绿

- [ ] **Step 5: Commit** — `test(app): U8 深链预填集成测试`

---

### Task 9: 文档与 CI

**Files:** `README.md`、`platforms/README.md`、`.github/workflows/ci.yml`、规格状态

- [x] **Step 1: README** Plan 6b 小节 + U8 命令

- [x] **Step 2: platforms/README.md**

- [x] **Step 3: CI** — `.github/workflows/ci.yml`：

```yaml
      matrix:
        suite: [engine, ui, deeplink]
```

```yaml
          if [ "${{ matrix.suite }}" = "engine" ]; then
            flutter test integration_test/engine_smoke_test.dart -d macos
          elif [ "${{ matrix.suite }}" = "deeplink" ]; then
            flutter test integration_test/deep_link_test.dart -d macos
          else
            flutter test integration_test/ui_test.dart -d macos \
              --dart-define=INTEGRATION_SKIP_PLAYER=true
          fi
```

- [x] **Step 4: Commit** — `docs: Plan 6b 说明与 CI deeplink 门禁`

```bash
git add -f README.md platforms/README.md docs/superpowers/specs/2026-09-08-system-share-design.md docs/superpowers/plans/2026-09-08-system-share.md .github/workflows/ci.yml
```

---

## Spec Coverage（自检）

| 规格 | Task |
|------|------|
| Android appendQueryParameter | 6 |
| iOS URLComponents | 7 |
| invalidScheme SnackBar | 2, 3 |
| 去重 + pending 排队 | 3（IngressUriListener） |
| 无 goBranch | 3 |
| Provider\<Stream\<Uri\>\> | 3 |
| U8 CI | 9 |
| share_ingress 无 Dart API | 5 |

## Placeholder Scan

- 无 TBD；`IngressUriListener` + `setBootstrapIngressUri` 为 Task 3 必交付
