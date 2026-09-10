import 'package:flutter_test/flutter_test.dart';
import 'package:video_sniffing/engine/models/download_log_entry.dart';
import 'package:video_sniffing/engine/models/download_task.dart';
import 'package:video_sniffing/engine/models/task_event.dart';
import 'package:video_sniffing/engine/models/task_status.dart';
import 'package:video_sniffing/providers/download_coordinator.dart';

import 'fakes/fake_engine_repository.dart';

class _RecordingRepo extends FakeEngineRepository {
  _RecordingRepo({super.tasks});

  int startCalls = 0;
  @override
  void startDownloads() => startCalls++;
}

void main() {
  test('ensureDownloads calls startDownloads when worker inactive', () {
    final repo = _RecordingRepo();
    final coordinator = DownloadCoordinator.forTest(
      repo,
      onInvalidateTasks: () {},
      onInvalidateLibrary: () {},
    );
    coordinator.ensureDownloads();
    expect(repo.startCalls, 1);
    coordinator.dispose();
  });

  test('appends download log on log task event', () async {
    final repo = _RecordingRepo();
    final logs = <DownloadLogEntry>[];
    final coordinator = DownloadCoordinator.forTest(
      repo,
      onInvalidateTasks: () {},
      onInvalidateLibrary: () {},
      onAppendLog: logs.add,
    );

    repo.emitTaskEvent(
      TaskEvent(
        kind: TaskEventKind.log,
        log: const DownloadLogEntry(
          taskId: 't1',
          message: '开始下载',
          atMs: 1,
        ),
      ),
    );
    await Future<void>.delayed(const Duration(milliseconds: 10));

    expect(logs, hasLength(1));
    expect(logs.first.message, '开始下载');
    coordinator.dispose();
  });

  test('workerStopped does not auto restart downloads', () async {
    final repo = _RecordingRepo(
      tasks: const [
        DownloadTask(
          id: 't1',
          title: 'queued',
          sourceUrl: 'https://example/x.mp4',
          status: TaskStatus.queued,
          progressBytes: 0,
          createdAtMs: 1,
          updatedAtMs: 1,
        ),
      ],
    );
    final coordinator = DownloadCoordinator.forTest(
      repo,
      onInvalidateTasks: () {},
      onInvalidateLibrary: () {},
    );
    coordinator.ensureDownloads();
    expect(repo.startCalls, 1);

    repo.emitTaskEvent(const TaskEvent(kind: TaskEventKind.workerStopped));
    await Future<void>.delayed(const Duration(milliseconds: 10));

    expect(repo.startCalls, 1);
    coordinator.dispose();
  });

  test('starts worker when queued tasks exist on init', () {
    final repo = _RecordingRepo(
      tasks: const [
        DownloadTask(
          id: 't1',
          title: 'queued',
          sourceUrl: 'https://example/x.mp4',
          status: TaskStatus.queued,
          progressBytes: 0,
          createdAtMs: 1,
          updatedAtMs: 1,
        ),
      ],
    );
    final coordinator = DownloadCoordinator.forTest(
      repo,
      onInvalidateTasks: () {},
      onInvalidateLibrary: () {},
    );
    expect(repo.startCalls, 1);
    coordinator.dispose();
  });
}
