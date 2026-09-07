import 'package:flutter/material.dart';
import 'package:video_sniffing/engine/models/download_task.dart';
import 'package:video_sniffing/features/tasks/widgets/task_tile.dart';

class ParentTaskGroup extends StatelessWidget {
  const ParentTaskGroup({
    super.key,
    required this.parent,
    required this.children,
    required this.onPause,
    required this.onResume,
    required this.onCancel,
  });

  final DownloadTask parent;
  final List<DownloadTask> children;
  final void Function(String taskId) onPause;
  final void Function(String taskId) onResume;
  final void Function(String taskId) onCancel;

  @override
  Widget build(BuildContext context) {
    if (children.isEmpty) {
      return TaskTile(
        task: parent,
        onPause: () => onPause(parent.id),
        onResume: () => onResume(parent.id),
        onCancel: () => onCancel(parent.id),
      );
    }

    return ExpansionTile(
      title: Text(parent.title),
      subtitle: Text('${children.length} 个子任务'),
      children: children
          .map(
            (child) => TaskTile(
              task: child,
              onPause: () => onPause(child.id),
              onResume: () => onResume(child.id),
              onCancel: () => onCancel(child.id),
            ),
          )
          .toList(),
    );
  }
}
