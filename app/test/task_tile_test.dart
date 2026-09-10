import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:video_sniffing/engine/models/download_task.dart';
import 'package:video_sniffing/engine/models/task_error.dart';
import 'package:video_sniffing/engine/models/task_status.dart';
import 'package:video_sniffing/features/tasks/widgets/task_tile.dart';

void main() {
  testWidgets('W11 needsSniff shows label without pause or resume', (
    tester,
  ) async {
    const task = DownloadTask(
      id: 't-sniff',
      title: 'ajax-episode',
      sourceUrl: 'https://example/play/1',
      status: TaskStatus.needsSniff,
      errorMessage: TaskError.needsSniff,
      progressBytes: 0,
      createdAtMs: 1,
      updatedAtMs: 1,
    );
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: TaskTile(
            task: task,
            onPause: () {},
            onResume: () {},
            onCancel: () {},
            onRetry: () {},
            onRestore: () {},
          ),
        ),
      ),
    );

    expect(find.text('待嗅探'), findsOneWidget);
    expect(find.byIcon(Icons.pause), findsNothing);
    expect(find.byIcon(Icons.play_arrow), findsNothing);
    expect(find.byIcon(Icons.close), findsOneWidget);
  });

  testWidgets('failed task shows retry button', (tester) async {
    const task = DownloadTask(
      id: 't-failed',
      title: '第01集',
      sourceUrl: 'https://example/x.m3u8',
      status: TaskStatus.failed,
      errorMessage: 'http error',
      progressBytes: 0,
      createdAtMs: 1,
      updatedAtMs: 1,
    );
    var retried = false;
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: TaskTile(
            task: task,
            onPause: () {},
            onResume: () {},
            onCancel: () {},
            onRetry: () => retried = true,
            onRestore: () {},
          ),
        ),
      ),
    );

    expect(find.byTooltip('重试'), findsOneWidget);
    await tester.tap(find.byTooltip('重试'));
    expect(retried, isTrue);
  });

  testWidgets('failed needs_sniff task hides retry button', (tester) async {
    const task = DownloadTask(
      id: 't-sniff-failed',
      title: '第01集',
      sourceUrl: 'https://example/play/1',
      status: TaskStatus.failed,
      errorMessage: TaskError.needsSniff,
      progressBytes: 0,
      createdAtMs: 1,
      updatedAtMs: 1,
    );
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: TaskTile(
            task: task,
            onPause: () {},
            onResume: () {},
            onCancel: () {},
            onRetry: () {},
            onRestore: () {},
          ),
        ),
      ),
    );

    expect(find.byTooltip('重试'), findsNothing);
  });

  testWidgets('cancelled task shows restore button', (tester) async {
    const task = DownloadTask(
      id: 't-cancelled',
      title: '第24集',
      sourceUrl: 'https://example.com/play/24',
      status: TaskStatus.cancelled,
      progressBytes: 0,
      createdAtMs: 1,
      updatedAtMs: 1,
    );
    var restored = false;
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: TaskTile(
            task: task,
            onPause: () {},
            onResume: () {},
            onCancel: () {},
            onRetry: () {},
            onRestore: () => restored = true,
          ),
        ),
      ),
    );

    expect(find.text('已取消'), findsOneWidget);
    expect(find.byTooltip('恢复'), findsOneWidget);
    await tester.tap(find.byTooltip('恢复'));
    expect(restored, isTrue);
  });

  testWidgets('running task shows segment progress when totalBytes is set', (
    tester,
  ) async {
    const task = DownloadTask(
      id: 't-hls',
      title: '第05集',
      sourceUrl: 'https://example/x.m3u8',
      status: TaskStatus.running,
      progressBytes: 3,
      totalBytes: 120,
      createdAtMs: 1,
      updatedAtMs: 1,
    );
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: TaskTile(
            task: task,
            onPause: () {},
            onResume: () {},
            onCancel: () {},
            onRetry: () {},
            onRestore: () {},
          ),
        ),
      ),
    );

    expect(find.text('下载中 3/120 分片'), findsOneWidget);
    final indicator = tester.widget<LinearProgressIndicator>(
      find.byType(LinearProgressIndicator),
    );
    expect(indicator.value, 0.025);
  });

  testWidgets('W2 shows indeterminate progress when totalBytes is null', (
    tester,
  ) async {
    const task = DownloadTask(
      id: 't1',
      title: 'running',
      sourceUrl: 'https://example/x.mp4',
      status: TaskStatus.running,
      progressBytes: 100,
      totalBytes: null,
      createdAtMs: 1,
      updatedAtMs: 1,
    );
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: TaskTile(
            task: task,
            onPause: () {},
            onResume: () {},
            onCancel: () {},
            onRetry: () {},
            onRestore: () {},
          ),
        ),
      ),
    );
    expect(find.text('下载中…'), findsOneWidget);
    expect(find.byType(LinearProgressIndicator), findsOneWidget);
    final indicator = tester.widget<LinearProgressIndicator>(
      find.byType(LinearProgressIndicator),
    );
    expect(indicator.value, isNull);
  });
}
