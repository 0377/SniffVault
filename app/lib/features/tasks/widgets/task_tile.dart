import 'package:flutter/material.dart';
import 'package:video_sniffing/engine/models/download_task.dart';
import 'package:video_sniffing/engine/models/task_error.dart';
import 'package:video_sniffing/engine/models/task_status.dart';

double? taskProgressFraction(DownloadTask task) {
  final total = task.totalBytes;
  if (total == null || total == 0) return null;
  return task.progressBytes / total;
}

String? taskRunningStatusText(DownloadTask task) {
  if (task.status != TaskStatus.running) {
    return null;
  }
  final total = task.totalBytes;
  if (total != null && total > 0) {
    return '下载中 ${task.progressBytes}/$total 分片';
  }
  return '下载中…';
}

String taskStatusLabel(TaskStatus status) {
  return switch (status) {
    TaskStatus.queued => '排队中',
    TaskStatus.running => '下载中',
    TaskStatus.paused => '已暂停',
    TaskStatus.needsSniff => '待嗅探',
    TaskStatus.completed => '已完成',
    TaskStatus.failed => '失败',
    TaskStatus.cancelled => '已取消',
  };
}

bool taskCanRetry(DownloadTask task) {
  return task.status == TaskStatus.failed &&
      task.errorMessage != TaskError.needsSniff;
}

class TaskTile extends StatelessWidget {
  const TaskTile({
    super.key,
    required this.task,
    required this.onPause,
    required this.onResume,
    required this.onCancel,
    required this.onRetry,
  });

  final DownloadTask task;
  final VoidCallback onPause;
  final VoidCallback onResume;
  final VoidCallback onCancel;
  final VoidCallback onRetry;

  @override
  Widget build(BuildContext context) {
    final fraction = taskProgressFraction(task);
    final runningStatus = taskRunningStatusText(task);

    return ListTile(
      title: Text(task.title),
      subtitle: task.status == TaskStatus.running
          ? Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                if (runningStatus != null) Text(runningStatus),
                const SizedBox(height: 4),
                LinearProgressIndicator(value: fraction),
              ],
            )
          : Text(
              task.status == TaskStatus.failed && task.errorMessage != null
                  ? '${taskStatusLabel(task.status)}：${task.errorMessage}'
                  : taskStatusLabel(task.status),
            ),
      trailing: _buildActions(),
    );
  }

  Widget? _buildActions() {
    final actions = <Widget>[];

    if (task.status == TaskStatus.running) {
      actions.add(
        IconButton(
          icon: const Icon(Icons.pause),
          tooltip: '暂停',
          onPressed: onPause,
        ),
      );
    } else if (task.status == TaskStatus.paused) {
      actions.add(
        IconButton(
          icon: const Icon(Icons.play_arrow),
          tooltip: '恢复',
          onPressed: onResume,
        ),
      );
    } else if (taskCanRetry(task)) {
      actions.add(
        IconButton(
          icon: const Icon(Icons.refresh),
          tooltip: '重试',
          onPressed: onRetry,
        ),
      );
    }

    if (task.status == TaskStatus.queued ||
        task.status == TaskStatus.running ||
        task.status == TaskStatus.paused ||
        task.status == TaskStatus.needsSniff) {
      actions.add(
        IconButton(
          icon: const Icon(Icons.close),
          tooltip: '取消',
          onPressed: onCancel,
        ),
      );
    }

    if (actions.isEmpty) {
      return null;
    }

    return Row(
      mainAxisSize: MainAxisSize.min,
      children: actions,
    );
  }
}
