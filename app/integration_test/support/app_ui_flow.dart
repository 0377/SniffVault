import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:media_kit/media_kit.dart';
import 'package:path_provider/path_provider.dart';
import 'package:video_sniffing/app.dart';
import 'package:video_sniffing/engine/engine_host.dart';
import 'package:video_sniffing/engine/models/task_status.dart';
import 'package:video_sniffing/engine/native_bindings.dart';
import 'package:video_sniffing/providers/engine_host_provider.dart';
import 'package:video_sniffing/providers/engine_repository.dart';
import 'package:video_sniffing/providers/library_provider.dart';
import 'package:video_sniffing/router.dart';

import 'playable_mp4.dart';
import 'test_pump.dart';

String? _activeDataDir;
HttpServer? _fixtureServer;

const _skipPlayer = bool.fromEnvironment(
  'INTEGRATION_SKIP_PLAYER',
  defaultValue: false,
);

Future<String> _createIsolatedDataDir(String label) async {
  final temp = await getTemporaryDirectory();
  final dir = Directory(
    '${temp.path}/ui_test_${label}_${DateTime.now().millisecondsSinceEpoch}',
  );
  await dir.create(recursive: true);
  return dir.path;
}

EngineRepository _testRepo(WidgetTester tester) {
  return ProviderScope.containerOf(
    tester.element(find.byType(MaterialApp)),
  ).read(engineRepositoryProvider);
}

void _invalidateLibrary(WidgetTester tester) {
  ProviderScope.containerOf(
    tester.element(find.byType(MaterialApp)),
  ).invalidate(libraryProvider);
}

Future<void> _startFixtureServer() async {
  if (_fixtureServer != null) {
    return;
  }
  final bytes = playableMp4Bytes();
  _fixtureServer = await HttpServer.bind(InternetAddress.loopbackIPv4, 0);
  final fixtureMp4Url = 'http://127.0.0.1:${_fixtureServer!.port}/clip.mp4';

  _fixtureServer!.listen((request) async {
    if (request.uri.path != '/clip.mp4') {
      request.response.statusCode = HttpStatus.notFound;
      await request.response.close();
      return;
    }

    request.response.headers.set('accept-ranges', 'bytes');
    request.response.headers.contentType = ContentType('video', 'mp4');
    request.response.headers.contentLength = bytes.length;

    if (request.method == 'HEAD') {
      request.response.statusCode = HttpStatus.ok;
      await request.response.close();
      return;
    }

    if (request.method == 'GET') {
      final range = request.headers.value(HttpHeaders.rangeHeader);
      if (range != null && range.startsWith('bytes=')) {
        final spec = range.substring(6);
        final parts = spec.split('-');
        final start = int.parse(parts[0]);
        final end = parts.length > 1 && parts[1].isNotEmpty
            ? int.parse(parts[1])
            : bytes.length - 1;
        final chunk = bytes.sublist(start, end + 1);
        request.response.statusCode = HttpStatus.partialContent;
        request.response.headers.set(
          HttpHeaders.contentRangeHeader,
          'bytes $start-$end/${bytes.length}',
        );
        request.response.contentLength = chunk.length;
        request.response.add(chunk);
        await request.response.close();
        return;
      }

      request.response.statusCode = HttpStatus.ok;
      request.response.add(bytes);
      await request.response.close();
      return;
    }

    request.response.statusCode = HttpStatus.methodNotAllowed;
    await request.response.close();
  });

  _fixtureMp4Url = fixtureMp4Url;
}

late String _fixtureMp4Url;

Future<void> _stopFixtureServer() async {
  await _fixtureServer?.close(force: true);
  _fixtureServer = null;
}

Future<void> _pumpUntilCondition(
  WidgetTester tester,
  bool Function() condition, {
  Duration timeout = const Duration(seconds: 15),
}) async {
  final end = DateTime.now().add(timeout);
  while (DateTime.now().isBefore(end)) {
    if (condition()) {
      return;
    }
    await Future<void>.delayed(const Duration(milliseconds: 100));
    await tester.pump(const Duration(milliseconds: 1));
  }
  fail('Timed out after ${timeout.inSeconds}s waiting for condition');
}

