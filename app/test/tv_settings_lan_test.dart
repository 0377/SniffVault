import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:video_sniffing/engine/models/engine_settings.dart';
import 'package:video_sniffing/features/settings/settings_screen.dart';
import 'package:video_sniffing/providers/device_profile.dart';
import 'package:video_sniffing/providers/engine_host_provider.dart';
import 'package:video_sniffing/providers/settings_provider.dart';

import 'fakes/fake_engine_repository.dart';

void main() {
  testWidgets('W17 TV settings shows pairing PIN when lan enabled', (tester) async {
    tester.view.physicalSize = const Size(800, 1200);
    tester.view.devicePixelRatio = 1.0;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);

    final fake = FakeEngineRepository(
      settingsValue: EngineSettings.defaults.copyWith(lanEnabled: true),
    );
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

    expect(find.byKey(const Key('settings_pairing_pin')), findsOneWidget);
    expect(find.text('123456'), findsOneWidget);
    expect(find.textContaining('60'), findsOneWidget);
  });

  testWidgets('W17 TV settings hides PIN when lan disabled', (tester) async {
    tester.view.physicalSize = const Size(800, 1200);
    tester.view.devicePixelRatio = 1.0;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);

    final fake = FakeEngineRepository(
      settingsValue: EngineSettings.defaults.copyWith(lanEnabled: false),
    );
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

    expect(find.byKey(const Key('settings_pairing_pin')), findsNothing);
  });
}
