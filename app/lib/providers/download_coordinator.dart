import 'dart:async';

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:video_sniffing/engine/engine_host.dart';
import 'package:video_sniffing/engine/models/task_event.dart';
import 'package:video_sniffing/engine/models/task_status.dart';
import 'package:video_sniffing/providers/engine_host_provider.dart';
import 'package:video_sniffing/providers/engine_repository.dart';
import 'package:video_sniffing/providers/library_provider.dart';
import 'package:video_sniffing/providers/tasks_provider.dart';

typedef _Invalidate = void Function();

class DownloadCoordinator {
  DownloadCoordinator._(
    this._repo, {
    required _Invalidate onInvalidateTasks,
    required _Invalidate onInvalidateLibrary,
  })  : _onInvalidateTasks = onInvalidateTasks,
        _onInvalidateLibrary = onInvalidateLibrary {
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
    required _Invalidate onInvalidateTasks,
    required _Invalidate onInvalidateLibrary,
  }) {
    return DownloadCoordinator._(
      repo,
      onInvalidateTasks: onInvalidateTasks,
      onInvalidateLibrary: onInvalidateLibrary,
    );
  }

  final EngineRepository _repo;
  final _Invalidate _onInvalidateTasks;
  final _Invalidate _onInvalidateLibrary;
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
