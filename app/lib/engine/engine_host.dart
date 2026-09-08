import 'dart:async';
import 'dart:convert';
import 'dart:ffi';
import 'dart:isolate';

import 'package:ffi/ffi.dart';

import 'models/cast_types.dart';
import 'models/download_auth.dart';
import 'models/download_task.dart';
import 'models/engine_settings.dart';
import 'models/ffi_response.dart';
import 'models/library_episode.dart';
import 'models/library_item.dart';
import 'models/resolve_types.dart';
import 'models/sniff_types.dart';
import 'models/task_event.dart';
import 'native_bindings.dart';

class EngineException implements Exception {
  EngineException(this.error);

  final FfiError error;

  @override
  String toString() => 'EngineException(${error.kind}): ${error.message}';
}

class EnqueueEpisodesResult {
  const EnqueueEpisodesResult({
    required this.parentId,
    required this.childIds,
  });

  final String parentId;
  final List<String> childIds;
}

class EngineHost {
  EngineHost._(
    this._bindings,
    this._handle,
    this._taskEventsPort,
    this._resolvePort,
    this._castEventsPort,
  ) {
    _taskEventsPort.listen(_onTaskEventMessage);
    _resolvePort.listen(_onResolveMessage);
    _castEventsPort.listen(_onCastEventMessage);
  }

  final NativeBindings _bindings;
  final Pointer<Void> _handle;
  final ReceivePort _taskEventsPort;
  final ReceivePort _resolvePort;
  final ReceivePort _castEventsPort;
  final _taskEventController = StreamController<TaskEvent>.broadcast();
  final _castEventController = StreamController<CastEvent>.broadcast();
  final _pendingResolves = <String, Completer<Map<String, dynamic>>>{};
  var _requestCounter = 0;
  var _disposed = false;

  static Future<EngineHost> open(String dataDir) async {
    final bindings = openNativeLibrary();
    final dataDirPtr = dataDir.toNativeUtf8();
    try {
      final handle = bindings.engineOpen(dataDirPtr);
      if (handle == nullptr) {
        final errPtr = bindings.engineLastError();
        if (errPtr == nullptr) {
          throw EngineException(
            const FfiError(kind: 'message', message: 'engine_open returned null'),
          );
        }
        try {
          final response = _parseFfiResponse(
            errPtr.cast<Utf8>().toDartString(),
            (json) => json,
          );
          throw EngineException(
            response.error ??
                const FfiError(kind: 'message', message: 'engine_open failed'),
          );
        } finally {
          bindings.engineFreeString(errPtr);
        }
      }

      final host = EngineHost._(
        bindings,
        handle,
        ReceivePort(),
        ReceivePort(),
        ReceivePort(),
      );
      host._subscribeTaskEvents();
      return host;
    } finally {
      malloc.free(dataDirPtr);
    }
  }

  Stream<TaskEvent> get taskEvents => _taskEventController.stream;

  Stream<CastEvent> get castEvents => _castEventController.stream;

  void dispose() {
    if (_disposed) {
      return;
    }
    _disposed = true;

    _bindings.engineUnsubscribeTaskEvents(_handle);
    _bindings.engineUnsubscribeCastEvents(_handle);
    _bindings.engineDestroy(_handle);

    for (final completer in _pendingResolves.values) {
      if (!completer.isCompleted) {
        completer.completeError(
          StateError('EngineHost disposed before async resolve completed'),
        );
      }
    }
    _pendingResolves.clear();

    _taskEventsPort.close();
    _resolvePort.close();
    _castEventsPort.close();
    _taskEventController.close();
    _castEventController.close();
  }

  EngineSettings settings() {
    return _callSync(
      (handle) => _bindings.engineSettings(handle),
      (json) => EngineSettings.fromJson(json as Map<String, dynamic>),
    );
  }

  void saveSettings(EngineSettings settings) {
    _callSyncVoid((handle) {
      final jsonPtr = jsonEncode(settings.toJson()).toNativeUtf8();
      try {
        return _bindings.engineSaveSettings(handle, jsonPtr);
      } finally {
        malloc.free(jsonPtr);
      }
    });
  }

  List<LibraryItem> listLibrary() {
    return _callSync(
      (handle) => _bindings.engineListLibrary(handle),
      (json) => (json as List<dynamic>)
          .map((item) => LibraryItem.fromJson(item as Map<String, dynamic>))
          .toList(),
    );
  }

