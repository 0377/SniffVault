import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:video_sniffing/engine/models/download_task.dart';
import 'package:video_sniffing/engine/models/resolve_types.dart';
import 'package:video_sniffing/engine/models/sniff_types.dart';
import 'package:video_sniffing/engine/models/task_status.dart';
import 'package:video_sniffing/providers/browse_session.dart';
import 'package:video_sniffing/providers/download_coordinator.dart';
import 'package:video_sniffing/providers/engine_host_provider.dart';
import 'package:video_sniffing/providers/engine_repository.dart';

const batchSniffEpisodeTimeout = Duration(seconds: 15);

ResourceCandidate? pickSniffCandidate(
  List<ResourceCandidate> candidates,
  List<SniffEvent> events,
) {
  if (candidates.isEmpty) {
    return null;
  }

  final hls = candidates.where((c) => c.kind == MediaKind.hls).toList();
  if (hls.isNotEmpty) {
    final mediaUrls = events
        .where((event) => event.initiator == SniffInitiator.media)
        .map((event) => event.url)
        .toSet();
    final preferred = hls.where((c) => mediaUrls.contains(c.url)).toList();
    return (preferred.isNotEmpty ? preferred : hls).first;
  }

  final mp4 = candidates.where((c) => c.kind == MediaKind.mp4).toList();
  if (mp4.isNotEmpty) {
    return mp4.first;
  }

  return candidates.first;
}

typedef BatchSniffLoadUrl = Future<void> Function(Uri url);
typedef BatchSniffEnsureDownloads = void Function();
typedef BatchSniffProgress = void Function(int current, int total);

class BatchSniffCoordinator {
  BatchSniffCoordinator._({
    required EngineRepository repo,
    required BrowseSession session,
    required BatchSniffEnsureDownloads ensureDownloads,
    required Duration debounce,
    required Duration pollInterval,
    required Duration episodeTimeout,
  })  : _repo = repo,
        _session = session,
        _ensureDownloads = ensureDownloads,
        _debounce = debounce,
        _pollInterval = pollInterval,
        _episodeTimeout = episodeTimeout;

  factory BatchSniffCoordinator(Ref ref) {
    return BatchSniffCoordinator._(
      repo: ref.watch(engineRepositoryProvider),
      session: ref.watch(browseSessionProvider),
      ensureDownloads: () =>
          ref.read(downloadCoordinatorProvider).ensureDownloads(),
      debounce: BrowseSession.sniffDebounce,
      pollInterval: const Duration(milliseconds: 200),
      episodeTimeout: batchSniffEpisodeTimeout,
    );
  }

  factory BatchSniffCoordinator.forTest({
    required EngineRepository repo,
    required BrowseSession session,
    required BatchSniffEnsureDownloads ensureDownloads,
    Duration debounce = BrowseSession.sniffDebounce,
    Duration pollInterval = const Duration(milliseconds: 200),
    Duration episodeTimeout = batchSniffEpisodeTimeout,
  }) {
    return BatchSniffCoordinator._(
      repo: repo,
      session: session,
      ensureDownloads: ensureDownloads,
      debounce: debounce,
      pollInterval: pollInterval,
      episodeTimeout: episodeTimeout,
    );
  }

  final EngineRepository _repo;
  final BrowseSession _session;
  final BatchSniffEnsureDownloads _ensureDownloads;
  final Duration _debounce;
  final Duration _pollInterval;
  final Duration _episodeTimeout;

  bool _cancelled = false;
  Future<void>? _runFuture;

  void cancel() {
    _cancelled = true;
  }

  Future<void> start({
    required String parentId,
    required BatchSniffLoadUrl loadUrl,
    VoidCallback? onComplete,
    BatchSniffProgress? onProgress,
  }) {
    if (_runFuture != null) {
      return _runFuture!;
    }
    _runFuture = _run(
      parentId: parentId,
      loadUrl: loadUrl,
      onComplete: onComplete,
      onProgress: onProgress,
    );
    return _runFuture!.whenComplete(() {
      _runFuture = null;
    });
  }

  Future<void> _run({
    required String parentId,
    required BatchSniffLoadUrl loadUrl,
    VoidCallback? onComplete,
    BatchSniffProgress? onProgress,
  }) async {
    _cancelled = false;
    final children = _needsSniffChildren(parentId);
    final total = children.length;
    for (var index = 0; index < children.length; index++) {
      if (_cancelled) {
        break;
      }
      onProgress?.call(index + 1, total);
      await _processEpisode(children[index], loadUrl);
    }
    onComplete?.call();
  }

  List<DownloadTask> _needsSniffChildren(String parentId) {
    final children = _repo
        .listTasks()
        .where(
          (task) =>
              task.parentId == parentId && task.status == TaskStatus.needsSniff,
        )
        .toList();
    children.sort(
      (a, b) => (a.episodeIndex ?? 0).compareTo(b.episodeIndex ?? 0),
    );
    return children;
  }

  Future<void> _processEpisode(
    DownloadTask task,
    BatchSniffLoadUrl loadUrl,
  ) async {
    final uri = Uri.parse(task.sourceUrl);
    await loadUrl(uri);
    if (_cancelled) {
      return;
    }

    final candidate = await _waitForCandidate(pageUrl: uri.toString());
    if (candidate == null || _cancelled) {
      return;
    }

    _repo.setTaskMediaUrl(task.id, candidate.url);
    _ensureDownloads();
  }

  Future<ResourceCandidate?> _waitForCandidate({required String pageUrl}) async {
    final deadline = DateTime.now().add(_episodeTimeout);
    while (DateTime.now().isBefore(deadline) && !_cancelled) {
      await Future<void>.delayed(_debounce);
      if (_cancelled) {
        return null;
      }

      final candidates = _session.candidates.isNotEmpty
          ? _session.candidates
          : _repo.sniffUrls(
              _session.sniffEvents,
              pageUrl: pageUrl,
            );
      final picked = pickSniffCandidate(candidates, _session.sniffEvents);
      if (picked != null) {
        return picked;
      }

      await Future<void>.delayed(_pollInterval);
    }
    return null;
  }
}

final batchSniffCoordinatorProvider = Provider<BatchSniffCoordinator>((ref) {
  return BatchSniffCoordinator(ref);
});
