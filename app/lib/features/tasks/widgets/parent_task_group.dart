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
    required this.onRetry,
    required this.onRestore,
    required this.onBatchRetry,
  });

  final DownloadTask parent;
  final List<DownloadTask> children;
  final void Function(String taskId) onPause;
  final void Function(String taskId) onResume;
  final void Function(String taskId) onCancel;
  final void Function(String taskId) onRetry;
  final void Function(String taskId) onRestore;
  final VoidCallback onBatchRetry;

  int _needsSniffCount() =>
      children.where((child) => child.status == TaskStatus.needsSniff).length;

  int _retryableFailedCount() =>
      children.where(taskCanRetry).length;

  String _subtitle(int needsSniffCount, int retryableFailedCount) {
    return parentTaskGroupSubtitle(
      childCount: children.length,
      needsSniffCount: needsSniffCount,
      retryableFailedCount: retryableFailedCount,
    );
  }

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
        onRetry: () => onRetry(parent.id),
        onRestore: () => onRestore(parent.id),
      );
    }

    final needsSniffCount = _needsSniffCount();
    final retryableFailedCount = _retryableFailedCount();
    final subtitle = _subtitle(needsSniffCount, retryableFailedCount);

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
          if (retryableFailedCount > 0)
            Align(
              alignment: Alignment.centerLeft,
              child: TextButton(
                key: const Key('batch_retry_button'),
                onPressed: onBatchRetry,
                child: const Text('重试失败'),
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
              onRetry: () => onRetry(child.id),
              onRestore: () => onRestore(child.id),
            ),
          )
          .toList(),
    );
  }
}

String parentTaskGroupSubtitle({
  required int childCount,
  required int needsSniffCount,
  required int retryableFailedCount,
}) {
  return switch ((needsSniffCount, retryableFailedCount)) {
    (final sniff, final failed) when sniff > 0 && failed > 0 =>
      '$childCount 个子任务，$sniff 待嗅探，$failed 失败',
    (final sniff, _) when sniff > 0 => '$childCount 个子任务，$sniff 待嗅探',
    (_, final failed) when failed > 0 => '$childCount 个子任务，$failed 失败',
    _ => '$childCount 个子任务',
  };
}
