import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:video_sniffing/features/browse/browse_load_error.dart';

void main() {
  testWidgets('load error shows in-page message and refresh', (tester) async {
    var retried = false;
    await tester.pumpWidget(
      MaterialApp(
        home: BrowseLoadError(
          message: 'net::ERR_NAME_NOT_RESOLVED',
          onRetry: () => retried = true,
        ),
      ),
    );

    expect(find.textContaining('加载失败'), findsOneWidget);
    expect(find.textContaining('net::ERR_NAME_NOT_RESOLVED'), findsOneWidget);

    await tester.tap(find.text('刷新'));
    await tester.pump();
    expect(retried, isTrue);
  });
}
