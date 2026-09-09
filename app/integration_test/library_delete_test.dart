import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';
import 'package:media_kit/media_kit.dart';

import 'support/app_ui_flow.dart';
import 'support/test_pump.dart';

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();
  MediaKit.ensureInitialized();

  testWidgets('U11 library delete removes db entry and file', (tester) async {
    await runAppUiSmokeFlow(
      tester,
      stopBeforePlay: true,
      onLibraryReady: (tester, mediaPath) async {
        await tester.tap(
          find.descendant(
            of: find.byKey(const Key('library_list')),
            matching: find.byType(ListTile),
          ).first,
        );
        await pumpUntil(
          tester,
          () => find.byKey(const Key('library_detail_menu')).evaluate().isNotEmpty,
        );

        // PopupMenuButton 在 macOS 集成测中常落在 AppBar 可视区外；用 TV Shortcuts 同路径触发删除。
        await tester.sendKeyEvent(LogicalKeyboardKey.contextMenu);
        await pumpUntil(
          tester,
          () => find.text('同时删除本地缓存文件').evaluate().isNotEmpty,
        );
        await tester.tap(find.widgetWithText(FilledButton, '删除'));
        await pumpUntil(
          tester,
          () => find.textContaining('片库为空').evaluate().isNotEmpty,
        );

        expect(File(mediaPath).existsSync(), isFalse);
      },
    );
  }, timeout: const Timeout(Duration(minutes: 5)));
}
