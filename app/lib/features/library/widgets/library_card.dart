import 'package:flutter/material.dart';
import 'package:video_sniffing/engine/models/library_item.dart';
import 'package:video_sniffing/engine/models/library_item_kind.dart';
import 'package:video_sniffing/features/library/widgets/poster_thumbnail.dart';

class LibraryCard extends StatelessWidget {
  const LibraryCard({
    super.key,
    required this.item,
    required this.onTap,
  });

  final LibraryItem item;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final seasonLabel = item.kind == LibraryItemKind.series && item.season != null
        ? '第 ${item.season} 季'
        : null;

    return Card(
      margin: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
      child: ListTile(
        leading: PosterThumbnail(
          posterPath: item.posterPath,
          title: item.title,
          width: 80,
          height: 120,
        ),
        title: Text(item.title),
        subtitle: seasonLabel != null ? Text(seasonLabel) : null,
        onTap: onTap,
      ),
    );
  }
}
