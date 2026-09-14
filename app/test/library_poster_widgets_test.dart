import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:go_router/go_router.dart';
import 'package:video_sniffing/engine/models/library_episode.dart';
import 'package:video_sniffing/engine/models/library_item.dart';
import 'package:video_sniffing/engine/models/library_item_kind.dart';
import 'package:video_sniffing/features/library/library_detail_screen.dart';
import 'package:video_sniffing/features/library/widgets/library_card.dart';
import 'package:video_sniffing/providers/engine_host_provider.dart';
import 'package:video_sniffing/providers/library_provider.dart';

import 'fakes/fake_engine_repository.dart';
import 'fakes/fake_ready_engine_host.dart';

File _samplePosterFixture() =>
    File('../engine/tests/fixtures/posters/sample.jpg');

void main() {
  testWidgets('W9c-1 LibraryCard with poster shows Image', (tester) async {
    final posterFile = File(
      '${Directory.systemTemp.path}/w9c1_${DateTime.now().microsecondsSinceEpoch}.jpg',
    );
    posterFile.writeAsBytesSync(_samplePosterFixture().readAsBytesSync());

    final item = LibraryItem(
      id: 'item-poster',
      kind: LibraryItemKind.single,
      title: '有封面片',
      posterPath: posterFile.path,
      createdAtMs: 1,
    );

    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: LibraryCard(item: item, onTap: () {}),
        ),
      ),
    );
    await tester.pumpAndSettle();

    expect(find.byType(Image), findsOneWidget);
  });

  testWidgets('W9c-2 LibraryCard without poster shows placeholder letter',
      (tester) async {
    const item = LibraryItem(
      id: 'item-no-poster',
      kind: LibraryItemKind.single,
      title: '剧',
      createdAtMs: 1,
    );

    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: LibraryCard(item: item, onTap: () {}),
        ),
      ),
    );
    await tester.pumpAndSettle();

    expect(find.byType(Image), findsNothing);
    expect(find.text('剧'), findsWidgets);
  });

  testWidgets('W9c-3a LibraryDetailScreen menu shows 抓取封面 without poster',
      (tester) async {
    final fake = _detailFake(posterPath: null);
    await tester.pumpWidget(_detailScope(fake));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const Key('library_detail_menu')));
    await tester.pumpAndSettle();
    expect(find.text('抓取封面'), findsOneWidget);
    expect(find.text('刷新封面'), findsNothing);
  });

  testWidgets('W9c-3b LibraryDetailScreen menu shows 刷新封面 with poster',
      (tester) async {
    final posterFile = File(
      '${Directory.systemTemp.path}/w9c3_poster_${DateTime.now().microsecondsSinceEpoch}.jpg',
    );
    posterFile.writeAsBytesSync(_samplePosterFixture().readAsBytesSync());

    final fake = _detailFake(posterPath: posterFile.path);
    await tester.pumpWidget(_detailScope(fake));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const Key('library_detail_menu')));
    await tester.pumpAndSettle();
    expect(find.text('刷新封面'), findsOneWidget);
    expect(find.text('抓取封面'), findsNothing);
  });
}

FakeEngineRepository _detailFake({required String? posterPath}) {
  final episodeFile = File(
    '${Directory.systemTemp.path}/w9c3_${DateTime.now().microsecondsSinceEpoch}.mp4',
  );
  episodeFile.writeAsStringSync('test');

  final fake = FakeEngineRepository(
    libraryItems: [
      LibraryItem(
        id: 'item-1',
        kind: LibraryItemKind.single,
        title: '测试影片',
        posterPath: posterPath,
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

Widget _detailScope(FakeEngineRepository fake) {
  final router = GoRouter(
    initialLocation: '/library/item-1',
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
      libraryProvider.overrideWith(
        (ref) => List<LibraryItem>.from(fake.libraryItems),
      ),
    ],
    child: MaterialApp.router(routerConfig: router),
  );
}
