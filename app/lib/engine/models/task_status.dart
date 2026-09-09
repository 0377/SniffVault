enum TaskStatus {
  queued('queued'),
  running('running'),
  paused('paused'),
  needsSniff('needs_sniff'),
  completed('completed'),
  failed('failed'),
  cancelled('cancelled');

  const TaskStatus(this.jsonValue);

  final String jsonValue;

  static TaskStatus fromJson(String value) {
    return TaskStatus.values.firstWhere(
      (status) => status.jsonValue == value,
      orElse: () => throw ArgumentError('unknown task status: $value'),
    );
  }

  String toJson() => jsonValue;
}
