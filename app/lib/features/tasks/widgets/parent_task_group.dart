import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:video_sniffing/engine/models/download_task.dart';
import 'package:video_sniffing/engine/models/task_status.dart';
import 'package:video_sniffing/features/tasks/widgets/task_tile.dart';
import 'package:video_sniffing/providers/batch_sniff_parent_provider.dart';

class ParentTaskGroup extends ConsumerWidget {
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

  int _needsSniffCount() =>
      children.where((child) => child.status == TaskStatus.needsSniff).length;

  void _startBatchSniff(BuildContext context, WidgetRef ref) {
    ref.read(batchSniffParentIdProvider.notifier).state = parent.id;
    context.push('/browse');
  }

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    if (children.isEmpty) {
      return TaskTile(
        task: parent,
        onPause: () => onPause(parent.id),
        onResume: () => onResume(parent.id),
        onCancel: () => onCancel(parent.id),
      );
    }

    final needsSniffCount = _needsSniffCount();
    final subtitle = needsSniffCount > 0
        ? '${children.length} 个子任务，$needsSniffCount 待嗅探'
        : '${children.length} 个子任务';

    return ExpansionTile(
      title: Text(parent.title),
      subtitle: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(subtitle),
          if (needsSniffCount > 0)
            Align(
              alignment: Alignment.centerLeft,
              child: TextButton(
                key: const Key('batch_sniff_button'),
                onPressed: () => _startBatchSniff(context, ref),
                child: const Text('嗅探补全'),
              ),
            ),
        ],
      ),
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
