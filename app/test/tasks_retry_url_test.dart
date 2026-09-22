import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:video_sniffing/engine/models/download_task.dart';
import 'package:video_sniffing/engine/models/task_status.dart';
import 'package:video_sniffing/features/tasks/tasks_screen.dart';
import 'package:video_sniffing/providers/engine_host_provider.dart';

import 'fakes/fake_engine_repository.dart';
import 'fakes/fake_ready_engine_host.dart';

const _failedTask = DownloadTask(
  id: 't-failed',
  title: '第01集',
  sourceUrl: 'https://example/x.m3u8',
  status: TaskStatus.failed,
  errorMessage: 'http error',
  progressBytes: 0,
  createdAtMs: 1,
  updatedAtMs: 1,
);

Widget _scope({
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
    child: MaterialApp(home: child),
  );
}

void main() {
  testWidgets('W9d-4b edit url dialog calls retryTask with newUrl', (
    tester,
  ) async {
    final fake = FakeEngineRepository(tasks: [_failedTask]);

    await tester.pumpWidget(_scope(fake: fake, child: const TasksScreen()));
    await tester.pumpAndSettle();

    await tester.tap(find.byKey(const Key('task_edit_url_menu_t-failed')));
    await tester.pumpAndSettle();
    await tester.tap(find.text('修改 URL 重试'));
    await tester.pumpAndSettle();

    await tester.enterText(
      find.byKey(const Key('retry_url_field')),
      'https://new.example/v.mp4',
    );
    await tester.tap(find.byKey(const Key('retry_url_confirm')));
    await tester.pumpAndSettle();

    expect(fake.lastRetryTaskId, 't-failed');
    expect(fake.lastRetryNewUrl, 'https://new.example/v.mp4');
  });

  testWidgets('W9d-5 refresh icon calls retryTask without newUrl', (
    tester,
  ) async {
    final fake = FakeEngineRepository(tasks: [_failedTask]);

    await tester.pumpWidget(_scope(fake: fake, child: const TasksScreen()));
    await tester.pumpAndSettle();

    await tester.tap(find.byTooltip('重试'));
    await tester.pumpAndSettle();

    expect(fake.lastRetryTaskId, 't-failed');
    expect(fake.lastRetryNewUrl, isNull);
  });
}
