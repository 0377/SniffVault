import 'package:video_sniffing/engine/engine_host.dart';
import 'package:video_sniffing/engine/models/download_task.dart';
import 'package:video_sniffing/engine/models/engine_settings.dart';
import 'package:video_sniffing/engine/models/library_episode.dart';
import 'package:video_sniffing/engine/models/library_item.dart';
import 'package:video_sniffing/engine/models/resolve_types.dart';
import 'package:video_sniffing/engine/models/sniff_types.dart';
import 'package:video_sniffing/engine/models/task_event.dart';

abstract class EngineRepository {
  Stream<TaskEvent> get taskEvents;

  EngineSettings settings();
  void saveSettings(EngineSettings settings);
  List<LibraryItem> listLibrary();
  List<LibraryEpisode> listEpisodes(String itemId);
  List<DownloadTask> listTasks();

  String enqueueSingle({
    required String title,
    required String url,
    String? qualityLabel,
  });

  EnqueueEpisodesResult enqueueEpisodes({
    required String listTitle,
    int? season,
    required List<(int index, String title, String url)> episodes,
    String? qualityLabel,
  });

  void startDownloads();
  void pauseTask(String taskId);
  void resumeTask(String taskId);
  void cancelTask(String taskId);
  void setEpisodePosition(String episodeId, int positionMs);

  Future<ResolveOutcome> resolveUrl(String url, {ResolveOptions? opts});
  Future<List<Quality>> resolveQualities(String mediaUrl, {ResolveOptions? opts});
}

class EngineHostRepository implements EngineRepository {
  EngineHostRepository(this._host);

  final EngineHost _host;

  @override
  Stream<TaskEvent> get taskEvents => _host.taskEvents;

  @override
  EngineSettings settings() => _host.settings();

  @override
  void saveSettings(EngineSettings settings) => _host.saveSettings(settings);

  @override
  List<LibraryItem> listLibrary() => _host.listLibrary();

  @override
  List<LibraryEpisode> listEpisodes(String itemId) => _host.listEpisodes(itemId);

  @override
  List<DownloadTask> listTasks() => _host.listTasks();

  @override
  String enqueueSingle({
    required String title,
    required String url,
    String? qualityLabel,
  }) =>
      _host.enqueueSingle(title: title, url: url, qualityLabel: qualityLabel);

  @override
  EnqueueEpisodesResult enqueueEpisodes({
    required String listTitle,
    int? season,
    required List<(int index, String title, String url)> episodes,
    String? qualityLabel,
  }) =>
      _host.enqueueEpisodes(
        listTitle: listTitle,
        season: season,
        episodes: episodes,
        qualityLabel: qualityLabel,
      );

  @override
  void startDownloads() => _host.startDownloads();

  @override
  void pauseTask(String taskId) => _host.pauseTask(taskId);

  @override
  void resumeTask(String taskId) => _host.resumeTask(taskId);

  @override
  void cancelTask(String taskId) => _host.cancelTask(taskId);

  @override
  void setEpisodePosition(String episodeId, int positionMs) =>
      _host.setEpisodePosition(episodeId, positionMs);

  @override
  Future<ResolveOutcome> resolveUrl(String url, {ResolveOptions? opts}) =>
      _host.resolveUrl(url, opts: opts);

  @override
  Future<List<Quality>> resolveQualities(
    String mediaUrl, {
    ResolveOptions? opts,
  }) =>
      _host.resolveQualities(mediaUrl, opts: opts);
}
