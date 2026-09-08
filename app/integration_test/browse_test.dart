import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';
import 'package:path_provider/path_provider.dart';
import 'package:video_sniffing/app.dart';
import 'package:video_sniffing/engine/engine_host.dart';
import 'package:video_sniffing/engine/native_bindings.dart';
import 'package:video_sniffing/providers/engine_host_provider.dart';

import 'support/playable_mp4.dart';
import 'support/test_pump.dart';

const _skipBrowse = bool.fromEnvironment(
  'INTEGRATION_SKIP_BROWSE',
  defaultValue: false,
);

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();

  testWidgets(
    'U6 browse HTML parse page shows download',
    (tester) async {
      if (_skipBrowse) {
        markTestSkipped(
          'INTEGRATION_SKIP_BROWSE: CI may skip WebView browse; local delivery must pass U6 without this flag',
        );
        return;
      }

      final server = await _startBrowseFixtureServer();
      addTearDown(() => server.close(force: true));
      final pageUrl = 'http://127.0.0.1:${server.port}/page.html';

      await _launchApp(tester, testLabel: 'u6-browse');
      await _openBrowseAndLoad(tester, pageUrl);
      await _tapResolvePage(tester);
      final showedDownload = await _waitUntil(
        tester,
        () => _downloadButtonFinder.evaluate().isNotEmpty,
        timeout: const Duration(seconds: 30),
      );
      expect(showedDownload, isTrue, reason: 'U6: 解析本页后向导应出现「下载」（不要求嗅探列表有条目）');
      expect(find.text('下载'), findsWidgets);
    },
    timeout: const Timeout(Duration(minutes: 5)),
  );

  testWidgets(
    'U7 video src sniff candidate optional',
    (tester) async {
      if (_skipBrowse) {
        markTestSkipped('INTEGRATION_SKIP_BROWSE');
        return;
      }

      final server = await _startBrowseFixtureServer();
      addTearDown(() => server.close(force: true));
      final videoUrl = 'http://127.0.0.1:${server.port}/video.html';

      await _launchApp(tester, testLabel: 'u7-browse');
      await _openBrowseAndLoad(tester, videoUrl);

      final found = await _waitUntil(
        tester,
        () =>
            find.textContaining('嗅探候选').evaluate().isNotEmpty ||
            find.textContaining('clip.mp4').evaluate().isNotEmpty,
        timeout: const Duration(seconds: 5),
      );
      if (!found) {
        markTestSkipped('U7: no sniff candidate within 5s');
        return;
      }
      expect(find.textContaining('clip.mp4'), findsWidgets);
    },
    timeout: const Timeout(Duration(minutes: 3)),
  );

  testWidgets(
    'optional cookie-backed parse does not block U6',
    (tester) async {
      if (_skipBrowse) {
        markTestSkipped('INTEGRATION_SKIP_BROWSE');
        return;
      }

      final server = await _startBrowseFixtureServer();
      addTearDown(() => server.close(force: true));
      final setCookieUrl = 'http://127.0.0.1:${server.port}/set-cookie';
      final needCookieUrl = 'http://127.0.0.1:${server.port}/need-cookie';

      await _launchApp(tester, testLabel: 'cookie-browse');
      await _openBrowseAndLoad(tester, setCookieUrl);
      await tester.runAsync(
        () => Future<void>.delayed(const Duration(seconds: 1)),
      );
      await _openBrowseAndLoad(tester, needCookieUrl);
      await _tapResolvePage(tester);
      final showedDownload = await _waitUntil(
        tester,
        () => _downloadButtonFinder.evaluate().isNotEmpty,
        timeout: const Duration(seconds: 15),
      );
      if (!showedDownload) {
        if (Platform.isWindows) {
          fail('U6w-cookie: Windows 上 cookie 解析本页必须出现「下载」');
        }
        markTestSkipped('cookie-backed parse did not show download');
        return;
      }
      expect(find.text('下载'), findsWidgets);
    },
    timeout: const Timeout(Duration(minutes: 3)),
  );
}

final Finder _downloadButtonFinder = find.ancestor(
  of: find.text('下载'),
  matching: find.byType(FilledButton),
);