  List<LibraryEpisode> listEpisodes(String itemId) {
    return _withUtf8(itemId, (itemIdPtr) {
      return _callSync(
        (handle) => _bindings.engineListEpisodes(handle, itemIdPtr),
        (json) => (json as List<dynamic>)
            .map(
              (episode) =>
                  LibraryEpisode.fromJson(episode as Map<String, dynamic>),
            )
            .toList(),
      );
    });
  }

  List<DownloadTask> listTasks() {
    return _callSync(
      (handle) => _bindings.engineListTasks(handle),
      (json) => (json as List<dynamic>)
          .map((task) => DownloadTask.fromJson(task as Map<String, dynamic>))
          .toList(),
    );
  }

  String enqueueSingle({
    required String title,
    required String url,
    String? qualityLabel,
    DownloadAuth? auth,
  }) {
    final optsJson = auth == null ? null : jsonEncode(auth.toJson());
    return _withOptionalUtf8(qualityLabel, (qualityLabelPtr) {
      return _withUtf8(title, (titlePtr) {
        return _withUtf8(url, (urlPtr) {
          return _withOptionalUtf8(optsJson, (optsJsonPtr) {
            return _callSync(
              (handle) => _bindings.engineEnqueueSingle(
                handle,
                titlePtr,
                urlPtr,
                qualityLabelPtr ?? nullptr.cast<Utf8>(),
                optsJsonPtr ?? nullptr.cast<Utf8>(),
              ),
              (json) => json as String,
            );
          });
        });
      });
    });
  }

  EnqueueEpisodesResult enqueueEpisodes({
    required String listTitle,
    int? season,
    required List<(int index, String title, String url)> episodes,
    String? qualityLabel,
    DownloadAuth? auth,
  }) {
    final args = <String, dynamic>{
      'list_title': listTitle,
      'season': ?season,
      'episodes': episodes
          .map((episode) => [episode.$1, episode.$2, episode.$3])
          .toList(),
      'quality_label': ?qualityLabel,
      if (auth != null) ...auth.toJson(),
    };
    final argsPtr = jsonEncode(args).toNativeUtf8();
    try {
      final result = _callSync(
        (handle) => _bindings.engineEnqueueEpisodes(handle, argsPtr),
        (json) => json as List<dynamic>,
      );
      return EnqueueEpisodesResult(
        parentId: result[0] as String,
        childIds: (result[1] as List<dynamic>).cast<String>(),
      );
    } finally {
      malloc.free(argsPtr);
    }
  }

  void startDownloads() {
    _callSyncVoid((handle) => _bindings.engineStartDownloads(handle));
  }

  void spawnDownloadWorker() {
    _callSyncVoid((handle) => _bindings.engineSpawnDownloadWorker(handle));
  }

  void stopDownloads() {
    _callSyncVoid((handle) => _bindings.engineStopDownloads(handle));
  }

  void pauseTask(String taskId) {
    _withUtf8(taskId, (taskIdPtr) {
      _callSyncVoid((handle) => _bindings.enginePauseTask(handle, taskIdPtr));
    });
  }

  void resumeTask(String taskId) {
    _withUtf8(taskId, (taskIdPtr) {
      _callSyncVoid((handle) => _bindings.engineResumeTask(handle, taskIdPtr));
    });
  }

  void cancelTask(String taskId) {
    _withUtf8(taskId, (taskIdPtr) {
      _callSyncVoid((handle) => _bindings.engineCancelTask(handle, taskIdPtr));
    });
  }

  void setTaskMediaUrl(String taskId, String mediaUrl) {
    _withUtf8(taskId, (taskIdPtr) {
      _withUtf8(mediaUrl, (mediaUrlPtr) {
        _callSyncVoid(
          (handle) => _bindings.engineSetTaskMediaUrl(
            handle,
            taskIdPtr,
            mediaUrlPtr,
          ),
        );
      });
    });
  }

  void setEpisodePosition(String episodeId, int positionMs) {
    _withUtf8(episodeId, (episodeIdPtr) {
      _callSyncVoid(
        (handle) => _bindings.engineSetEpisodePosition(
          handle,
          episodeIdPtr,
          positionMs,
        ),
      );
    });
  }

  List<ResourceCandidate> sniffUrls(
    List<SniffEvent> events, {
    String? pageUrl,
  }) {
    final eventsPtr = jsonEncode(events.map((e) => e.toJson()).toList())
        .toNativeUtf8();
    return _withOptionalUtf8(pageUrl, (pageUrlPtr) {
      try {
        return _callSync(
          (handle) => _bindings.engineSniffUrls(
            handle,
            eventsPtr,
            pageUrlPtr ?? nullptr.cast<Utf8>(),
          ),
          (json) => (json as List<dynamic>)
              .map(
                (candidate) =>
                    ResourceCandidate.fromJson(candidate as Map<String, dynamic>),
              )
              .toList(),
        );
      } finally {
        malloc.free(eventsPtr);
      }
    });
  }

