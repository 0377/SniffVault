import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:video_sniffing/features/tasks/widgets/download_log_panel.dart';
import 'package:video_sniffing/features/tasks/widgets/parent_task_group.dart';
import 'package:video_sniffing/features/tasks/widgets/task_tile.dart';
import 'package:video_sniffing/providers/download_coordinator.dart';
import 'package:video_sniffing/providers/download_logs_provider.dart';
import 'package:video_sniffing/providers/engine_host_provider.dart';
import 'package:video_sniffing/providers/tasks_provider.dart';

class TasksScreen extends ConsumerWidget {
  const TasksScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final tasks = ref.watch(tasksProvider);
    final roots = tasks.where((task) => task.parentId == null).toList();

    if (roots.isEmpty) {
      return Scaffold(
        body: SafeArea(
          child: Center(child: Text('暂无下载任务，去「添加」粘贴 URL')),
        ),
      );
    }

    final repo = ref.read(engineRepositoryProvider);
    final coordinator = ref.read(downloadCoordinatorProvider);
    final logs = ref.watch(downloadLogsProvider);
    final tasksById = {for (final task in tasks) task.id: task};

    return Scaffold(
      body: SafeArea(
        bottom: false,
        child: Column(
          children: [
          Expanded(
            child: ListView.builder(
              key: const Key('tasks_list'),
              itemCount: roots.length,
              itemBuilder: (context, index) {
                final root = roots[index];
                final children =
                    tasks.where((task) => task.parentId == root.id).toList();

                return ParentTaskGroup(
                  parent: root,
                  children: children,
                  onPause: repo.pauseTask,
                  onResume: (taskId) {
                    repo.resumeTask(taskId);
                    coordinator.ensureDownloads();
                  },
                  onCancel: repo.cancelTask,
                  onRetry: (taskId) {
                    repo.retryTask(taskId);
                    ref.invalidate(tasksProvider);
                    coordinator.ensureDownloads();
                  },
                  onRestore: (taskId) {
                    repo.restoreTask(taskId);
                    ref.invalidate(tasksProvider);
                    coordinator.ensureDownloads();
                  },
                  onBatchRetry: () {
                    for (final child in children.where(taskCanRetry)) {
                      repo.retryTask(child.id);
                    }
                    ref.invalidate(tasksProvider);
                    coordinator.ensureDownloads();
                  },
                );
              },
            ),
          ),
          DownloadLogPanel(
            entries: logs,
            tasksById: tasksById,
            onClear: () => ref.read(downloadLogsProvider.notifier).clear(),
          ),
        ],
        ),
      ),
    );
  }
}
