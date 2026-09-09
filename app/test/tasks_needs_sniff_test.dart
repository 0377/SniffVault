import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:go_router/go_router.dart';
import 'package:video_sniffing/engine/models/download_task.dart';
import 'package:video_sniffing/engine/models/task_status.dart';
import 'package:video_sniffing/features/tasks/widgets/parent_task_group.dart';
import 'package:video_sniffing/providers/batch_sniff_parent_provider.dart';

DownloadTask _child({
  required String id,
  required TaskStatus status,
  required int episodeIndex,
  String parentId = 'parent-1',
}) {
  return DownloadTask(
    id: id,
    parentId: parentId,
    episodeIndex: episodeIndex,
    title: '第 $episodeIndex 集',
    sourceUrl: 'https://example/ep$episodeIndex',
    status: status,
    progressBytes: 0,
    createdAtMs: 1,
    updatedAtMs: 1,
  );
}

void main() {
  testWidgets('parent task shows needs sniff count and batch sniff button', (
    tester,
  ) async {
    const parent = DownloadTask(
      id: 'parent-1',
      title: '测试剧集',
      sourceUrl: 'https://example/list',
      status: TaskStatus.running,
      progressBytes: 0,
      createdAtMs: 1,
      updatedAtMs: 1,
    );
    final children = [
      _child(id: 'c1', status: TaskStatus.needsSniff, episodeIndex: 1),
      _child(id: 'c2', status: TaskStatus.completed, episodeIndex: 2),
      _child(id: 'c3', status: TaskStatus.needsSniff, episodeIndex: 3),
    ];

    final container = ProviderContainer();
    final router = GoRouter(
      initialLocation: '/tasks',
      routes: [
        GoRoute(
          path: '/tasks',
          builder: (_, _) => Scaffold(
            body: ParentTaskGroup(
              parent: parent,
              children: children,
              onPause: (_) {},
              onResume: (_) {},
              onCancel: (_) {},
            ),
          ),
        ),
        GoRoute(
          path: '/browse',
          builder: (_, _) => const Scaffold(
            key: Key('browse_screen'),
            body: Text('browse'),
          ),
        ),
      ],
    );

    await tester.pumpWidget(
      UncontrolledProviderScope(
        container: container,
        child: MaterialApp.router(routerConfig: router),
      ),
    );
    await tester.pumpAndSettle();

    expect(find.text('3 个子任务，2 待嗅探'), findsOneWidget);
    expect(find.text('嗅探补全'), findsOneWidget);

    await tester.tap(find.byKey(const Key('batch_sniff_button')));
    await tester.pumpAndSettle();

    expect(container.read(batchSniffParentIdProvider), 'parent-1');
    expect(find.byKey(const Key('browse_screen')), findsOneWidget);
    container.dispose();
  });

  testWidgets('parent task hides batch sniff button when no needs sniff children', (
    tester,
  ) async {
    const parent = DownloadTask(
      id: 'parent-2',
      title: '已完成剧集',
      sourceUrl: 'https://example/list',
      status: TaskStatus.completed,
      progressBytes: 0,
      createdAtMs: 1,
      updatedAtMs: 1,
    );
    final children = [
      _child(
        id: 'c1',
        status: TaskStatus.completed,
        episodeIndex: 1,
        parentId: 'parent-2',
      ),
    ];

    await tester.pumpWidget(
      ProviderScope(
        child: MaterialApp(
          home: Scaffold(
            body: ParentTaskGroup(
              parent: parent,
              children: children,
              onPause: (_) {},
              onResume: (_) {},
              onCancel: (_) {},
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    expect(find.text('1 个子任务'), findsOneWidget);
    expect(find.textContaining('待嗅探'), findsNothing);
    expect(find.text('嗅探补全'), findsNothing);
  });
}
