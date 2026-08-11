import 'download_task.dart';

enum TaskEventKind {
  taskUpdated('task_updated'),
  workerStopped('worker_stopped');

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
  });

  final TaskEventKind kind;
  final DownloadTask? task;

  factory TaskEvent.fromJson(Map<String, dynamic> json) {
    return TaskEvent(
      kind: TaskEventKind.fromJson(json['kind'] as String),
      task: json['task'] == null
          ? null
          : DownloadTask.fromJson(json['task'] as Map<String, dynamic>),
    );
  }

  Map<String, dynamic> toJson() => {
        'kind': kind.toJson(),
        if (task != null) 'task': task!.toJson(),
      };

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is TaskEvent && kind == other.kind && task == other.task;

  @override
  int get hashCode => Object.hash(kind, task);
}
