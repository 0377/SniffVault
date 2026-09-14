import 'dart:convert';
import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';
import 'package:sqlite3/sqlite3.dart';
import 'package:video_sniffing/engine/engine_host.dart';

import 'support/cast_flow.dart';

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();

  testWidgets('U11c engine listLibrary removeLibraryItem deletes poster', (
    tester,
  ) async {
    final dataDir = Directory.systemTemp.createTempSync('u11c');
    final host = await EngineHost.open(dataDir.path);
    try {
      final episode = await seedCachedEpisode(tester, host);
      final itemId = episode.itemId;
      final settings = host.settings();
      final postersDir = Directory(
        '${dataDir.path}/${settings.mediaDir}/.posters',
      );
      await postersDir.create(recursive: true);
      final posterFile = File('${postersDir.path}/$itemId.jpg');
      await posterFile.writeAsBytes(_samplePosterBytes());
      final posterPath = posterFile.absolute.path;

      final db = sqlite3.open('${dataDir.path}/library.db');
      try {
        db.execute(
          "UPDATE library_items SET poster_path='${_escapeSql(posterPath)}' WHERE id='$itemId'",
        );
      } finally {
        db.dispose();
      }

      final listed = host.listLibrary().firstWhere((item) => item.id == itemId);
      expect(listed.posterPath, posterPath);
      expect(File(posterPath).existsSync(), isTrue);

      host.removeLibraryItem(itemId, deleteFiles: true);
      expect(File(posterPath).existsSync(), isFalse);
      expect(host.listLibrary(), isEmpty);
    } finally {
      host.dispose();
    }
  }, timeout: const Timeout(Duration(minutes: 3)));
}

const _sampleJpegBase64 =
    '/9j/4AAQSkZJRgABAQAAAQABAAD/2wBDAAgGBgcGBQgHBwcJCQgKDBQNDAsLDBkSEw8UHRofHh0aHBwgJC4nICIsIxwcKDcpLDAxNDQ0Hyc5PTgyPC4zNDL/wAALCAABAAEBAREA/8QAHwAAAQUBAQEBAQEAAAAAAAAAAAECAwQFBgcICQoL/8QAtRAAAgEDAwIEAwUFBAQAAAF9AQIDAAQRBRIhMUEGE1FhByJxFDKBkaEII0KxwRVS0fAkM2JyggkKFhcYGRolJicoKSo0NTY3ODk6Q0RFRkdISUpTVFVWV1hZWmNkZWZnaGlqc3R1dnd4eXqDhIWGh4iJipKTlJWWl5iZmqKjpKWmp6ipqrKztLW2t7i5usLDxMXGx8jJytLT1NXW19jZ2uHi4+Tl5ufo6erx8vP09fb3+Pn6/9oACAEBAAA/ADf/2Q==';

List<int> _samplePosterBytes() {
  final candidates = [
    '../engine/tests/fixtures/posters/sample.jpg',
    '../../engine/tests/fixtures/posters/sample.jpg',
    '../../../engine/tests/fixtures/posters/sample.jpg',
  ];
  for (final path in candidates) {
    final file = File(path);
    if (file.existsSync()) {
      return file.readAsBytesSync();
    }
  }
  return base64Decode(_sampleJpegBase64);
}

String _escapeSql(String value) => value.replaceAll("'", "''");
