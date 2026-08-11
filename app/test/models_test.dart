import 'dart:convert';

import 'package:flutter_test/flutter_test.dart';
import 'package:video_sniffing/engine/models/engine_settings.dart';

void main() {
  group('EngineSettings', () {
    test('roundtrip through JSON', () {
      final original = EngineSettings(
        mediaDir: 'videos',
        maxConcurrency: 4,
        defaultQualityLabel: '1080p',
        userAgent: 'TestAgent/1.0',
        deviceName: 'MyDevice',
      );

      final json = original.toJson();
      final decoded = EngineSettings.fromJson(json);

      expect(decoded, original);
    });

    test('roundtrip through encoded JSON string', () {
      final original = EngineSettings.defaults;

      final encoded = jsonEncode(original.toJson());
      final decoded =
          EngineSettings.fromJson(jsonDecode(encoded) as Map<String, dynamic>);

      expect(decoded, original);
    });
  });
}
