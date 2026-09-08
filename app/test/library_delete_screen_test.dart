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
  testWidgets('W9 delete item calls repo and invalidates library', (tester) async {
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
    await tester.tap(find.text('删除'));
    await tester.pumpAndSettle();
    await tester.tap(find.widgetWithText(FilledButton, '删除'));
    await tester.pumpAndSettle();

    expect(fake.lastRemovedItemId, 'item-1');
    expect(fake.lastDeleteFiles, isTrue);
    expect(fake.libraryItems, isEmpty);
    expect(libraryReads, greaterThan(1));
    expect(find.text('已删除'), findsOneWidget);
  });

  testWidgets('W9 delete episode calls removeEpisode', (tester) async {
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
    await tester.tap(find.text('删除此分集'));
    await tester.pumpAndSettle();
    await tester.tap(find.widgetWithText(FilledButton, '删除'));
    await tester.pumpAndSettle();

    expect(fake.lastRemovedEpisodeId, 'ep-2');
    expect(fake.lastDeleteFiles, isTrue);
    expect(fake.episodesByItemId['series-1'], hasLength(1));
    expect(fake.episodesByItemId['series-1']!.first.id, 'ep-1');
    expect(libraryReads, greaterThan(1));
    expect(find.text('已删除'), findsOneWidget);
  });

  testWidgets('W9 delete item failure shows error SnackBar', (tester) async {
    final fake = _setupSingleItemFake();
    fake.removeLibraryItemError = EngineException(
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
    await tester.tap(find.text('删除'));
    await tester.pumpAndSettle();
    await tester.tap(find.widgetWithText(FilledButton, '删除'));
    await tester.pumpAndSettle();

    expect(fake.lastRemovedItemId, isNull);
    expect(fake.libraryItems, hasLength(1));
    expect(find.text('已删除'), findsNothing);
    expect(find.text('本地存储异常：disk full'), findsOneWidget);
    expect(find.text('片库列表'), findsNothing);
  });

  testWidgets('W9 delete episode failure shows error SnackBar', (tester) async {
    final fake = _setupSeriesFake();
    fake.removeEpisodeError = EngineException(
      const FfiError(kind: 'io', message: 'disk full'),
    );
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
    await tester.tap(find.text('删除此分集'));
    await tester.pumpAndSettle();
    await tester.tap(find.widgetWithText(FilledButton, '删除'));
    await tester.pumpAndSettle();

    expect(fake.lastRemovedEpisodeId, isNull);
    expect(fake.episodesByItemId['series-1'], hasLength(2));
    expect(find.text('已删除'), findsNothing);
    expect(find.text('本地存储异常：disk full'), findsOneWidget);
  });
}

FakeEngineRepository _setupSingleItemFake() {
  final episodeFile = File(
    '${Directory.systemTemp.path}/library_delete_test_${DateTime.now().microsecondsSinceEpoch}.mp4',
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
