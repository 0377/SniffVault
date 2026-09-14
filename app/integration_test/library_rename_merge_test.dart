import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';
import 'package:video_sniffing/engine/engine_host.dart';
import 'package:video_sniffing/engine/models/library_item_kind.dart';

import 'support/library_merge_seed.dart';

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();

  testWidgets('U11b engine merge duplicate series shells', (tester) async {
    final dataDir = Directory.systemTemp.createTempSync('u11b');
    final host = await EngineHost.open(dataDir.path);
    try {
      final (sourceId, targetId) = await seedDuplicateSeriesForIntegration(
        host,
        dataDir.path,
      );
      host.mergeLibraryItems(sourceId, targetId);
      final series = host
          .listLibrary()
          .where((i) => i.kind == LibraryItemKind.series)
          .toList();
      expect(series.length, 1);
      expect(series.first.id, targetId);
      expect(host.listEpisodes(targetId).length, 2);
    } finally {
      host.dispose();
    }
  }, timeout: const Timeout(Duration(minutes: 2)));
}
