import 'package:video_sniffing/engine/engine_host.dart';
import 'package:video_sniffing/engine/models/task_event.dart';

/// Gate-only [EngineHost] for widget tests when [engineRepositoryProvider] is overridden.
class FakeReadyEngineHost implements EngineHost {
  @override
  void dispose() {}

  @override
  Stream<TaskEvent> get taskEvents => const Stream.empty();

  @override
  dynamic noSuchMethod(Invocation invocation) =>
      throw UnimplementedError('${invocation.memberName}');
}