  Future<ResolveOutcome> resolveUrl(
    String url, {
    ResolveOptions? opts,
  }) {
    return _resolveAsync(
      (requestIdPtr, optsPtr, portId) => _withUtf8(url, (urlPtr) {
        return _bindings.engineResolveUrlAsync(
          _handle,
          urlPtr,
          optsPtr ?? nullptr.cast<Utf8>(),
          portId,
          requestIdPtr,
        );
      }),
      opts,
      (json) => ResolveOutcome.fromJson(json as Map<String, dynamic>),
    );
  }

  Future<List<Quality>> resolveQualities(
    String mediaUrl, {
    ResolveOptions? opts,
  }) {
    return _resolveAsync(
      (requestIdPtr, optsPtr, portId) => _withUtf8(mediaUrl, (mediaUrlPtr) {
        return _bindings.engineResolveQualitiesAsync(
          _handle,
          mediaUrlPtr,
          optsPtr ?? nullptr.cast<Utf8>(),
          portId,
          requestIdPtr,
        );
      }),
      opts,
      (json) => (json as List<dynamic>)
          .map((quality) => Quality.fromJson(quality as Map<String, dynamic>))
          .toList(),
    );
  }

  void applyLanSettings({required bool isReceiver}) {
    _callSyncVoid(
      (handle) => _bindings.engineApplyLanSettings(
        handle,
        isReceiver ? 1 : 0,
      ),
    );
    if (isReceiver) {
      _subscribeCastEvents();
    }
  }

  void stopLan() {
    _callSyncVoid((handle) => _bindings.engineStopLan(handle));
  }

  int? get lanHttpPort {
    return _callSync(
      (handle) => _bindings.engineLanHttpPort(handle),
      (json) => json as int?,
    );
  }

  List<LanPeer> discoverPeers() {
    return _callSync(
      (handle) => _bindings.engineDiscoverPeers(handle),
      (json) => (json as List<dynamic>)
          .map((peer) => LanPeer.fromJson(peer as Map<String, dynamic>))
          .toList(),
    );
  }

  String beginPairing() {
    return _callSync(
      (handle) => _bindings.engineBeginPairing(handle),
      (json) => json as String,
    );
  }

  String? pairingPin() {
    final pin = _callSync(
      (handle) => _bindings.enginePairingPin(handle),
      (json) => json as String,
    );
    return pin.isEmpty ? null : pin;
  }

  void pairPeer({
    required String host,
    required int port,
    required String pin,
  }) {
    final argsPtr = jsonEncode({
      'host': host,
      'port': port,
      'pin': pin,
    }).toNativeUtf8();
    try {
      _callSyncVoid(
        (handle) => _bindings.enginePairPeer(handle, argsPtr),
      );
    } finally {
      malloc.free(argsPtr);
    }
  }

  List<TrustedPeer> listTrustedPeers() {
    return _callSync(
      (handle) => _bindings.engineListTrustedPeers(handle),
      (json) => (json as List<dynamic>)
          .map((peer) => TrustedPeer.fromJson(peer as Map<String, dynamic>))
          .toList(),
    );
  }

  bool removeTrustedPeer(String peerDeviceId) {
    return _withUtf8(peerDeviceId, (peerDeviceIdPtr) {
      return _callSync(
        (handle) => _bindings.engineRemoveTrustedPeer(handle, peerDeviceIdPtr),
        (json) => json as bool,
      );
    });
  }

  void castEpisode({
    required String episodeId,
    required String peerDeviceId,
  }) {
    final argsPtr = jsonEncode({
      'episode_id': episodeId,
      'peer_device_id': peerDeviceId,
    }).toNativeUtf8();
    try {
      _callSyncVoid(
        (handle) => _bindings.engineCastEpisode(handle, argsPtr),
      );
    } finally {
      malloc.free(argsPtr);
    }
  }

  void stopCast() {
    _callSyncVoid((handle) => _bindings.engineStopCast(handle));
  }

  void _subscribeTaskEvents() {
    _callSyncVoid(
      (handle) => _bindings.engineSubscribeTaskEvents(
        handle,
        _taskEventsPort.sendPort.nativePort,
      ),
    );
  }

  void _onTaskEventMessage(dynamic message) {
    if (_disposed) {
      return;
    }
    final json = jsonDecode(message as String) as Map<String, dynamic>;
    _taskEventController.add(TaskEvent.fromJson(json));
  }

