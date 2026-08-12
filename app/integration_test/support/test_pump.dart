import 'dart:async';

import 'package:flutter_test/flutter_test.dart';

Future<void> pumpEngineEvents(WidgetTester tester) async {
  await Future<void>.delayed(const Duration(milliseconds: 50));
  try {
    await tester.pump(const Duration(milliseconds: 1)).timeout(
      const Duration(milliseconds: 100),
    );
  } on TimeoutException {
    // Integration tests may block in pump when the app cannot foreground.
  }
}
