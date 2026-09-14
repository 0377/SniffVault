import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:video_sniffing/features/tasks/widgets/retry_url_dialog.dart';

void main() {
  testWidgets('showRetryUrlDialog returns trimmed url on confirm', (
    tester,
  ) async {
    String? result;
    await tester.pumpWidget(
      MaterialApp(
        home: Builder(
          builder: (context) => ElevatedButton(
            onPressed: () async {
              result = await showRetryUrlDialog(
                context,
                initialUrl: 'https://old.example/a.mp4',
              );
            },
            child: const Text('open'),
          ),
        ),
      ),
    );

    await tester.tap(find.text('open'));
    await tester.pumpAndSettle();

    await tester.enterText(
      find.byKey(const Key('retry_url_field')),
      '  https://new.example/b.mp4  ',
    );
    await tester.tap(find.byKey(const Key('retry_url_confirm')));
    await tester.pumpAndSettle();

    expect(result, 'https://new.example/b.mp4');
  });
}
