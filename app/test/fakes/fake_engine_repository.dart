import 'dart:async';

import 'package:video_sniffing/engine/engine_host.dart';
import 'package:video_sniffing/engine/models/ffi_response.dart';
import 'package:video_sniffing/engine/models/download_task.dart';
import 'package:video_sniffing/engine/models/engine_settings.dart';
import 'package:video_sniffing/engine/models/library_episode.dart';
import 'package:video_sniffing/engine/models/library_item.dart';
import 'package:video_sniffing/engine/models/resolve_types.dart';
import 'package:video_sniffing/engine/models/task_event.dart';
import 'package:video_sniffing/providers/engine_repository.dart';

void validateMediaDirForTest(String name) {
  if (name.isEmpty) {
    throw EngineException(
      const FfiError(kind: 'invalid_arg', message: 'media_dir must not be empty'),
    );
  }
  if (name == '.' ||
      name.startsWith('/') ||
      name.contains('..') ||
      name.contains('/') ||
      name.contains('\\')) {
    throw EngineException(
      const FfiError(
        kind: 'invalid_arg',
        message: 'media_dir must be a single relative directory name',
      ),
    );
  }
}

class FakeEngineRepository implements EngineRepository {
  FakeEngineRepository({
    this.settingsValue = EngineSettings.defaults,
    this.libraryItems = const [],
    this.tasks = const [],
  });

  EngineSettings settingsValue;
  List<LibraryItem> libraryItems;
  List<DownloadTask> tasks;
  final _events = StreamController<TaskEvent>.broadcast();

  @override
  Stream<TaskEvent> get taskEvents => _events.stream;

  @override
  EngineSettings settings() => settingsValue;

  @override
  void saveSettings(EngineSettings settings) {
    validateMediaDirForTest(settings.mediaDir);
    settingsValue = settings;
  }

  @override
  List<LibraryItem> listLibrary() => libraryItems;

  @override
  List<LibraryEpisode> listEpisodes(String itemId) => [];

  @override
  List<DownloadTask> listTasks() => tasks;

  @override
  String enqueueSingle({
    required String title,
    required String url,
    String? qualityLabel,
  }) =>
      'fake-task-id';

  @override
  EnqueueEpisodesResult enqueueEpisodes({
    required String listTitle,
    int? season,
    required List<(int index, String title, String url)> episodes,
    String? qualityLabel,
  }) =>
      const EnqueueEpisodesResult(parentId: 'parent', childIds: ['c1']);

  @override
  void startDownloads() {}

  @override
  void pauseTask(String taskId) {}

  @override
  void resumeTask(String taskId) {}

  @override
  void cancelTask(String taskId) {}

  @override
  void setEpisodePosition(String episodeId, int positionMs) {}

  @override
  Future<ResolveOutcome> resolveUrl(String url, {ResolveOptions? opts}) async {
    return ResolveOutcomeSingle(
      ResourceCandidate(
        id: '1',
        url: url,
        kind: MediaKind.mp4,
      ),
    );
  }

  @override
  Future<List<Quality>> resolveQualities(
    String mediaUrl, {
    ResolveOptions? opts,
  }) async =>
      [const Quality(label: '1080p')];

  void dispose() => _events.close();
}
