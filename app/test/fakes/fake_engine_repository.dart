import 'dart:async';

import 'package:video_sniffing/engine/engine_host.dart';
import 'package:video_sniffing/engine/models/cast_types.dart';
import 'package:video_sniffing/engine/models/ffi_response.dart';
import 'package:video_sniffing/engine/models/download_task.dart';
import 'package:video_sniffing/engine/models/engine_settings.dart';
import 'package:video_sniffing/engine/models/library_episode.dart';
import 'package:video_sniffing/engine/models/library_item.dart';
import 'package:video_sniffing/engine/models/download_auth.dart';
import 'package:video_sniffing/engine/models/resolve_types.dart';
import 'package:video_sniffing/engine/models/sniff_types.dart';
import 'package:video_sniffing/engine/models/task_event.dart';
import 'package:video_sniffing/providers/engine_repository.dart';

void validateMediaDirForTest(String name) {
  if (name.isEmpty) {
    throw EngineException(
      const FfiError(
        kind: 'invalid_arg',
        message: 'media_dir must not be empty',
      ),
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
  DownloadAuth? lastEnqueueAuth;
  ResolveOptions? lastResolveOpts;
  ResolveOptions? lastResolveQualitiesOpts;
  List<ResourceCandidate> sniffResults = const [];
  List<LanPeer> discoverPeerResults = const [];
  List<TrustedPeer> trustedPeers = const [];
  String pairingPinValue = '123456';
  bool? lastApplyLanIsReceiver;
  final _events = StreamController<TaskEvent>.broadcast();
  final _castEvents = StreamController<CastEvent>.broadcast();

  @override
  Stream<TaskEvent> get taskEvents => _events.stream;

  @override
  Stream<CastEvent> get castEvents => _castEvents.stream;

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
    DownloadAuth? auth,
  }) {
    lastEnqueueAuth = auth;
    return 'fake-task-id';
  }

  @override
  EnqueueEpisodesResult enqueueEpisodes({
    required String listTitle,
    int? season,
    required List<(int index, String title, String url)> episodes,
    String? qualityLabel,
    DownloadAuth? auth,
  }) {
    lastEnqueueAuth = auth;
    return const EnqueueEpisodesResult(parentId: 'parent', childIds: ['c1']);
  }

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
    lastResolveOpts = opts;
    return ResolveOutcomeSingle(
      ResourceCandidate(id: '1', url: url, kind: MediaKind.mp4),
    );
  }

  @override
  Future<List<Quality>> resolveQualities(
    String mediaUrl, {
    ResolveOptions? opts,
  }) async {
    lastResolveQualitiesOpts = opts;
    return [const Quality(label: '1080p')];
  }

  @override
  List<ResourceCandidate> sniffUrls(
    List<SniffEvent> events, {
    String? pageUrl,
  }) => sniffResults;

  @override
  void applyLanSettings({required bool isReceiver}) {
    lastApplyLanIsReceiver = isReceiver;
  }

  @override
  void stopLan() {}

  @override
  List<LanPeer> discoverPeers() => discoverPeerResults;

  @override
  String beginPairing() => pairingPinValue;

  @override
  String? pairingPin() => pairingPinValue;

  @override
  void pairPeer({
    required String host,
    required int port,
    required String pin,
  }) {}

  @override
  List<TrustedPeer> listTrustedPeers() => trustedPeers;

  @override
  bool removeTrustedPeer(String peerDeviceId) => false;

  @override
  void castEpisode({
    required String episodeId,
    required String peerDeviceId,
  }) {}

  @override
  void stopCast() {}

  void dispose() {
    _events.close();
    _castEvents.close();
  }
}
