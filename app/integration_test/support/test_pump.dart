import 'dart:async';

import 'package:flutter_test/flutter_test.dart';

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
