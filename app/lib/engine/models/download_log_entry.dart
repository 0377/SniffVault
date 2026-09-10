class DownloadLogEntry {
  const DownloadLogEntry({
    required this.taskId,
    required this.message,
    required this.atMs,
  });

  final String taskId;
  final String message;
  final int atMs;

  factory DownloadLogEntry.fromJson(Map<String, dynamic> json) {
    return DownloadLogEntry(
      taskId: json['task_id'] as String,
      message: json['message'] as String,
      atMs: json['at_ms'] as int,
    );
  }

  Map<String, dynamic> toJson() => {
        'task_id': taskId,
        'message': message,
        'at_ms': atMs,
      };

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is DownloadLogEntry &&
          taskId == other.taskId &&
          message == other.message &&
          atMs == other.atMs;

  @override
  int get hashCode => Object.hash(taskId, message, atMs);
}
