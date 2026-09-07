import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:video_sniffing/features/browse/browse_chrome.dart';

void main() {
  testWidgets('W6 javascript rejected does not call onSubmit', (tester) async {
    var submitted = false;
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: BrowseChrome(
            onSubmit: (_) {
              submitted = true;
            },
          ),
        ),
      ),
    );

    await tester.enterText(
      find.byKey(const Key('browse_url_field')),
      'javascript:alert(1)',
    );
    await tester.testTextInput.receiveAction(TextInputAction.go);
    await tester.pump();

    expect(submitted, isFalse);
    expect(find.text('非法地址'), findsOneWidget);
  });
}
