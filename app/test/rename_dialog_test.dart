import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:video_sniffing/features/library/widgets/rename_dialog.dart';

void main() {
  testWidgets('RenameDialog returns trimmed title on save', (tester) async {
    String? result;
    await tester.pumpWidget(MaterialApp(
      home: Builder(builder: (context) {
        return ElevatedButton(
          onPressed: () async {
            result = await showRenameDialogResult(
              context,
              initialTitle: '  旧  ',
            );
          },
          child: const Text('open'),
        );
      }),
    ));
    await tester.tap(find.text('open'));
    await tester.pumpAndSettle();
    await tester.enterText(
      find.byKey(const Key('rename_dialog_field')),
      '  新名  ',
    );
    await tester.tap(find.widgetWithText(FilledButton, '保存'));
    await tester.pumpAndSettle();
    expect(result, '新名');
  });
}
