import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:video_sniffing/features/tasks/widgets/parent_task_group.dart';
import 'package:video_sniffing/providers/download_coordinator.dart';
import 'package:video_sniffing/providers/engine_host_provider.dart';
import 'package:video_sniffing/providers/tasks_provider.dart';

class TasksScreen extends ConsumerWidget {
  const TasksScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final tasks = ref.watch(tasksProvider);
    final roots = tasks.where((task) => task.parentId == null).toList();

    if (roots.isEmpty) {
      return const Scaffold(
        body: Center(child: Text('暂无下载任务，去「添加」粘贴 URL')),
      );
    }

    final repo = ref.read(engineRepositoryProvider);
    final coordinator = ref.read(downloadCoordinatorProvider);

    return Scaffold(
      body: ListView.builder(
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
          );
        },
      ),
    );
  }
}
