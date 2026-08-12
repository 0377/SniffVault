import 'dart:async';

import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';
import 'package:path_provider/path_provider.dart';
import 'package:video_sniffing/engine/engine_host.dart';
import 'package:video_sniffing/engine/models/download_task.dart';
import 'package:video_sniffing/engine/models/engine_settings.dart';
import 'package:video_sniffing/engine/models/library_item.dart';
import 'package:video_sniffing/engine/models/resolve_types.dart';
import 'package:video_sniffing/engine/models/task_event.dart';

import 'support/app_ui_flow.dart';
import 'support/test_pump.dart';

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();

  testWidgets('F1 settings persist via temporary directory', (tester) async {
    final dataDir = await getTemporaryDirectory();
    final uniqueDir =
        '${dataDir.path}/f1_${DateTime.now().millisecondsSinceEpoch}';

    final host = await EngineHost.open(uniqueDir);
    final custom = EngineSettings(
      mediaDir: 'videos',
      maxConcurrency: 4,
      defaultQualityLabel: '1080p',
      userAgent: 'SmokeTest/1.0',
      deviceName: 'SmokeDevice',
    );
    host.saveSettings(custom);
    host.dispose();

    final host2 = await EngineHost.open(uniqueDir);
    try {
      expect(host2.settings(), custom);
    } finally {
      host2.dispose();
    }
  });

  testWidgets('F2 resolveUrl direct mp4', (tester) async {
    final dataDir = await getTemporaryDirectory();
    final uniqueDir =
        '${dataDir.path}/f2_${DateTime.now().millisecondsSinceEpoch}';
    final host = await EngineHost.open(uniqueDir);
    try {
      final outcome = await pumpUntil(
        tester,
        host.resolveUrl('https://cdn.example/clip.mp4'),
      );
      expect(outcome, isA<ResolveOutcomeSingle>());
      final single = outcome as ResolveOutcomeSingle;
      expect(single.candidate.kind, MediaKind.mp4);
      expect(single.candidate.url, 'https://cdn.example/clip.mp4');
    } finally {
      host.dispose();
    }
  });

  testWidgets('F3 enqueueSingle startDownloads taskEvents', (tester) async {
    final dataDir = await getTemporaryDirectory();
    final uniqueDir =
        '${dataDir.path}/f3_${DateTime.now().millisecondsSinceEpoch}';
    final host = await EngineHost.open(uniqueDir);
    try {
      final taskId = host.enqueueSingle(
        title: 'smoke clip',
        url: 'https://cdn.example/clip.mp4',
      );
      expect(taskId, isNotEmpty);

      final event = await waitForTaskEvent(
        tester,
        host.taskEvents.first,
        beforeWait: () async {
          host.startDownloads();
        },
      );
      expect(event.kind, TaskEventKind.taskUpdated);
      expect(event.task, isNotNull);
      expect(event.task!.id, taskId);
    } finally {
      host.dispose();
    }
  });

  testWidgets('F4 listLibrary and listTasks field checks', (tester) async {
    final dataDir = await getTemporaryDirectory();
    final uniqueDir =
        '${dataDir.path}/f4_${DateTime.now().millisecondsSinceEpoch}';
    final host = await EngineHost.open(uniqueDir);
    try {
      host.enqueueSingle(
        title: 'field check',
        url: 'https://cdn.example/clip.mp4',
      );

      final library = host.listLibrary();
      for (final item in library) {
        expectLibraryItemFields(item);
        final roundtrip = LibraryItem.fromJson(item.toJson());
        expect(roundtrip, item);
      }

      final tasks = host.listTasks();
      expect(tasks, isNotEmpty);
      for (final task in tasks) {
        expectDownloadTaskFields(task);
        final roundtrip = DownloadTask.fromJson(task.toJson());
        expect(roundtrip, task);
      }
    } finally {
      host.dispose();
    }
  });

  testWidgets('F5 dispose then listTasks throws EngineException', (
    tester,
  ) async {
    final dataDir = await getTemporaryDirectory();
    final uniqueDir =
        '${dataDir.path}/f5_${DateTime.now().millisecondsSinceEpoch}';
    final host = await EngineHost.open(uniqueDir);
    host.dispose();

    expect(
      () => host.listTasks(),
      throwsA(isA<EngineException>()),
    );
  });

  testWidgets('U1-U3 app smoke flow', (tester) async {
    await runAppUiSmokeFlow(tester);
  }, timeout: const Timeout(Duration(minutes: 5)));
}

void expectLibraryItemFields(LibraryItem item) {
  expect(item.id, isNotEmpty);
  expect(item.title, isNotEmpty);
  expect(item.createdAtMs, greaterThan(0));
}

void expectDownloadTaskFields(DownloadTask task) {
  expect(task.id, isNotEmpty);
  expect(task.title, isNotEmpty);
  expect(task.sourceUrl, isNotEmpty);
  expect(task.progressBytes, greaterThanOrEqualTo(0));
  expect(task.createdAtMs, greaterThan(0));
  expect(task.updatedAtMs, greaterThanOrEqualTo(task.createdAtMs));
}

Future<TaskEvent> waitForTaskEvent(
  WidgetTester tester,
  Future<TaskEvent> future, {
  Duration timeout = const Duration(seconds: 15),
  Future<void> Function()? beforeWait,
}) async {
  final completer = Completer<TaskEvent>();
  unawaited(
    future.then((event) {
      if (!completer.isCompleted) {
        completer.complete(event);
      }
    }).catchError((Object error, StackTrace stackTrace) {
      if (!completer.isCompleted) {
        completer.completeError(error, stackTrace);
      }
    }),
  );

  if (beforeWait != null) {
    await beforeWait();
  }

  final end = DateTime.now().add(timeout);
  while (!completer.isCompleted && DateTime.now().isBefore(end)) {
    await pumpEngineEvents(tester);
  }

  if (!completer.isCompleted) {
    throw TimeoutException('Timed out waiting for task event', timeout);
  }
  return completer.future;
}

Future<T> pumpUntil<T>(
  WidgetTester tester,
  Future<T> future, {
  Duration timeout = const Duration(seconds: 15),
}) async {
  final done = Completer<void>();
  final result = future.timeout(
    timeout,
    onTimeout: () => throw TimeoutException(
      'Timed out waiting for async engine callback',
      timeout,
    ),
  );
  unawaited(
    result.whenComplete(() {
      if (!done.isCompleted) {
        done.complete();
      }
    }),
  );

  final end = DateTime.now().add(timeout);
  while (!done.isCompleted && DateTime.now().isBefore(end)) {
    await pumpEngineEvents(tester);
  }

  if (!done.isCompleted) {
    throw TimeoutException(
      'Timed out waiting for async engine callback',
      timeout,
    );
  }
  return result;
}
