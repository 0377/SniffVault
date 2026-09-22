import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:video_sniffing/features/settings/media_directory_picker.dart';
import 'package:video_sniffing/features/settings/settings_screen.dart';
import 'package:video_sniffing/providers/device_profile.dart';
import 'package:video_sniffing/providers/engine_host_provider.dart';
import 'package:video_sniffing/providers/settings_provider.dart';

import 'fakes/fake_engine_repository.dart';

class FakeMediaDirectoryPicker implements MediaDirectoryPicker {
  FakeMediaDirectoryPicker(this.result);
  final String? result;

  @override
  Future<String?> pickDirectoryPath() async => result;
}

void main() {
  testWidgets('W9d-2 pick media dir fills basename into field', (tester) async {
    tester.view.physicalSize = const Size(800, 1200);
    tester.view.devicePixelRatio = 1.0;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);

    final fake = FakeEngineRepository();
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          engineRepositoryProvider.overrideWithValue(fake),
          settingsProvider.overrideWith((ref) => fake.settings()),
          isTelevisionProvider.overrideWith((ref) async => false),
        ],
        child: MaterialApp(
          home: SettingsScreen(
            directoryPicker: FakeMediaDirectoryPicker(
              '/Users/me/Downloads/MyVault',
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    await tester.tap(find.byKey(const Key('settings_pick_media_dir')));
    await tester.pumpAndSettle();

    expect(
      find.byKey(const Key('settings_media_dir')),
      findsOneWidget,
    );
    final field = tester.widget<TextField>(
      find.byKey(const Key('settings_media_dir')),
    );
    expect(field.controller?.text, 'MyVault');
    expect(find.text('已填入目录名，记得保存'), findsOneWidget);
  });

  testWidgets('W9d-3 TV hides pick media dir button', (tester) async {
    tester.view.physicalSize = const Size(800, 1200);
    tester.view.devicePixelRatio = 1.0;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);

    final fake = FakeEngineRepository();
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          engineRepositoryProvider.overrideWithValue(fake),
          settingsProvider.overrideWith((ref) => fake.settings()),
          isTelevisionProvider.overrideWith((ref) async => true),
        ],
        child: const MaterialApp(home: SettingsScreen()),
      ),
    );
    await tester.pumpAndSettle();

    expect(find.byKey(const Key('settings_pick_media_dir')), findsNothing);
    expect(find.textContaining('选择外置路径'), findsNothing);
    expect(find.textContaining('应用数据目录下的文件夹名称'), findsOneWidget);
  });
}
