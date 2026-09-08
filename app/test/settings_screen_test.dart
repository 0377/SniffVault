import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:video_sniffing/features/settings/settings_screen.dart';
import 'package:video_sniffing/providers/device_profile.dart';
import 'package:video_sniffing/providers/engine_host_provider.dart';
import 'package:video_sniffing/providers/settings_provider.dart';

import 'fakes/fake_engine_repository.dart';

void main() {
  testWidgets('W4 shows error when media_dir contains slash', (tester) async {
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
        child: const MaterialApp(home: SettingsScreen()),
      ),
    );
    await tester.pumpAndSettle();

    await tester.enterText(find.byKey(const Key('settings_media_dir')), 'bad/dir');
    await tester.tap(find.byKey(const Key('settings_save')));
    await tester.pump();
    expect(
      find.textContaining('media_dir must be a single relative directory name'),
      findsOneWidget,
    );
  });
}
