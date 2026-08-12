import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:video_sniffing/engine/models/download_task.dart';
import 'package:video_sniffing/engine/models/task_status.dart';
import 'package:video_sniffing/features/tasks/widgets/task_tile.dart';

void main() {
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
