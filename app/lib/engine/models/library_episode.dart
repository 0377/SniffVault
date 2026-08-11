class LibraryEpisode {
  const LibraryEpisode({
    required this.id,
    required this.itemId,
    required this.index,
    required this.title,
    required this.filePath,
    this.durationMs,
    required this.positionMs,
    this.sourceUrl,
  });

  final String id;
  final String itemId;
  final int index;
  final String title;
  final String filePath;
  final int? durationMs;
  final int positionMs;
  final String? sourceUrl;

  factory LibraryEpisode.fromJson(Map<String, dynamic> json) {
    return LibraryEpisode(
      id: json['id'] as String,
      itemId: json['item_id'] as String,
      index: json['index'] as int,
      title: json['title'] as String,
      filePath: json['file_path'] as String,
      durationMs: json['duration_ms'] as int?,
      positionMs: json['position_ms'] as int,
      sourceUrl: json['source_url'] as String?,
    );
  }

  Map<String, dynamic> toJson() => {
        'id': id,
        'item_id': itemId,
        'index': index,
        'title': title,
        'file_path': filePath,
        if (durationMs != null) 'duration_ms': durationMs,
        'position_ms': positionMs,
        if (sourceUrl != null) 'source_url': sourceUrl,
      };

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is LibraryEpisode &&
          id == other.id &&
          itemId == other.itemId &&
          index == other.index &&
          title == other.title &&
          filePath == other.filePath &&
          durationMs == other.durationMs &&
          positionMs == other.positionMs &&
          sourceUrl == other.sourceUrl;

  @override
  int get hashCode => Object.hash(
        id,
        itemId,
        index,
        title,
        filePath,
        durationMs,
        positionMs,
        sourceUrl,
      );
}
