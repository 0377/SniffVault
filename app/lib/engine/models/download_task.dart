import 'task_status.dart';

class DownloadTask {
  const DownloadTask({
    required this.id,
    this.parentId,
    this.season,
    required this.title,
    required this.sourceUrl,
    this.qualityLabel,
    required this.status,
    required this.progressBytes,
    this.totalBytes,
    this.errorMessage,
    this.outputPath,
    this.libraryItemId,
    this.episodeIndex,
    required this.createdAtMs,
    required this.updatedAtMs,
  });

  final String id;
  final String? parentId;
  final int? season;
  final String title;
  final String sourceUrl;
  final String? qualityLabel;
  final TaskStatus status;
  final int progressBytes;
  final int? totalBytes;
  final String? errorMessage;
  final String? outputPath;
  final String? libraryItemId;
  final int? episodeIndex;
  final int createdAtMs;
  final int updatedAtMs;

  factory DownloadTask.fromJson(Map<String, dynamic> json) {
    return DownloadTask(
      id: json['id'] as String,
      parentId: json['parent_id'] as String?,
      season: json['season'] as int?,
      title: json['title'] as String,
      sourceUrl: json['source_url'] as String,
      qualityLabel: json['quality_label'] as String?,
      status: TaskStatus.fromJson(json['status'] as String),
      progressBytes: json['progress_bytes'] as int,
      totalBytes: json['total_bytes'] as int?,
      errorMessage: json['error_message'] as String?,
      outputPath: json['output_path'] as String?,
      libraryItemId: json['library_item_id'] as String?,
      episodeIndex: json['episode_index'] as int?,
      createdAtMs: json['created_at_ms'] as int,
      updatedAtMs: json['updated_at_ms'] as int,
    );
  }

  Map<String, dynamic> toJson() => {
        'id': id,
        if (parentId != null) 'parent_id': parentId,
        if (season != null) 'season': season,
        'title': title,
        'source_url': sourceUrl,
        if (qualityLabel != null) 'quality_label': qualityLabel,
        'status': status.toJson(),
        'progress_bytes': progressBytes,
        if (totalBytes != null) 'total_bytes': totalBytes,
        if (errorMessage != null) 'error_message': errorMessage,
        if (outputPath != null) 'output_path': outputPath,
        if (libraryItemId != null) 'library_item_id': libraryItemId,
        if (episodeIndex != null) 'episode_index': episodeIndex,
        'created_at_ms': createdAtMs,
        'updated_at_ms': updatedAtMs,
      };

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is DownloadTask &&
          id == other.id &&
          parentId == other.parentId &&
          season == other.season &&
          title == other.title &&
          sourceUrl == other.sourceUrl &&
          qualityLabel == other.qualityLabel &&
          status == other.status &&
          progressBytes == other.progressBytes &&
          totalBytes == other.totalBytes &&
          errorMessage == other.errorMessage &&
          outputPath == other.outputPath &&
          libraryItemId == other.libraryItemId &&
          episodeIndex == other.episodeIndex &&
          createdAtMs == other.createdAtMs &&
          updatedAtMs == other.updatedAtMs;

  @override
  int get hashCode => Object.hash(
        id,
        parentId,
        season,
        title,
        sourceUrl,
        qualityLabel,
        status,
        progressBytes,
        totalBytes,
        errorMessage,
        outputPath,
        libraryItemId,
        episodeIndex,
        createdAtMs,
        updatedAtMs,
      );
}
