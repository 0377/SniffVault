import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:go_router/go_router.dart';
import 'package:video_sniffing/engine/engine_host.dart';
import 'package:video_sniffing/engine/models/ffi_response.dart';
import 'package:video_sniffing/engine/models/library_episode.dart';
import 'package:video_sniffing/engine/models/library_item.dart';
import 'package:video_sniffing/engine/models/library_item_kind.dart';
import 'package:video_sniffing/features/library/library_detail_screen.dart';
import 'package:video_sniffing/providers/engine_host_provider.dart';
import 'package:video_sniffing/providers/library_provider.dart';

import 'fakes/fake_engine_repository.dart';
import 'fakes/fake_ready_engine_host.dart';

void main() {
  testWidgets('W9b-2 rename item calls repo and invalidates library', (tester) async {
    final fake = _setupSingleItemFake();
    var libraryReads = 0;

    await tester.pumpWidget(
      _detailScope(
        fake,
        libraryReads: () => libraryReads++,
      ),
    );
    await tester.pumpAndSettle();

    await tester.tap(find.byKey(const Key('library_detail_menu')));
    await tester.pumpAndSettle();
    await tester.tap(find.text('重命名'));
    await tester.pumpAndSettle();
    await tester.enterText(
      find.byKey(const Key('rename_dialog_field')),
      '新片名',
    );
    await tester.tap(find.widgetWithText(FilledButton, '保存'));
    await tester.pumpAndSettle();

    expect(fake.lastRenamedItemId, 'item-1');
    expect(fake.lastRenamedTitle, '新片名');
    expect(libraryReads, greaterThan(1));
    expect(find.text('已重命名'), findsOneWidget);
  });

  testWidgets('W9b-5 rename item failure shows error SnackBar', (tester) async {
    final fake = _setupSingleItemFake();
    fake.renameLibraryItemError = EngineException(
      const FfiError(kind: 'io', message: 'disk full'),
    );
    var libraryReads = 0;

    await tester.pumpWidget(
      _detailScope(
        fake,
        libraryReads: () => libraryReads++,
      ),
    );
    await tester.pumpAndSettle();

    await tester.tap(find.byKey(const Key('library_detail_menu')));
    await tester.pumpAndSettle();
    await tester.tap(find.text('重命名'));
    await tester.pumpAndSettle();
    await tester.enterText(
      find.byKey(const Key('rename_dialog_field')),
      '新片名',
    );
    await tester.tap(find.widgetWithText(FilledButton, '保存'));
    await tester.pumpAndSettle();

    expect(fake.lastRenamedItemId, isNull);
    expect(fake.libraryItems.first.title, '测试影片');
    expect(find.text('已重命名'), findsNothing);
    expect(find.text('本地存储异常：disk full'), findsOneWidget);
  });

  testWidgets('W9b-6 rename episode calls repo.renameEpisode', (tester) async {
    final fake = _setupSeriesFake();
    var libraryReads = 0;

    await tester.pumpWidget(
      _detailScope(
        fake,
        itemId: 'series-1',
        libraryReads: () => libraryReads++,
      ),
    );
    await tester.pumpAndSettle();

    await tester.tap(find.byKey(const Key('episode_menu_ep-2')));
    await tester.pumpAndSettle();
    await tester.tap(find.text('重命名'));
    await tester.pumpAndSettle();
    await tester.enterText(
      find.byKey(const Key('rename_dialog_field')),
      '新第2集',
    );
    await tester.tap(find.widgetWithText(FilledButton, '保存'));
    await tester.pumpAndSettle();

    expect(fake.lastRenamedEpisodeId, 'ep-2');
    expect(fake.lastRenamedEpisodeTitle, '新第2集');
    expect(libraryReads, greaterThan(1));
    expect(find.text('已重命名'), findsOneWidget);
  });
}

FakeEngineRepository _setupSingleItemFake() {
  final episodeFile = File(
    '${Directory.systemTemp.path}/library_rename_test_${DateTime.now().microsecondsSinceEpoch}.mp4',
  );
  episodeFile.writeAsStringSync('test');

  final fake = FakeEngineRepository(
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

FakeEngineRepository _setupSeriesFake() {
  final fake = FakeEngineRepository(
    libraryItems: const [
      LibraryItem(
        id: 'series-1',
        kind: LibraryItemKind.series,
        title: '测试剧',
        season: 1,
        createdAtMs: 1,
      ),
    ],
  );
  fake.episodesByItemId = {
    'series-1': [
      const LibraryEpisode(
        id: 'ep-1',
        itemId: 'series-1',
        index: 1,
        title: '第1集',
        filePath: '/tmp/e1.mp4',
        positionMs: 0,
      ),
      const LibraryEpisode(
        id: 'ep-2',
        itemId: 'series-1',
        index: 2,
        title: '第2集',
        filePath: '/tmp/e2.mp4',
        positionMs: 0,
      ),
    ],
  };
  return fake;
}

Widget _detailScope(
  FakeEngineRepository fake, {
  String itemId = 'item-1',
  required void Function() libraryReads,
}) {
  final router = GoRouter(
    initialLocation: '/library/$itemId',
    routes: [
      GoRoute(
        path: '/library',
        builder: (_, _) => const Scaffold(body: Text('片库列表')),
        routes: [
          GoRoute(
            path: ':itemId',
            builder: (_, state) => LibraryDetailScreen(
              itemId: state.pathParameters['itemId']!,
            ),
          ),
        ],
      ),
    ],
  );

  return ProviderScope(
    overrides: [
      engineHostProvider.overrideWith((ref) async => FakeReadyEngineHost()),
      engineRepositoryProvider.overrideWithValue(fake),
      libraryProvider.overrideWith((ref) {
        libraryReads();
        return List<LibraryItem>.from(fake.libraryItems);
      }),
    ],
    child: MaterialApp.router(routerConfig: router),
  );
}
