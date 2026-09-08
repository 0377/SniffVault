import 'dart:async';

import 'package:flutter_test/flutter_test.dart';

/// Polls [condition] until true or [timeout] elapses.
Future<void> pumpUntil(
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

/// Advances frames for engine / WebView async work without racing [WidgetTester] guards.
Future<void> pumpEngineEvents(WidgetTester tester) async {
  await Future<void>.delayed(const Duration(milliseconds: 50));
  try {
    await tester.pump(const Duration(milliseconds: 1)).timeout(
      const Duration(seconds: 5),
    );
  } on TimeoutException {
    // Do not return while pump is still in flight — that causes guarded conflicts
    // when the next expect() runs. Prefer waiting out a slow CI frame.
    await tester.pump(const Duration(milliseconds: 1));
  }
}
