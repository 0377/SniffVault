import 'package:flutter_test/flutter_test.dart';
import 'package:video_sniffing/engine/models/engine_settings.dart';

void main() {
  group('EngineSettings', () {
    test('fromJson fills defaults for legacy settings without device_id', () {
      final s = EngineSettings.fromJson({
        'media_dir': 'media',
        'max_concurrency': 2,
        'device_name': 'X',
      });
      expect(s.lanEnabled, isFalse);
      expect(s.deviceId, isEmpty);
    });
  });
}
