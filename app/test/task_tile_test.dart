import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:video_sniffing/engine/models/download_task.dart';
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
      errorMessage: 'needs_sniff',
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
          ),
        ),
      ),
    );

    expect(find.text('待嗅探'), findsOneWidget);
    expect(find.byIcon(Icons.pause), findsNothing);
    expect(find.byIcon(Icons.play_arrow), findsNothing);
    expect(find.byIcon(Icons.close), findsOneWidget);
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
          ),
        ),
      ),
    );
    expect(find.byType(LinearProgressIndicator), findsOneWidget);
    final indicator = tester.widget<LinearProgressIndicator>(
      find.byType(LinearProgressIndicator),
    );
    expect(indicator.value, isNull);
  });
}
