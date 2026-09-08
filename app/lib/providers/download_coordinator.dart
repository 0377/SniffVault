import 'dart:async';

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:video_sniffing/engine/engine_host.dart';
import 'package:video_sniffing/engine/models/task_event.dart';
import 'package:video_sniffing/engine/models/task_status.dart';
import 'package:video_sniffing/providers/engine_host_provider.dart';
import 'package:video_sniffing/providers/engine_repository.dart';
import 'package:video_sniffing/providers/library_provider.dart';
import 'package:video_sniffing/providers/tasks_provider.dart';

typedef InvalidateCallback = void Function();

class DownloadCoordinator {
  DownloadCoordinator._(
    this._repo, {
    required this._onInvalidateTasks,
    required this._onInvalidateLibrary,
  }) {
    _subscription = _repo.taskEvents.listen(_onEvent);
    _startQueuedDownloadsIfNeeded();
  }

  factory DownloadCoordinator(Ref ref, EngineRepository repo) {
    return DownloadCoordinator._(
      repo,
      onInvalidateTasks: () => ref.invalidate(tasksProvider),
      onInvalidateLibrary: () => ref.invalidate(libraryProvider),
    );
  }

  factory DownloadCoordinator.forTest(
    EngineRepository repo, {
    required InvalidateCallback onInvalidateTasks,
    required InvalidateCallback onInvalidateLibrary,
  }) {
    return DownloadCoordinator._(
      repo,
      onInvalidateTasks: onInvalidateTasks,
      onInvalidateLibrary: onInvalidateLibrary,
    );
  }

  final EngineRepository _repo;
  final InvalidateCallback _onInvalidateTasks;
  final InvalidateCallback _onInvalidateLibrary;
  StreamSubscription<TaskEvent>? _subscription;
  var _workerActive = false;

  void _startQueuedDownloadsIfNeeded() {
    final hasQueued = _repo
        .listTasks()
        .any((task) => task.status == TaskStatus.queued);
    if (hasQueued) {
      ensureDownloads();
    }
  }

  void ensureDownloads() {
    if (_workerActive) {
      return;
    }
    try {
      _repo.startDownloads();
      _workerActive = true;
    } on EngineException catch (e) {
      final msg = e.error.message.toLowerCase();
      if (!msg.contains('downloads already running')) {
        rethrow;
      }
      _workerActive = true;
    }
  }

  void _onEvent(TaskEvent event) {
    switch (event.kind) {
      case TaskEventKind.workerStopped:
        _workerActive = false;
        _onInvalidateTasks();
        final queued = _repo
            .listTasks()
            .any((task) => task.status == TaskStatus.queued);
        if (queued) {
          ensureDownloads();
        }
      case TaskEventKind.taskUpdated:
        _onInvalidateTasks();
        final task = event.task;
        if (task != null && task.status == TaskStatus.completed) {
          _onInvalidateLibrary();
        }
    }
  }

  void dispose() {
    _subscription?.cancel();
  }
}

final downloadCoordinatorProvider = Provider<DownloadCoordinator>((ref) {
  final repo = ref.watch(engineRepositoryProvider);
  final coordinator = DownloadCoordinator(ref, repo);
  ref.onDispose(coordinator.dispose);
  return coordinator;
});
