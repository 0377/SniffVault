import 'package:flutter/material.dart';
import 'package:video_sniffing/engine/models/download_log_entry.dart';
import 'package:video_sniffing/engine/models/download_task.dart';

String formatLogTime(int atMs) {
  final dt = DateTime.fromMillisecondsSinceEpoch(atMs);
  final h = dt.hour.toString().padLeft(2, '0');
  final m = dt.minute.toString().padLeft(2, '0');
  final s = dt.second.toString().padLeft(2, '0');
  return '$h:$m:$s';
}

String logLineLabel(
  DownloadLogEntry entry,
  Map<String, DownloadTask> tasksById,
) {
  final task = tasksById[entry.taskId];
  final title = task?.title ?? entry.taskId;
  return '[${formatLogTime(entry.atMs)}] $title：${entry.message}';
}

class DownloadLogPanel extends StatelessWidget {
  const DownloadLogPanel({
    super.key,
    required this.entries,
    required this.tasksById,
    required this.onClear,
  });

  final List<DownloadLogEntry> entries;
  final Map<String, DownloadTask> tasksById;
  final VoidCallback onClear;

  @override
  Widget build(BuildContext context) {
    if (entries.isEmpty) {
      return const SizedBox.shrink();
    }

    final theme = Theme.of(context);
    return Material(
      elevation: 2,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
            child: Row(
              children: [
                Text(
                  '下载日志',
                  style: theme.textTheme.titleSmall,
                ),
                const Spacer(),
                TextButton(
                  key: const Key('download_log_clear'),
                  onPressed: onClear,
                  child: const Text('清空'),
                ),
              ],
            ),
          ),
          const Divider(height: 1),
          SizedBox(
            height: 160,
            child: ListView.builder(
              key: const Key('download_log_list'),
              padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
              itemCount: entries.length,
              itemBuilder: (context, index) {
                final entry = entries[index];
                return Text(
                  logLineLabel(entry, tasksById),
                  style: theme.textTheme.bodySmall?.copyWith(
                    fontFamily: 'monospace',
                  ),
                );
              },
            ),
          ),
        ],
      ),
    );
  }
}