  void _onCastEventMessage(dynamic message) {
    if (_disposed) {
      return;
    }
    final json = jsonDecode(message as String) as Map<String, dynamic>;
    _castEventController.add(CastEvent.fromJson(json));
  }

  void _subscribeCastEvents() {
    _callSyncVoid(
      (handle) => _bindings.engineSubscribeCastEvents(
        handle,
        _castEventsPort.sendPort.nativePort,
      ),
    );
  }

  void _onResolveMessage(dynamic message) {
    final json = jsonDecode(message as String) as Map<String, dynamic>;
    final requestId = json['request_id'] as String;
    final completer = _pendingResolves.remove(requestId);
    completer?.complete(json);
  }

  Future<T> _resolveAsync<T>(
    Pointer<Char> Function(Pointer<Utf8> requestIdPtr, Pointer<Utf8>? optsPtr, int portId)
        invoke,
    ResolveOptions? opts,
    T Function(Object? json) parseData,
  ) {
    final requestId = _nextRequestId();
    final completer = Completer<Map<String, dynamic>>();
    _pendingResolves[requestId] = completer;

    final requestIdPtr = requestId.toNativeUtf8();
    Pointer<Utf8>? optsPtr;
    if (opts != null) {
      optsPtr = jsonEncode(opts.toJson()).toNativeUtf8();
    }

    try {
      final responsePtr = invoke(requestIdPtr, optsPtr, _resolvePort.sendPort.nativePort);
      _consumeResponse(
        responsePtr,
        (_) => null,
        allowNullData: true,
      );
    } catch (error, stackTrace) {
      _pendingResolves.remove(requestId);
      if (!completer.isCompleted) {
        completer.completeError(error, stackTrace);
      }
      rethrow;
    } finally {
      malloc.free(requestIdPtr);
      if (optsPtr != null) {
        malloc.free(optsPtr);
      }
    }

    return completer.future.then((json) {
      final ok = json['ok'] as bool;
      if (!ok) {
        final errorJson = json['error'] as Map<String, dynamic>;
        throw EngineException(FfiError.fromJson(errorJson));
      }
      return parseData(json['data']);
    });
  }

  String _nextRequestId() {
    _requestCounter += 1;
    return 'req-$_requestCounter';
  }

  T _callSync<T>(
    Pointer<Char> Function(Pointer<Void> handle) invoke,
    T Function(Object? json) parseData,
  ) {
    _ensureOpen();
    return _consumeResponse(invoke(_handle), parseData);
  }

  void _callSyncVoid(Pointer<Char> Function(Pointer<Void> handle) invoke) {
    _ensureOpen();
    _consumeResponse(
      invoke(_handle),
      (_) => null,
      allowNullData: true,
    );
  }

  T _consumeResponse<T>(
    Pointer<Char> responsePtr,
    T Function(Object? json) parseData, {
    bool allowNullData = false,
  }) {
    if (responsePtr == nullptr) {
      throw EngineException(
        const FfiError(kind: 'message', message: 'FFI call returned null'),
      );
    }

    try {
      final response = _parseFfiResponse(
        responsePtr.cast<Utf8>().toDartString(),
        parseData,
        allowNullData: allowNullData,
      );
      if (!response.ok) {
        throw EngineException(
          response.error ??
              const FfiError(kind: 'message', message: 'unknown engine error'),
        );
      }
      return response.data as T;
    } finally {
      _bindings.engineFreeString(responsePtr);
    }
  }

  static FfiResponse<T> _parseFfiResponse<T>(
    String json,
    T Function(Object? json) parseData, {
    bool allowNullData = false,
  }) {
    final decoded = jsonDecode(json) as Map<String, dynamic>;
    final response = FfiResponse<T>.fromJson(decoded, parseData);
    if (response.ok && response.data == null && !allowNullData) {
      throw EngineException(
        const FfiError(kind: 'message', message: 'FFI response missing data'),
      );
    }
    return response;
  }

  void _ensureOpen() {
    if (_disposed) {
      throw EngineException(
        const FfiError(
          kind: 'message',
          message: 'EngineHost has been disposed',
        ),
      );
    }
  }

  T _withUtf8<T>(String value, T Function(Pointer<Utf8> ptr) action) {
    final ptr = value.toNativeUtf8();
    try {
      return action(ptr);
    } finally {
      malloc.free(ptr);
    }
  }

  T _withOptionalUtf8<T>(
    String? value,
    T Function(Pointer<Utf8>? ptr) action,
  ) {
    if (value == null) {
      return action(null);
    }
    return _withUtf8(value, (ptr) => action(ptr));
  }
}
