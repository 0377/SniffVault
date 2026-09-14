import 'package:flutter_test/flutter_test.dart';
import 'package:video_sniffing/engine/models/task_event.dart';

void main() {
  test('parses log task event', () {
    final event = TaskEvent.fromJson({
      'kind': 'log',
      'log': {
        'task_id': 't1',
        'message': 'HLS：分片 1/120',
        'at_ms': 1_700_000_000_000,
      },
    });

    expect(event.kind, TaskEventKind.log);
    expect(event.log?.taskId, 't1');
    expect(event.log?.message, 'HLS：分片 1/120');
  });
}