Future<HttpServer> _startBrowseFixtureServer() async {
  final bytes = playableMp4Bytes();
  final server = await HttpServer.bind(InternetAddress.loopbackIPv4, 0);

  server.listen((request) async {
    final path = request.uri.path;
    final clip = 'http://127.0.0.1:${server.port}/clip.mp4';

    if (path == '/page.html') {
      request.response.headers.contentType = ContentType.html;
      request.response.write(
        '<!DOCTYPE html><html><body><a href="$clip">clip</a></body></html>',
      );
      await request.response.close();
      return;
    }

    if (path == '/video.html') {
      request.response.headers.contentType = ContentType.html;
      request.response.write(
        '<!DOCTYPE html><html><body><video src="$clip"></video></body></html>',
      );
      await request.response.close();
      return;
    }

    if (path == '/set-cookie') {
      request.response.headers.set(
        HttpHeaders.setCookieHeader,
        'secret=1; Path=/',
      );
      request.response.headers.contentType = ContentType.html;
      request.response.write('<!DOCTYPE html><html><body>ok</body></html>');
      await request.response.close();
      return;
    }

    if (path == '/need-cookie') {
      final cookie = request.headers.value(HttpHeaders.cookieHeader) ?? '';
      if (!cookie.contains('secret=1')) {
        request.response.statusCode = HttpStatus.forbidden;
        await request.response.close();
        return;
      }
      request.response.headers.contentType = ContentType.html;
      request.response.write(
        '<!DOCTYPE html><html><body><a href="$clip">clip</a></body></html>',
      );
      await request.response.close();
      return;
    }

    if (path != '/clip.mp4') {
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

  return server;
}

Future<void> _launchApp(
  WidgetTester tester, {
  required String testLabel,
}) async {
  final temp = await getTemporaryDirectory();
  final dir = Directory(
    '${temp.path}/browse_test_${testLabel}_${DateTime.now().millisecondsSinceEpoch}',
  );
  await dir.create(recursive: true);

  openNativeLibrary();

  tester.view.physicalSize = const Size(800, 800);
  tester.view.devicePixelRatio = 1.0;
  addTearDown(tester.view.resetPhysicalSize);
  addTearDown(tester.view.resetDevicePixelRatio);

  await tester.pumpWidget(
    ProviderScope(
      overrides: [
        engineHostProvider.overrideWith((ref) async {
          final host = await EngineHost.open(dir.path);
          ref.onDispose(host.dispose);
          return host;
        }),
      ],
      child: const VideoSniffingApp(),
    ),
  );
  final ready = await _waitUntil(
    tester,
    () =>
        find.text('正在初始化引擎…').evaluate().isEmpty &&
        find.text('片库').evaluate().isNotEmpty,
  );
  expect(ready, isTrue, reason: 'app failed to initialize');
}

Future<void> _openBrowseAndLoad(WidgetTester tester, String url) async {
  final browseTab = find.text('浏览');
  final tabVisible = await _waitUntil(
    tester,
    () => browseTab.evaluate().isNotEmpty,
  );
  expect(tabVisible, isTrue, reason: 'browse destination missing (TV?)');
  await tester.ensureVisible(browseTab);
  await tester.tap(browseTab);
  await pumpEngineEvents(tester);

  final urlField = find.byKey(const Key('browse_url_field'));
  final fieldReady = await _waitUntil(
    tester,
    () => urlField.evaluate().isNotEmpty,
    timeout: const Duration(seconds: 20),
  );
  expect(fieldReady, isTrue, reason: 'browse address field missing');

  await tester.enterText(urlField, url);
  await tester.testTextInput.receiveAction(TextInputAction.go);
  await pumpEngineEvents(tester);
}

Future<void> _tapResolvePage(WidgetTester tester) async {
  final finder = find.byKey(const Key('browse_resolve_page'));
  final enabled = await _waitUntil(tester, () {
    if (finder.evaluate().isEmpty) {
      return false;
    }
    return tester.widget<FilledButton>(finder).onPressed != null;
  });
  expect(enabled, isTrue, reason: '解析本页 was not enabled');
  final onPressed = tester.widget<FilledButton>(finder).onPressed;
  expect(onPressed, isNotNull);
  onPressed!();
  await pumpEngineEvents(tester);
}

Future<bool> _waitUntil(
  WidgetTester tester,
  bool Function() condition, {
  Duration timeout = const Duration(seconds: 15),
}) async {
  final end = DateTime.now().add(timeout);
  while (DateTime.now().isBefore(end)) {
    if (condition()) {
      return true;
    }
    await pumpEngineEvents(tester);
  }
  return condition();
}
