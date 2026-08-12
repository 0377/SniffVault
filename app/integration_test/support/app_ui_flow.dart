import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';

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

import 'test_pump.dart';

String? _activeDataDir;
HttpServer? _fixtureServer;

const _playableMp4Base64 =
    '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';

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
  final bytes = Uint8List.fromList(base64Decode(_playableMp4Base64));
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
  final items = repo.listLibrary().where((item) => item.title == title).toList();
  expect(items, hasLength(1));
  final itemId = items.first.id;

  final container = ProviderScope.containerOf(tester.element(find.text('片库')));
  container.read(appRouterProvider).push('/library/$itemId');
  await _pumpUntilCondition(tester, () => find.text('播放').evaluate().isNotEmpty);
}

Future<void> _launchApp(WidgetTester tester, {required String testLabel}) async {
  _activeDataDir = await _createIsolatedDataDir(testLabel);
  MediaKit.ensureInitialized();
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

  await tester.enterText(
    find.byKey(const Key('resolve_title_field')),
    title,
  );
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

Future<void> runAppUiSmokeFlow(WidgetTester tester) async {
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

    await _openLibraryDetailForTitle(tester, title);
    await _tapFilledButton(tester, '播放');
    await _pumpUntilCondition(
      tester,
      () => find.byIcon(Icons.arrow_back).evaluate().isNotEmpty,
      timeout: const Duration(seconds: 15),
    );
    await tester.runAsync(() => Future<void>.delayed(const Duration(seconds: 6)));
    await pumpEngineEvents(tester);
    final backButton =
        tester.widget<IconButton>(find.byKey(const Key('player_back')));
    expect(backButton.onPressed, isNotNull);
    backButton.onPressed!();
    await _pumpUntilCondition(
      tester,
      () => find.text('播放').evaluate().isNotEmpty,
      timeout: const Duration(seconds: 5),
    );

    final repo = _testRepo(tester);
    final items = repo.listLibrary().where((item) => item.title == title).toList();
    expect(items, hasLength(1));
    final episodes = repo.listEpisodes(items.first.id);
    expect(episodes, hasLength(1));
    expect(episodes.first.positionMs, greaterThan(0));
  } finally {
    await _stopFixtureServer();
  }
}
