import 'package:flutter/material.dart';
import 'package:video_sniffing/engine/models/library_episode.dart';

String _basename(String filePath) {
  final segments = filePath.split(RegExp(r'[/\\]'));
  return segments.last;
}

double? _progressValue(LibraryEpisode episode) {
  final duration = episode.durationMs;
  if (duration == null || duration == 0) {
    return null;
  }
  return episode.positionMs / duration;
}

class EpisodeTile extends StatelessWidget {
  const EpisodeTile({
    super.key,
    required this.episode,
    required this.onTap,
    this.onCast,
    this.onRename,
    this.onDelete,
  });

  final LibraryEpisode episode;
  final VoidCallback onTap;
  final VoidCallback? onCast;
  final VoidCallback? onRename;
  final VoidCallback? onDelete;

  @override
  Widget build(BuildContext context) {
    final progress = _progressValue(episode);
    final showResume = episode.positionMs > 0;

    return ListTile(
      title: Text(episode.title),
      subtitle: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(_basename(episode.filePath)),
          if (progress != null) ...[
            const SizedBox(height: 4),
            LinearProgressIndicator(value: progress),
          ],
        ],
      ),
      trailing: _buildTrailing(showResume),
      onTap: onTap,
    );
  }

  Widget? _buildTrailing(bool showResume) {
    if (onCast == null && !showResume && onRename == null && onDelete == null) {
      return null;
    }
    return Row(
      mainAxisSize: MainAxisSize.min,
      children: [
        if (onCast != null)
          IconButton(
            key: Key('cast_episode_${episode.id}'),
            icon: const Icon(Icons.cast),
            tooltip: '投送',
            onPressed: onCast,
          ),
        if (showResume)
          const Chip(
            label: Text('续播'),
            visualDensity: VisualDensity.compact,
            materialTapTargetSize: MaterialTapTargetSize.shrinkWrap,
          ),
        if (onRename != null || onDelete != null)
          PopupMenuButton<String>(
            key: Key('episode_menu_${episode.id}'),
            onSelected: (value) {
              if (value == 'rename') {
                onRename!();
              } else if (value == 'delete') {
                onDelete!();
              }
            },
            itemBuilder: (_) => [
              if (onRename != null)
                const PopupMenuItem(value: 'rename', child: Text('重命名')),
              if (onDelete != null)
                const PopupMenuItem(
                  value: 'delete',
                  child: Text('删除此分集'),
                ),
            ],
          ),
      ],
    );
  }
}
