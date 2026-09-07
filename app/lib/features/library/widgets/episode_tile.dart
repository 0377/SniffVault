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
  });

  final LibraryEpisode episode;
  final VoidCallback onTap;

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
      trailing: showResume
          ? const Chip(
              label: Text('续播'),
              visualDensity: VisualDensity.compact,
              materialTapTargetSize: MaterialTapTargetSize.shrinkWrap,
            )
          : null,
      onTap: onTap,
    );
  }
}
