import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:video_sniffing/features/library/widgets/confirm_delete_dialog.dart';

void main() {
  testWidgets('W9 delete dialog defaults deleteFiles to true', (tester) async {
    ({bool confirmed, bool deleteFiles})? result;
    await tester.pumpWidget(
      MaterialApp(
        home: Builder(
          builder: (context) => ElevatedButton(
            onPressed: () async {
              result = await showConfirmDeleteDialogResult(
                context,
                title: '删除「测试」？',
                message: '将删除 1 个文件',
              );
            },
            child: const Text('open'),
          ),
        ),
      ),
    );
    await tester.tap(find.text('open'));
    await tester.pumpAndSettle();
    expect(find.byType(Checkbox), findsOneWidget);
    expect(tester.widget<Checkbox>(find.byType(Checkbox)).value, isTrue);
    await tester.tap(find.text('删除'));
    await tester.pumpAndSettle();
    expect(result?.confirmed, isTrue);
    expect(result?.deleteFiles, isTrue);
  });

  testWidgets('W9 cancel returns null', (tester) async {
    ({bool confirmed, bool deleteFiles})? result;
    await tester.pumpWidget(
      MaterialApp(
        home: Builder(
          builder: (context) => ElevatedButton(
            onPressed: () async {
              result = await showConfirmDeleteDialogResult(
                context,
                title: '删除「测试」？',
                message: '将删除 1 个文件',
              );
            },
            child: const Text('open'),
          ),
        ),
      ),
    );
    await tester.tap(find.text('open'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('取消'));
    await tester.pumpAndSettle();
    expect(result, isNull);
  });
}
