import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:video_sniffing/engine/models/engine_settings.dart';
import 'package:video_sniffing/engine/models/library_episode.dart';
import 'package:video_sniffing/engine/models/library_item.dart';
import 'package:video_sniffing/engine/models/library_item_kind.dart';
import 'package:video_sniffing/features/cast/cast_actions.dart';
import 'package:video_sniffing/features/library/library_detail_screen.dart';
import 'package:video_sniffing/providers/engine_host_provider.dart';
import 'package:video_sniffing/providers/library_provider.dart';
import 'package:video_sniffing/providers/settings_provider.dart';

import 'fakes/fake_engine_repository.dart';
import 'fakes/fake_ready_engine_host.dart';

void main() {
  testWidgets('library detail shows lan disabled snackbar', (tester) async {
    final fake = _setupFake(lanEnabled: false);
    await tester.pumpWidget(_libraryDetailScope(fake));
    await tester.pumpAndSettle();

    await tester.tap(find.byKey(const Key('library_detail_cast')));
    await tester.pumpAndSettle();

    expect(find.text(lanDisabledMessage), findsOneWidget);
    expect(find.text('投送到电视'), findsNothing);
  });

  testWidgets('library detail opens cast sheet when lan enabled', (tester) async {
    final fake = _setupFake(lanEnabled: true);
    await tester.pumpWidget(_libraryDetailScope(fake));
    await tester.pumpAndSettle();

    await tester.tap(find.byKey(const Key('library_detail_cast')));
    await tester.pumpAndSettle();

    expect(find.text('投送到 TV'), findsOneWidget);
    expect(
      find.text('未找到电视，请确认电视已开启局域网投送'),
      findsOneWidget,
    );
  });

  testWidgets('requestCast shows lan disabled snackbar', (tester) async {
    final fake = _setupFake(lanEnabled: false);
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          engineRepositoryProvider.overrideWithValue(fake),
          settingsProvider.overrideWith((ref) => fake.settings()),
        ],
        child: MaterialApp(
          home: _CastProbe(episodeId: 'ep-1'),
        ),
      ),
    );
    await tester.pumpAndSettle();

    await tester.tap(find.byKey(const Key('cast_probe')));
    await tester.pumpAndSettle();

    expect(find.text(lanDisabledMessage), findsOneWidget);
  });
}

FakeEngineRepository _setupFake({required bool lanEnabled}) {
  final episodeFile = File(
    '${Directory.systemTemp.path}/cast_entry_test_${DateTime.now().microsecondsSinceEpoch}.mp4',
  );
  episodeFile.writeAsStringSync('test');

  final fake = FakeEngineRepository(
    settingsValue: EngineSettings.defaults.copyWith(lanEnabled: lanEnabled),
    libraryItems: const [
      LibraryItem(
        id: 'item-1',
        kind: LibraryItemKind.single,
        title: '测试影片',
        createdAtMs: 1,
      ),
    ],
  );
  fake.episodesByItemId = {
    'item-1': [
      LibraryEpisode(
        id: 'ep-1',
        itemId: 'item-1',
        index: 1,
        title: '正片',
        filePath: episodeFile.path,
        positionMs: 0,
      ),
    ],
  };
  return fake;
}

Widget _libraryDetailScope(FakeEngineRepository fake) {
  return ProviderScope(
    overrides: [
      engineHostProvider.overrideWith((ref) async => FakeReadyEngineHost()),
      engineRepositoryProvider.overrideWithValue(fake),
      settingsProvider.overrideWith((ref) => fake.settings()),
      libraryProvider.overrideWith((ref) => fake.libraryItems),
    ],
    child: const MaterialApp(
      home: LibraryDetailScreen(itemId: 'item-1'),
    ),
  );
}

class _CastProbe extends ConsumerWidget {
  const _CastProbe({required this.episodeId});

  final String episodeId;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    return Scaffold(
      body: Center(
        child: FilledButton(
          key: const Key('cast_probe'),
          onPressed: () => requestCast(context, ref, episodeId),
          child: const Text('投送'),
        ),
      ),
    );
  }
}
