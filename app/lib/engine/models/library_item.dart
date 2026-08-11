import 'library_item_kind.dart';

class LibraryItem {
  const LibraryItem({
    required this.id,
    required this.kind,
    required this.title,
    this.season,
    this.posterPath,
    required this.createdAtMs,
  });

  final String id;
  final LibraryItemKind kind;
  final String title;
  final int? season;
  final String? posterPath;
  final int createdAtMs;

  factory LibraryItem.fromJson(Map<String, dynamic> json) {
    return LibraryItem(
      id: json['id'] as String,
      kind: LibraryItemKind.fromJson(json['kind'] as String),
      title: json['title'] as String,
      season: json['season'] as int?,
      posterPath: json['poster_path'] as String?,
      createdAtMs: json['created_at_ms'] as int,
    );
  }

  Map<String, dynamic> toJson() => {
        'id': id,
        'kind': kind.toJson(),
        'title': title,
        if (season != null) 'season': season,
        if (posterPath != null) 'poster_path': posterPath,
        'created_at_ms': createdAtMs,
      };

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is LibraryItem &&
          id == other.id &&
          kind == other.kind &&
          title == other.title &&
          season == other.season &&
          posterPath == other.posterPath &&
          createdAtMs == other.createdAtMs;

  @override
  int get hashCode =>
      Object.hash(id, kind, title, season, posterPath, createdAtMs);
}
