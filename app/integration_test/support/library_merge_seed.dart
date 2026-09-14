import 'dart:io';

import 'package:video_sniffing/engine/engine_host.dart';

const _sourceItemId = 'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa';
const _targetItemId = 'bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb';
const _sourceEpId = 'cccccccc-cccc-cccc-cccc-cccccccccccc';
const _targetEpId = 'dddddddd-dddd-dddd-dddd-dddddddddddd';

/// 经 [EngineHost] 已打开的 data 目录写入两个同 title+season 的 Series 壳；
/// source 写入 idx=1，target 写入 idx=2（避免冲突），返回 (sourceItemId, targetItemId)。
Future<(String, String)> seedDuplicateSeriesForIntegration(
  EngineHost host,
  String dataDir, {
  String title = '示意剧',
  int season = 1,
}) async {
  final settings = host.settings();
  final mediaDir = Directory('$dataDir/${settings.mediaDir}');
  await mediaDir.create(recursive: true);

  final sourceFile = File('${mediaDir.path}/s1.mp4');
  final targetFile = File('${mediaDir.path}/t2.mp4');
  await sourceFile.writeAsBytes(const [0x78]);
  await targetFile.writeAsBytes(const [0x78]);

  final dbPath = '$dataDir/library.db';
  final sql = '''
INSERT INTO library_items (id, kind, title, season, poster_path, created_at_ms) VALUES
  ('$_sourceItemId', 'series', '${_escapeSql(title)}', $season, NULL, 1),
  ('$_targetItemId', 'series', '${_escapeSql(title)}', $season, NULL, 2);
INSERT INTO library_episodes (id, item_id, idx, title, file_path, duration_ms, position_ms, source_url) VALUES
  ('$_sourceEpId', '$_sourceItemId', 1, '源1', '${_escapeSql(sourceFile.path)}', 10000, 0, NULL),
  ('$_targetEpId', '$_targetItemId', 2, '目标2', '${_escapeSql(targetFile.path)}', 10000, 0, NULL);
''';

  final result = await Process.run('sqlite3', [dbPath, sql]);
  if (result.exitCode != 0) {
    throw StateError(
      'sqlite3 seed failed (exit ${result.exitCode}): ${result.stderr}',
    );
  }

  return (_sourceItemId, _targetItemId);
}

String _escapeSql(String value) => value.replaceAll("'", "''");