Future<void> _waitForLibraryItemInEngine(
  WidgetTester tester,
  String title,
) async {
  final end = DateTime.now().add(const Duration(seconds: 60));
  while (DateTime.now().isBefore(end)) {
    if (_testRepo(tester).listLibrary().any((item) => item.title == title)) {
      return;
    }
    await pumpEngineEvents(tester);
    await Future<void>.delayed(const Duration(milliseconds: 200));
  }
  fail('Timed out waiting for library item: $title');
}

Future<void> _openLibraryDetailForTitle(
  WidgetTester tester,
  String title,
) async {
  final repo = _testRepo(tester);
  final items = repo
      .listLibrary()
      .where((item) => item.title == title)
      .toList();
  expect(items, hasLength(1));
  final itemId = items.first.id;

  final container = ProviderScope.containerOf(tester.element(find.text('片库')));
  container.read(appRouterProvider).push('/library/$itemId');
  await _pumpUntilCondition(
    tester,
    () => find.text('播放').evaluate().isNotEmpty,
  );
}

Future<void> _launchApp(
  WidgetTester tester, {
  required String testLabel,
}) async {
  _activeDataDir = await _createIsolatedDataDir(testLabel);
  openNativeLibrary();

  tester.view.physicalSize = const Size(400, 800);
  tester.view.devicePixelRatio = 1.0;
  addTearDown(tester.view.resetPhysicalSize);
  addTearDown(tester.view.resetDevicePixelRatio);

  await tester.pumpWidget(
    ProviderScope(
      overrides: [
        engineHostProvider.overrideWith((ref) async {
          final host = await EngineHost.open(_activeDataDir!);
          ref.onDispose(host.dispose);
          return host;
        }),
      ],
      child: const VideoSniffingApp(),
    ),
  );
  await _pumpUntilCondition(
    tester,
    () =>
        find.text('正在初始化引擎…').evaluate().isEmpty &&
        find.text('片库').evaluate().isNotEmpty,
  );
}

Future<void> _tapFilledButton(WidgetTester tester, String label) async {
  final finder = find.ancestor(
    of: find.text(label),
    matching: find.byType(FilledButton),
  );
  final button = tester.widget<FilledButton>(finder);
  final onPressed = button.onPressed;
  expect(onPressed, isNotNull);
  onPressed!();
  await tester.pump();
}

Future<void> _tapDownload(WidgetTester tester) async {
  final finder = find.ancestor(
    of: find.text('下载'),
    matching: find.byType(FilledButton),
  );
  final button = tester.widget<FilledButton>(finder);
  final onPressed = button.onPressed;
  expect(onPressed, isNotNull);
  await tester.runAsync(() async {
    await (onPressed! as Future<void> Function())();
  });
  await _pumpUntilCondition(
    tester,
    () => find.byKey(const Key('tasks_list')).evaluate().isNotEmpty,
    timeout: const Duration(seconds: 10),
  );
}

Future<void> _enqueueFixtureMp4(
  WidgetTester tester, {
  required String title,
}) async {
  final addTab = find.text('添加');
  await tester.ensureVisible(addTab);
  await tester.tap(addTab);
  await _pumpUntilCondition(
    tester,
    () => find.byKey(const Key('add_url_field')).evaluate().isNotEmpty,
  );
  await tester.enterText(
    find.byKey(const Key('add_url_field')),
    _fixtureMp4Url,
  );
  await tester.tap(find.byKey(const Key('add_resolve_button')));
  await pumpEngineEvents(tester);
  await _pumpUntilCondition(
    tester,
    () => find.byKey(const Key('resolve_title_field')).evaluate().isNotEmpty,
  );

  await tester.enterText(find.byKey(const Key('resolve_title_field')), title);
  await _tapDownload(tester);
  expect(find.text(title), findsWidgets);
}

