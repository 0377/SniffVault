import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:video_sniffing/engine/models/download_task.dart';
import 'package:video_sniffing/engine/models/task_status.dart';
import 'package:video_sniffing/providers/engine_host_provider.dart';
import 'package:video_sniffing/providers/engine_repository.dart';
import 'package:video_sniffing/providers/tasks_provider.dart';

import 'fakes/fake_engine_repository.dart';
import 'fakes/fake_ready_engine_host.dart';

class _UpdatingRepo extends FakeEngineRepository {
  _UpdatingRepo({required super.tasks});

  @override
  void setTaskMediaUrl(String taskId, String mediaUrl) {
    tasks = tasks
        .map(
          (task) => task.id == taskId
              ? DownloadTask(
                  id: task.id,
                  parentId: task.parentId,
                  season: task.season,
                  title: task.title,
                  sourceUrl: task.sourceUrl,
                  qualityLabel: task.qualityLabel,
                  status: TaskStatus.queued,
                  progressBytes: task.progressBytes,
                  totalBytes: task.totalBytes,
                  errorMessage: null,
                  outputPath: task.outputPath,
                  libraryItemId: task.libraryItemId,
                  episodeIndex: task.episodeIndex,
                  createdAtMs: task.createdAtMs,
                  updatedAtMs: task.updatedAtMs,
                  resolvedMediaUrl: mediaUrl,
                )
              : task,
        )
        .toList();
  }
}

DownloadTask _needsSniffChild(String id, String parentId, int episodeIndex) {
  return DownloadTask(
    id: id,
    parentId: parentId,
    episodeIndex: episodeIndex,
    title: '第 $episodeIndex 集',
    sourceUrl: 'https://example/ep$episodeIndex',
    status: TaskStatus.needsSniff,
    progressBytes: 0,
    createdAtMs: 1,
    updatedAtMs: 1,
  );
}

int needsSniffCount(ProviderContainer container) {
  return container
      .read(tasksProvider)
      .where((task) => task.status == TaskStatus.needsSniff)
      .length;
}

void main() {
  test('setTaskMediaUrl updates repo but tasksProvider stays stale until invalidated', () {
    const parentId = 'parent-1';
    final repo = _UpdatingRepo(
      tasks: [
        const DownloadTask(
          id: parentId,
          title: 'series',
          sourceUrl: 'https://example/list',
          status: TaskStatus.running,
          progressBytes: 0,
          createdAtMs: 1,
          updatedAtMs: 1,
        ),
        _needsSniffChild('c1', parentId, 1),
        _needsSniffChild('c2', parentId, 2),
        _needsSniffChild('c3', parentId, 3),
      ],
    );
    final container = ProviderContainer(
      overrides: [
        engineHostProvider.overrideWith((ref) async => FakeReadyEngineHost()),
        engineRepositoryProvider.overrideWithValue(repo),
      ],
    );

    expect(needsSniffCount(container), 3);

    repo.setTaskMediaUrl('c1', 'https://cdn.example.com/ep1.m3u8');
    expect(
      repo.listTasks().where((t) => t.status == TaskStatus.needsSniff).length,
      2,
    );
    expect(needsSniffCount(container), 3);

    container.invalidate(tasksProvider);
    expect(needsSniffCount(container), 2);

    container.dispose();
  });
}
