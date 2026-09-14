import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';
import 'package:sqlite3/sqlite3.dart';
import 'package:video_sniffing/engine/engine_host.dart';
import 'package:video_sniffing/engine/models/task_status.dart';

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();

  testWidgets('U11d retry failed task with new url via EngineHost', (
    tester,
  ) async {
    final dataDir = Directory.systemTemp.createTempSync('u11d');
    final host = await EngineHost.open(dataDir.path);
    try {
      const taskId = 'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa';
      final db = sqlite3.open('${dataDir.path}/tasks.db');
      try {
        db.execute('''
INSERT INTO download_tasks (
  id, parent_id, season, title, source_url, quality_label, status,
  progress_bytes, total_bytes, error_message, output_path,
  library_item_id, episode_index, created_at_ms, updated_at_ms,
  cookie_header, referer, resolved_media_url, poster_url,
  checkpoint_json
) VALUES (
  '$taskId', NULL, NULL, 'u11d', 'https://old.example/bad.mp4', NULL, 'failed',
  100, 200, 'http error', NULL,
  NULL, NULL, 1, 1,
  NULL, NULL, 'https://cdn.example/old.m3u8', NULL,
  '{"media_url":"https://cdn.example/old.m3u8"}'
);
''');
      } finally {
        db.dispose();
      }

      host.retryTask(taskId, newUrl: 'https://new.example/good.mp4');

      final task = host.listTasks().firstWhere((t) => t.id == taskId);
      expect(task.status, TaskStatus.queued);
      expect(task.sourceUrl, 'https://new.example/good.mp4');
      expect(task.resolvedMediaUrl, isNull);
      expect(task.progressBytes, 0);

      final db2 = sqlite3.open('${dataDir.path}/tasks.db');
      try {
        final checkpoint = db2.select(
          "SELECT checkpoint_json FROM download_tasks WHERE id='$taskId'",
        );
        expect(checkpoint.first.columnAt(0), isNull);
      } finally {
        db2.dispose();
      }
    } finally {
      host.dispose();
    }
  }, timeout: const Timeout(Duration(minutes: 2)));
}
