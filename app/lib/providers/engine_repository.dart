import 'package:video_sniffing/engine/engine_host.dart';
import 'package:video_sniffing/engine/models/cast_types.dart';
import 'package:video_sniffing/engine/models/download_auth.dart';
import 'package:video_sniffing/engine/models/download_task.dart';
import 'package:video_sniffing/engine/models/engine_settings.dart';
import 'package:video_sniffing/engine/models/library_episode.dart';
import 'package:video_sniffing/engine/models/library_item.dart';
import 'package:video_sniffing/engine/models/resolve_types.dart';
import 'package:video_sniffing/engine/models/sniff_types.dart';
import 'package:video_sniffing/engine/models/task_event.dart';

abstract class EngineRepository {
  Stream<TaskEvent> get taskEvents;
  Stream<CastEvent> get castEvents;

  EngineSettings settings();
  void saveSettings(EngineSettings settings);
  List<LibraryItem> listLibrary();
  List<LibraryEpisode> listEpisodes(String itemId);
  void removeLibraryItem(String itemId, {bool deleteFiles});
  void removeEpisode(String episodeId, {bool deleteFiles});
  List<DownloadTask> listTasks();

  String enqueueSingle({
    required String title,
    required String url,
    String? qualityLabel,
    DownloadAuth? auth,
  });

  EnqueueEpisodesResult enqueueEpisodes({
    required String listTitle,
    int? season,
    required List<(int index, String title, String url)> episodes,
    String? qualityLabel,
    DownloadAuth? auth,
  });

  void startDownloads();
  void pauseTask(String taskId);
  void resumeTask(String taskId);
  void cancelTask(String taskId);
  void setTaskMediaUrl(String taskId, String mediaUrl);
  void setEpisodePosition(String episodeId, int positionMs);

  Future<ResolveOutcome> resolveUrl(String url, {ResolveOptions? opts});
  Future<List<Quality>> resolveQualities(String mediaUrl, {ResolveOptions? opts});
  List<ResourceCandidate> sniffUrls(List<SniffEvent> events, {String? pageUrl});

  void applyLanSettings({required bool isReceiver});
  void stopLan();
  List<LanPeer> discoverPeers();
  String beginPairing();
  String? pairingPin();
  void pairPeer({required String host, required int port, required String pin});
  List<TrustedPeer> listTrustedPeers();
  bool removeTrustedPeer(String peerDeviceId);
  void castEpisode({required String episodeId, required String peerDeviceId});
  void stopCast();
}

class EngineHostRepository implements EngineRepository {
  EngineHostRepository(this._host);

  final EngineHost _host;

  @override
  Stream<TaskEvent> get taskEvents => _host.taskEvents;

  @override
  Stream<CastEvent> get castEvents => _host.castEvents;

  @override
  EngineSettings settings() => _host.settings();

  @override
  void saveSettings(EngineSettings settings) => _host.saveSettings(settings);

  @override
  List<LibraryItem> listLibrary() => _host.listLibrary();

  @override
  List<LibraryEpisode> listEpisodes(String itemId) => _host.listEpisodes(itemId);

  @override
  void removeLibraryItem(String itemId, {bool deleteFiles = true}) =>
      _host.removeLibraryItem(itemId, deleteFiles: deleteFiles);

  @override
  void removeEpisode(String episodeId, {bool deleteFiles = true}) =>
      _host.removeEpisode(episodeId, deleteFiles: deleteFiles);

  @override
  List<DownloadTask> listTasks() => _host.listTasks();

  @override
  String enqueueSingle({
    required String title,
    required String url,
    String? qualityLabel,
    DownloadAuth? auth,
  }) =>
      _host.enqueueSingle(
        title: title,
        url: url,
        qualityLabel: qualityLabel,
        auth: auth,
      );

  @override
  EnqueueEpisodesResult enqueueEpisodes({
    required String listTitle,
    int? season,
    required List<(int index, String title, String url)> episodes,
    String? qualityLabel,
    DownloadAuth? auth,
  }) =>
      _host.enqueueEpisodes(
        listTitle: listTitle,
        season: season,
        episodes: episodes,
        qualityLabel: qualityLabel,
        auth: auth,
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
  void setTaskMediaUrl(String taskId, String mediaUrl) =>
      _host.setTaskMediaUrl(taskId, mediaUrl);

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

  @override
  List<ResourceCandidate> sniffUrls(List<SniffEvent> events, {String? pageUrl}) =>
      _host.sniffUrls(events, pageUrl: pageUrl);

  @override
  void applyLanSettings({required bool isReceiver}) =>
      _host.applyLanSettings(isReceiver: isReceiver);

  @override
  void stopLan() => _host.stopLan();

  @override
  List<LanPeer> discoverPeers() => _host.discoverPeers();

  @override
  String beginPairing() => _host.beginPairing();

  @override
  String? pairingPin() => _host.pairingPin();

  @override
  void pairPeer({
    required String host,
    required int port,
    required String pin,
  }) =>
      _host.pairPeer(host: host, port: port, pin: pin);

  @override
  List<TrustedPeer> listTrustedPeers() => _host.listTrustedPeers();

  @override
  bool removeTrustedPeer(String peerDeviceId) =>
      _host.removeTrustedPeer(peerDeviceId);

  @override
  void castEpisode({
    required String episodeId,
    required String peerDeviceId,
  }) =>
      _host.castEpisode(episodeId: episodeId, peerDeviceId: peerDeviceId);

  @override
  void stopCast() => _host.stopCast();
}
