import 'download_log_entry.dart';
import 'download_task.dart';

enum TaskEventKind {
  taskUpdated('task_updated'),
  workerStopped('worker_stopped'),
  log('log');

  const TaskEventKind(this.jsonValue);

  final String jsonValue;

  static TaskEventKind fromJson(String value) {
    return TaskEventKind.values.firstWhere(
      (kind) => kind.jsonValue == value,
      orElse: () => throw ArgumentError('unknown task event kind: $value'),
    );
  }

  String toJson() => jsonValue;
}

class TaskEvent {
  const TaskEvent({
    required this.kind,
    this.task,
    this.log,
  });

  final TaskEventKind kind;
  final DownloadTask? task;
  final DownloadLogEntry? log;

  factory TaskEvent.fromJson(Map<String, dynamic> json) {
    return TaskEvent(
      kind: TaskEventKind.fromJson(json['kind'] as String),
      task: json['task'] == null
          ? null
          : DownloadTask.fromJson(json['task'] as Map<String, dynamic>),
      log: json['log'] == null
          ? null
          : DownloadLogEntry.fromJson(json['log'] as Map<String, dynamic>),
    );
  }

  Map<String, dynamic> toJson() => {
        'kind': kind.toJson(),
        if (task != null) 'task': task!.toJson(),
        if (log != null) 'log': log!.toJson(),
      };

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is TaskEvent &&
          kind == other.kind &&
          task == other.task &&
          log == other.log;

  @override
  int get hashCode => Object.hash(kind, task, log);
}
