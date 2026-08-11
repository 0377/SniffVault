import 'package:flutter_test/flutter_test.dart';
import 'fakes/fake_engine_repository.dart';
import 'package:video_sniffing/providers/download_coordinator.dart';

class _RecordingRepo extends FakeEngineRepository {
  int startCalls = 0;
  @override
  void startDownloads() => startCalls++;
}

void main() {
  test('ensureDownloads calls startDownloads when worker inactive', () {
    final repo = _RecordingRepo();
    final coordinator = DownloadCoordinator.forTest(
      repo,
      onInvalidateTasks: () {},
      onInvalidateLibrary: () {},
    );
    coordinator.ensureDownloads();
    expect(repo.startCalls, 1);
    coordinator.dispose();
  });
}
