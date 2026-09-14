import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';
import 'package:sqlite3/sqlite3.dart';
import 'package:video_sniffing/engine/engine_host.dart';

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();

  testWidgets('U11c engine poster path list and delete', (tester) async {
    final dataDir = Directory.systemTemp.createTempSync('u11c');
    final host = await EngineHost.open(dataDir.path);
    try {
      const itemId = 'eeeeeeee-eeee-eeee-eeee-eeeeeeeeeeee';
      const epId = 'ffffffff-ffff-ffff-ffff-ffffffffffff';

      final settings = host.settings();
      final mediaDir = Directory('${dataDir.path}/${settings.mediaDir}');
      await mediaDir.create(recursive: true);
      final postersDir = Directory('${mediaDir.path}/.posters');
      await postersDir.create(recursive: true);

      final posterFile = File('${postersDir.path}/$itemId.jpg');
      await posterFile.writeAsBytes(const [0x01, 0x02]);
      final videoFile = File('${mediaDir.path}/clip.mp4');
      await videoFile.writeAsBytes(const [0x78]);

      final db = sqlite3.open('${dataDir.path}/library.db');
      try {
        db.execute('BEGIN');
        db.execute('''
INSERT INTO library_items (id, kind, title, season, poster_path, created_at_ms) VALUES
  ('$itemId', 'single', 'u11c-poster', NULL, '${posterFile.path.replaceAll("'", "''")}', 1);
INSERT INTO library_episodes (id, item_id, idx, title, file_path, duration_ms, position_ms, source_url) VALUES
  ('$epId', '$itemId', 1, 'u11c', '${videoFile.path.replaceAll("'", "''")}', 10000, 0, NULL);
''');
        db.execute('COMMIT');
      } catch (error) {
        db.execute('ROLLBACK');
        rethrow;
      } finally {
        db.dispose();
      }

      final item = host.listLibrary().firstWhere((i) => i.id == itemId);
      expect(item.posterPath, isNotNull);
      expect(item.posterPath!, contains('.posters'));
      expect(File(item.posterPath!).existsSync(), isTrue);

      host.removeLibraryItem(itemId, deleteFiles: true);
      expect(posterFile.existsSync(), isFalse);
      expect(host.listLibrary(), isEmpty);
    } finally {
      host.dispose();
    }
  }, timeout: const Timeout(Duration(minutes: 2)));
}
