import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:video_sniffing/engine/models/download_log_entry.dart';
import 'package:video_sniffing/engine/models/download_task.dart';
import 'package:video_sniffing/engine/models/task_status.dart';
import 'package:video_sniffing/features/tasks/widgets/download_log_panel.dart';

void main() {
  testWidgets('shows download log entries with task title', (tester) async {
    const task = DownloadTask(
      id: 't1',
      title: '第01集',
      sourceUrl: 'https://example/x.m3u8',
      status: TaskStatus.running,
      progressBytes: 1,
      totalBytes: 10,
      createdAtMs: 1,
      updatedAtMs: 1,
    );
    const entry = DownloadLogEntry(
      taskId: 't1',
      message: 'HLS：分片 1/10',
      atMs: 1_758_000_000_000,
    );

    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: DownloadLogPanel(
            entries: [entry],
            tasksById: const {'t1': task},
            onClear: () {},
          ),
        ),
      ),
    );

    expect(find.textContaining('第01集'), findsOneWidget);
    expect(find.textContaining('HLS：分片 1/10'), findsOneWidget);
  });

  testWidgets('shows newest log entry without scrolling', (tester) async {
    const older = DownloadLogEntry(
      taskId: 't1',
      message: '开始下载',
      atMs: 1,
    );
    const newer = DownloadLogEntry(
      taskId: 't1',
      message: 'HLS：分片 2/10',
      atMs: 2,
    );

    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: DownloadLogPanel(
            entries: const [older, newer],
            tasksById: const {},
            onClear: () {},
          ),
        ),
      ),
    );

    expect(find.textContaining('HLS：分片 2/10'), findsOneWidget);
  });
}
