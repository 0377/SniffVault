import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:video_sniffing/engine/models/download_task.dart';
import 'package:video_sniffing/engine/models/library_item.dart';
import 'package:video_sniffing/engine/models/library_item_kind.dart';
import 'package:video_sniffing/engine/models/task_status.dart';
import 'package:video_sniffing/features/browse/browse_unavailable_screen.dart';
import 'package:video_sniffing/features/library/library_screen.dart';
import 'package:video_sniffing/features/tasks/tasks_screen.dart';
import 'package:video_sniffing/providers/engine_host_provider.dart';
import 'package:video_sniffing/providers/engine_repository.dart';

import 'fakes/fake_engine_repository.dart';
import 'fakes/fake_ready_engine_host.dart';

const _topInset = 44.0;

Widget _withTopInset(Widget child) {
  return MediaQuery(
    data: const MediaQueryData(padding: EdgeInsets.only(top: _topInset)),
    child: MaterialApp(home: child),
  );
}

ProviderScope _scope({
  required FakeEngineRepository fake,
  required Widget child,
}) {
  return ProviderScope(
    overrides: [
      engineHostProvider.overrideWith((ref) async {
        final host = FakeReadyEngineHost();
        ref.onDispose(host.dispose);
        return host;
      }),
      engineRepositoryProvider.overrideWithValue(fake),
    ],
    child: child,
  );
}

void main() {
  testWidgets('LibraryScreen list starts below top safe area inset', (
    tester,
  ) async {
    final fake = FakeEngineRepository(
      libraryItems: [
        LibraryItem(
          id: 'item-1',
          kind: LibraryItemKind.single,
          title: '测试影片',
          createdAtMs: 1,
        ),
      ],
    );

    await tester.pumpWidget(
      _scope(
        fake: fake,
        child: _withTopInset(const LibraryScreen()),
      ),
    );
    await tester.pumpAndSettle();

    final card = tester.renderObject<RenderBox>(find.text('测试影片'));
    expect(card.localToGlobal(Offset.zero).dy, greaterThanOrEqualTo(_topInset));
  });

  testWidgets('TasksScreen list starts below top safe area inset', (
    tester,
  ) async {
    final fake = FakeEngineRepository(
      tasks: [
        const DownloadTask(
          id: 'task-1',
          title: '测试任务',
          sourceUrl: 'https://example.com/video',
          status: TaskStatus.completed,
          progressBytes: 100,
          totalBytes: 100,
          createdAtMs: 1,
          updatedAtMs: 1,
        ),
      ],
    );

    await tester.pumpWidget(
      _scope(
        fake: fake,
        child: _withTopInset(const TasksScreen()),
      ),
    );
    await tester.pumpAndSettle();

    final list = tester.renderObject<RenderBox>(
      find.byKey(const Key('tasks_list')),
    );
    expect(list.localToGlobal(Offset.zero).dy, greaterThanOrEqualTo(_topInset));
  });

  testWidgets('BrowseUnavailableScreen message starts below top safe area inset', (
    tester,
  ) async {
    await tester.pumpWidget(
      _withTopInset(
        const BrowseUnavailableScreen(message: '浏览不可用'),
      ),
    );
    await tester.pumpAndSettle();

    final message = tester.renderObject<RenderBox>(find.text('浏览不可用'));
    expect(
      message.localToGlobal(Offset.zero).dy,
      greaterThanOrEqualTo(_topInset),
    );
  });
}
