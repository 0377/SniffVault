import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:path_provider/path_provider.dart';
import 'package:video_sniffing/app.dart';
import 'package:video_sniffing/deep_link/deep_link_providers.dart';
import 'package:video_sniffing/engine/engine_host.dart';
import 'package:video_sniffing/engine/native_bindings.dart';
import 'package:video_sniffing/features/add/add_screen.dart';
import 'package:video_sniffing/providers/engine_host_provider.dart';

import 'test_pump.dart';

Future<void> _pumpUntilCondition(
  WidgetTester tester,
  bool Function() condition, {
  Duration timeout = const Duration(seconds: 30),
}) async {
  final end = DateTime.now().add(timeout);
  while (DateTime.now().isBefore(end)) {
    if (condition()) {
      return;
    }
    await pumpEngineEvents(tester);
  }
  fail('Timed out after ${timeout.inSeconds}s waiting for condition');
}

/// U8：冷启动 `sniffvault://add?url=...` 预填添加页 URL 字段。
Future<void> runDeepLinkPrefillFlow(WidgetTester tester) async {
  setBootstrapIngressUri(
    Uri.parse('sniffvault://add?url=https%3A%2F%2Fexample.com'),
  );

  final temp = await getTemporaryDirectory();
  final dir = Directory(
    '${temp.path}/deep_link_test_${DateTime.now().millisecondsSinceEpoch}',
  );
  await dir.create(recursive: true);

  openNativeLibrary();

  tester.view.physicalSize = const Size(400, 800);
  tester.view.devicePixelRatio = 1.0;
  addTearDown(tester.view.resetPhysicalSize);
  addTearDown(tester.view.resetDevicePixelRatio);
  addTearDown(() => setBootstrapIngressUri(null));

  await tester.pumpWidget(
    ProviderScope(
      overrides: [
        engineHostProvider.overrideWith((ref) async {
          final host = await EngineHost.open(dir.path);
          ref.onDispose(host.dispose);
          return host;
        }),
        ingressUriStreamProvider.overrideWithValue(const Stream.empty()),
      ],
      child: const VideoSniffingApp(),
    ),
  );

  await _pumpUntilCondition(
    tester,
    () =>
        find.text('正在初始化引擎…').evaluate().isEmpty &&
        find.byType(AddScreen).evaluate().isNotEmpty,
  );

  final fieldFinder = find.byKey(const Key('add_url_field'));
  expect(fieldFinder, findsOneWidget);
  final field = tester.widget<TextField>(fieldFinder);
  expect(field.controller?.text, isNotEmpty);
  expect(field.controller?.text, 'https://example.com');
  expect(find.textContaining('example.com'), findsWidgets);
}
