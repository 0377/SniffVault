import 'package:flutter_test/flutter_test.dart';
import 'package:video_sniffing/engine/models/download_task.dart';
import 'package:video_sniffing/engine/models/resolve_types.dart';
import 'package:video_sniffing/engine/models/sniff_types.dart';
import 'package:video_sniffing/engine/models/task_status.dart';
import 'package:video_sniffing/features/browse/cookie_store.dart';
import 'package:video_sniffing/providers/batch_sniff_coordinator.dart';
import 'package:video_sniffing/providers/browse_session.dart';
import 'package:video_sniffing/providers/download_coordinator.dart';

import 'fakes/fake_engine_repository.dart';

class _RecordingRepo extends FakeEngineRepository {
  _RecordingRepo({required super.tasks});

  final setTaskMediaUrlCalls = <(String taskId, String mediaUrl)>[];

  @override
  void setTaskMediaUrl(String taskId, String mediaUrl) {
    setTaskMediaUrlCalls.add((taskId, mediaUrl));
  }
}

class _RecordingDownloadCoordinator {
  _RecordingDownloadCoordinator(this.repo);

  final _RecordingRepo repo;
  int ensureDownloadsCalls = 0;

  DownloadCoordinator asDownloadCoordinator() {
    return DownloadCoordinator.forTest(
      repo,
      onInvalidateTasks: () {},
      onInvalidateLibrary: () {},
    );
  }

  void ensureDownloads() {
    ensureDownloadsCalls++;
    asDownloadCoordinator().ensureDownloads();
  }
}

DownloadTask _needsSniffChild({
  required String id,
  required int episodeIndex,
  required String sourceUrl,
  required String parentId,
}) {
  return DownloadTask(
    id: id,
    parentId: parentId,
    episodeIndex: episodeIndex,
    title: '第 $episodeIndex 集',
    sourceUrl: sourceUrl,
    status: TaskStatus.needsSniff,
    progressBytes: 0,
    createdAtMs: 1,
    updatedAtMs: 1,
  );
}

void main() {
  test('pickSniffCandidate prefers hls then media initiator', () {
    const events = [
      SniffEvent(
        url: 'http://x/a.mp4',
        initiator: SniffInitiator.media,
      ),
      SniffEvent(
        url: 'http://x/b.m3u8',
        initiator: SniffInitiator.subResource,
      ),
      SniffEvent(
        url: 'http://x/c.m3u8',
        initiator: SniffInitiator.media,
      ),
    ];
    const candidates = [
      ResourceCandidate(id: '1', url: 'http://x/a.mp4', kind: MediaKind.mp4),
      ResourceCandidate(id: '2', url: 'http://x/b.m3u8', kind: MediaKind.hls),
      ResourceCandidate(id: '3', url: 'http://x/c.m3u8', kind: MediaKind.hls),
    ];

    final picked = pickSniffCandidate(candidates, events);
    expect(picked?.url, 'http://x/c.m3u8');
  });

  test('W14 batch sniff loads needs_sniff children in episode_index order', () async {
    const parentId = 'parent-1';
    final repo = _RecordingRepo(
      tasks: [
        const DownloadTask(
          id: parentId,
          title: 'series',
          sourceUrl: 'http://x/list',
          status: TaskStatus.running,
          progressBytes: 0,
          createdAtMs: 1,
          updatedAtMs: 1,
        ),
        _needsSniffChild(
          id: 'child-2',
          parentId: parentId,
          episodeIndex: 2,
          sourceUrl: 'http://x/ep2',
        ),
        _needsSniffChild(
          id: 'child-1',
          parentId: parentId,
          episodeIndex: 1,
          sourceUrl: 'http://x/ep1',
        ),
      ],
    );
    final session = BrowseSession(
      repo: repo,
      cookies: FakeCookieExporter(null),
    );
    final downloads = _RecordingDownloadCoordinator(repo);
    final loadedUrls = <String>[];

    repo.sniffResults = const [
      ResourceCandidate(
        id: 'm',
        url: 'http://x/media.m3u8',
        kind: MediaKind.hls,
      ),
    ];

    final coordinator = BatchSniffCoordinator.forTest(
      repo: repo,
      session: session,
      ensureDownloads: downloads.ensureDownloads,
      debounce: Duration.zero,
      pollInterval: Duration.zero,
      episodeTimeout: const Duration(seconds: 1),
    );

    await coordinator.start(
      parentId: parentId,
      loadUrl: (uri) async {
        loadedUrls.add(uri.toString());
        session.onTopLevelNavigation(uri);
        session.onHookEvent(
          const SniffEvent(
            url: 'http://x/media.m3u8',
            initiator: SniffInitiator.media,
          ),
        );
      },
    );

    expect(loadedUrls, ['http://x/ep1', 'http://x/ep2']);
    expect(
      repo.setTaskMediaUrlCalls,
      [
        ('child-1', 'http://x/media.m3u8'),
        ('child-2', 'http://x/media.m3u8'),
      ],
    );
    expect(downloads.ensureDownloadsCalls, 2);
  });

  test('cancel stops remaining episodes', () async {
    const parentId = 'parent-1';
    final repo = _RecordingRepo(
      tasks: [
        _needsSniffChild(
          id: 'child-1',
          parentId: parentId,
          episodeIndex: 1,
          sourceUrl: 'http://x/ep1',
        ),
        _needsSniffChild(
          id: 'child-2',
          parentId: parentId,
          episodeIndex: 2,
          sourceUrl: 'http://x/ep2',
        ),
      ],
    );
    final session = BrowseSession(
      repo: repo,
      cookies: FakeCookieExporter(null),
    );
    final downloads = _RecordingDownloadCoordinator(repo);
    repo.sniffResults = const [
      ResourceCandidate(
        id: 'm',
        url: 'http://x/media.m3u8',
        kind: MediaKind.hls,
      ),
    ];

    final coordinator = BatchSniffCoordinator.forTest(
      repo: repo,
      session: session,
      ensureDownloads: downloads.ensureDownloads,
      debounce: Duration.zero,
      pollInterval: Duration.zero,
      episodeTimeout: const Duration(seconds: 1),
    );

    final run = coordinator.start(
      parentId: parentId,
      loadUrl: (uri) async {
        session.onTopLevelNavigation(uri);
        session.onHookEvent(
          const SniffEvent(
            url: 'http://x/media.m3u8',
            initiator: SniffInitiator.media,
          ),
        );
        if (uri.toString().endsWith('ep2')) {
          coordinator.cancel();
        }
      },
    );

    await run;

    expect(repo.setTaskMediaUrlCalls.length, 1);
    expect(downloads.ensureDownloadsCalls, 1);
  });
}

class FakeCookieExporter implements CookieExporter {
  FakeCookieExporter(this.header);

  final String? header;

  @override
  Future<String?> cookieHeaderFor(Uri page) async => header;
}