Future<void> _waitForTaskCompleted(
  WidgetTester tester, {
  required String title,
}) async {
  final end = DateTime.now().add(const Duration(seconds: 60));
  while (DateTime.now().isBefore(end)) {
    for (final task in _testRepo(tester).listTasks()) {
      if (task.title != title) {
        continue;
      }
      if (task.status == TaskStatus.completed) {
        await pumpEngineEvents(tester);
        await _waitForLibraryItemInEngine(tester, title);
        _invalidateLibrary(tester);
        await pumpEngineEvents(tester);
        return;
      }
      if (task.status == TaskStatus.failed) {
        fail('Download failed: ${task.errorMessage ?? "unknown error"}');
      }
    }

    final tasksTab = find.text('任务');
    await tester.ensureVisible(tasksTab);
    await tester.tap(tasksTab);
    await pumpEngineEvents(tester);
    await Future<void>.delayed(const Duration(seconds: 1));
  }
  fail('Timed out waiting for task to complete');
}

void _ensureMediaKit() {
  MediaKit.ensureInitialized();
}

Future<void> _runPlayerResumeFlow(WidgetTester tester, String title) async {
  _ensureMediaKit();
  await _openLibraryDetailForTitle(tester, title);
  await _tapFilledButton(tester, '播放');
  await _pumpUntilCondition(
    tester,
    () => find.byIcon(Icons.arrow_back).evaluate().isNotEmpty,
    timeout: const Duration(seconds: 15),
  );
  await tester.runAsync(() => Future<void>.delayed(const Duration(seconds: 6)));
  await pumpEngineEvents(tester);
  final backButton = tester.widget<IconButton>(
    find.byKey(const Key('player_back')),
  );
  expect(backButton.onPressed, isNotNull);
  backButton.onPressed!();
  await _pumpUntilCondition(
    tester,
    () => find.text('播放').evaluate().isNotEmpty,
    timeout: const Duration(seconds: 5),
  );

  final repo = _testRepo(tester);
  final items = repo
      .listLibrary()
      .where((item) => item.title == title)
      .toList();
  expect(items, hasLength(1));
  final episodes = repo.listEpisodes(items.first.id);
  expect(episodes, hasLength(1));
  expect(episodes.first.positionMs, greaterThan(0));
}

typedef LibraryReadyCallback = Future<void> Function(
  WidgetTester tester,
  String mediaPath,
);

Future<void> runAppUiSmokeFlow(
  WidgetTester tester, {
  bool stopBeforePlay = false,
  LibraryReadyCallback? onLibraryReady,
}) async {
  await _startFixtureServer();
  try {
    final title = 'ui-smoke-${DateTime.now().millisecondsSinceEpoch}';

    await _launchApp(tester, testLabel: 'u1-u3-flow');
    await _enqueueFixtureMp4(tester, title: title);
    expect(find.text(title), findsWidgets);

    await _waitForTaskCompleted(tester, title: title);
    await tester.tap(find.text('片库'));
    await pumpEngineEvents(tester);
    await _pumpUntilCondition(
      tester,
      () =>
          find.byKey(const Key('library_list')).evaluate().isNotEmpty ||
          find.textContaining(title).evaluate().isNotEmpty,
      timeout: const Duration(seconds: 30),
    );
    expect(find.textContaining(title), findsWidgets);

    final repo = _testRepo(tester);
    final items = repo
        .listLibrary()
        .where((item) => item.title == title)
        .toList();
    expect(items, hasLength(1));
    final episodes = repo.listEpisodes(items.first.id);
    expect(episodes, isNotEmpty);
    final mediaPath = episodes.first.filePath;

    if (onLibraryReady != null) {
      await onLibraryReady(tester, mediaPath);
      return;
    }

    if (stopBeforePlay || _skipPlayer) {
      await _openLibraryDetailForTitle(tester, title);
      return;
    }

    await _runPlayerResumeFlow(tester, title);
  } finally {
    await _stopFixtureServer();
  }
}
