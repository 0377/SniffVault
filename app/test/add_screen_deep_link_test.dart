import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:video_sniffing/features/add/add_screen.dart';
import 'package:video_sniffing/providers/device_profile.dart';

void main() {
  testWidgets('W13 updates url field when initialUrl changes', (tester) async {
    await tester.pumpWidget(
      ProviderScope(
        overrides: [isTelevisionProvider.overrideWith((ref) async => false)],
        child: const MaterialApp(
          home: AddScreen(initialUrl: 'https://first.example'),
        ),
      ),
    );
    await tester.pumpWidget(
      ProviderScope(
        overrides: [isTelevisionProvider.overrideWith((ref) async => false)],
        child: const MaterialApp(
          home: AddScreen(initialUrl: 'https://second.example'),
        ),
      ),
    );
    final field = tester.widget<TextField>(
      find.byKey(const Key('add_url_field')),
    );
    expect(field.controller?.text, 'https://second.example');
  });
}
