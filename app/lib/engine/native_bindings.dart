import 'dart:ffi';
import 'dart:io';

import 'package:ffi/ffi.dart';

typedef EngineOpenNative = Pointer<Void> Function(Pointer<Utf8> dataDir);
typedef EngineOpen = Pointer<Void> Function(Pointer<Utf8> dataDir);

typedef EngineLastErrorNative = Pointer<Char> Function();
typedef EngineLastError = Pointer<Char> Function();

typedef EngineDestroyNative = Void Function(Pointer<Void> handle);
typedef EngineDestroy = void Function(Pointer<Void> handle);

typedef EngineFreeStringNative = Void Function(Pointer<Char> s);
typedef EngineFreeString = void Function(Pointer<Char> s);

typedef EngineSettingsFnNative = Pointer<Char> Function(Pointer<Void> handle);
typedef EngineSettingsFn = Pointer<Char> Function(Pointer<Void> handle);

typedef EngineSaveSettingsNative = Pointer<Char> Function(
  Pointer<Void> handle,
  Pointer<Utf8> json,
);
typedef EngineSaveSettings = Pointer<Char> Function(
  Pointer<Void> handle,
  Pointer<Utf8> json,
);

typedef EngineListLibraryNative = Pointer<Char> Function(Pointer<Void> handle);
typedef EngineListLibrary = Pointer<Char> Function(Pointer<Void> handle);

typedef EngineListEpisodesNative = Pointer<Char> Function(
  Pointer<Void> handle,
  Pointer<Utf8> itemId,
);
typedef EngineListEpisodes = Pointer<Char> Function(
  Pointer<Void> handle,
  Pointer<Utf8> itemId,
);

typedef EngineSetEpisodePositionNative = Pointer<Char> Function(
  Pointer<Void> handle,
  Pointer<Utf8> episodeId,
  Int64 positionMs,
);
typedef EngineSetEpisodePosition = Pointer<Char> Function(
  Pointer<Void> handle,
  Pointer<Utf8> episodeId,
  int positionMs,
);

typedef EngineListTasksNative = Pointer<Char> Function(Pointer<Void> handle);
typedef EngineListTasks = Pointer<Char> Function(Pointer<Void> handle);

typedef EngineEnqueueSingleNative = Pointer<Char> Function(
  Pointer<Void> handle,
  Pointer<Utf8> title,
  Pointer<Utf8> url,
  Pointer<Utf8> qualityLabel,
);
typedef EngineEnqueueSingle = Pointer<Char> Function(
  Pointer<Void> handle,
  Pointer<Utf8> title,
  Pointer<Utf8> url,
  Pointer<Utf8> qualityLabel,
);

typedef EngineEnqueueEpisodesNative = Pointer<Char> Function(
  Pointer<Void> handle,
  Pointer<Utf8> argsJson,
);
typedef EngineEnqueueEpisodes = Pointer<Char> Function(
  Pointer<Void> handle,
  Pointer<Utf8> argsJson,
);

typedef EngineStartDownloadsNative = Pointer<Char> Function(Pointer<Void> handle);
typedef EngineStartDownloads = Pointer<Char> Function(Pointer<Void> handle);

typedef EngineStopDownloadsNative = Pointer<Char> Function(Pointer<Void> handle);
typedef EngineStopDownloads = Pointer<Char> Function(Pointer<Void> handle);

typedef EnginePauseTaskNative = Pointer<Char> Function(
  Pointer<Void> handle,
  Pointer<Utf8> taskId,
);
typedef EnginePauseTask = Pointer<Char> Function(
  Pointer<Void> handle,
  Pointer<Utf8> taskId,
);

typedef EngineResumeTaskNative = Pointer<Char> Function(
  Pointer<Void> handle,
  Pointer<Utf8> taskId,
);
typedef EngineResumeTask = Pointer<Char> Function(
  Pointer<Void> handle,
  Pointer<Utf8> taskId,
);

typedef EngineCancelTaskNative = Pointer<Char> Function(
  Pointer<Void> handle,
  Pointer<Utf8> taskId,
);
typedef EngineCancelTask = Pointer<Char> Function(
  Pointer<Void> handle,
  Pointer<Utf8> taskId,
);

typedef EngineSniffUrlsNative = Pointer<Char> Function(
  Pointer<Void> handle,
  Pointer<Utf8> eventsJson,
  Pointer<Utf8> pageUrl,
);
typedef EngineSniffUrls = Pointer<Char> Function(
  Pointer<Void> handle,
  Pointer<Utf8> eventsJson,
  Pointer<Utf8> pageUrl,
);

typedef EngineResolveUrlAsyncNative = Pointer<Char> Function(
  Pointer<Void> handle,
  Pointer<Utf8> url,
  Pointer<Utf8> optsJson,
  Int64 portId,
  Pointer<Utf8> requestId,
);
typedef EngineResolveUrlAsync = Pointer<Char> Function(
  Pointer<Void> handle,
  Pointer<Utf8> url,
  Pointer<Utf8> optsJson,
  int portId,
  Pointer<Utf8> requestId,
);

typedef EngineResolveQualitiesAsyncNative = Pointer<Char> Function(
  Pointer<Void> handle,
  Pointer<Utf8> mediaUrl,
  Pointer<Utf8> optsJson,
  Int64 portId,
  Pointer<Utf8> requestId,
);
typedef EngineResolveQualitiesAsync = Pointer<Char> Function(
  Pointer<Void> handle,
  Pointer<Utf8> mediaUrl,
  Pointer<Utf8> optsJson,
  int portId,
  Pointer<Utf8> requestId,
);

typedef EngineSubscribeTaskEventsNative = Pointer<Char> Function(
  Pointer<Void> handle,
  Int64 portId,
);
typedef EngineSubscribeTaskEvents = Pointer<Char> Function(
  Pointer<Void> handle,
  int portId,
);

typedef EngineUnsubscribeTaskEventsNative = Void Function(Pointer<Void> handle);
typedef EngineUnsubscribeTaskEvents = void Function(Pointer<Void> handle);

class NativeBindings {
  NativeBindings(DynamicLibrary lib)
      : engineOpen =
            lib.lookupFunction<EngineOpenNative, EngineOpen>('engine_open'),
        engineLastError = lib.lookupFunction<EngineLastErrorNative,
            EngineLastError>('engine_last_error'),
        engineDestroy =
            lib.lookupFunction<EngineDestroyNative, EngineDestroy>(
          'engine_destroy',
        ),
        engineFreeString = lib.lookupFunction<EngineFreeStringNative,
            EngineFreeString>('engine_free_string'),
        engineSettings = lib.lookupFunction<EngineSettingsFnNative,
            EngineSettingsFn>('engine_settings'),
        engineSaveSettings = lib.lookupFunction<EngineSaveSettingsNative,
            EngineSaveSettings>('engine_save_settings'),
        engineListLibrary = lib.lookupFunction<EngineListLibraryNative,
            EngineListLibrary>('engine_list_library'),
        engineListEpisodes = lib.lookupFunction<EngineListEpisodesNative,
            EngineListEpisodes>('engine_list_episodes'),
        engineSetEpisodePosition = lib
            .lookupFunction<EngineSetEpisodePositionNative,
                EngineSetEpisodePosition>('engine_set_episode_position'),
        engineListTasks =
            lib.lookupFunction<EngineListTasksNative, EngineListTasks>(
          'engine_list_tasks',
        ),
        engineEnqueueSingle = lib.lookupFunction<EngineEnqueueSingleNative,
            EngineEnqueueSingle>('engine_enqueue_single'),
        engineEnqueueEpisodes = lib
            .lookupFunction<EngineEnqueueEpisodesNative, EngineEnqueueEpisodes>(
          'engine_enqueue_episodes',
        ),
        engineStartDownloads = lib
            .lookupFunction<EngineStartDownloadsNative, EngineStartDownloads>(
          'engine_start_downloads',
        ),
        engineStopDownloads = lib
            .lookupFunction<EngineStopDownloadsNative, EngineStopDownloads>(
          'engine_stop_downloads',
        ),
        enginePauseTask =
            lib.lookupFunction<EnginePauseTaskNative, EnginePauseTask>(
          'engine_pause_task',
        ),
        engineResumeTask =
            lib.lookupFunction<EngineResumeTaskNative, EngineResumeTask>(
          'engine_resume_task',
        ),
        engineCancelTask =
            lib.lookupFunction<EngineCancelTaskNative, EngineCancelTask>(
          'engine_cancel_task',
        ),
        engineSniffUrls =
            lib.lookupFunction<EngineSniffUrlsNative, EngineSniffUrls>(
          'engine_sniff_urls',
        ),
        engineResolveUrlAsync = lib
            .lookupFunction<EngineResolveUrlAsyncNative, EngineResolveUrlAsync>(
          'engine_resolve_url_async',
        ),
        engineResolveQualitiesAsync = lib.lookupFunction<
            EngineResolveQualitiesAsyncNative,
            EngineResolveQualitiesAsync>('engine_resolve_qualities_async'),
        engineSubscribeTaskEvents = lib.lookupFunction<
            EngineSubscribeTaskEventsNative,
            EngineSubscribeTaskEvents>('engine_subscribe_task_events'),
        engineUnsubscribeTaskEvents = lib.lookupFunction<
            EngineUnsubscribeTaskEventsNative,
            EngineUnsubscribeTaskEvents>('engine_unsubscribe_task_events');

  final EngineOpen engineOpen;
  final EngineLastError engineLastError;
  final EngineDestroy engineDestroy;
  final EngineFreeString engineFreeString;
  final EngineSettingsFn engineSettings;
  final EngineSaveSettings engineSaveSettings;
  final EngineListLibrary engineListLibrary;
  final EngineListEpisodes engineListEpisodes;
  final EngineSetEpisodePosition engineSetEpisodePosition;
  final EngineListTasks engineListTasks;
  final EngineEnqueueSingle engineEnqueueSingle;
  final EngineEnqueueEpisodes engineEnqueueEpisodes;
  final EngineStartDownloads engineStartDownloads;
  final EngineStopDownloads engineStopDownloads;
  final EnginePauseTask enginePauseTask;
  final EngineResumeTask engineResumeTask;
  final EngineCancelTask engineCancelTask;
  final EngineSniffUrls engineSniffUrls;
  final EngineResolveUrlAsync engineResolveUrlAsync;
  final EngineResolveQualitiesAsync engineResolveQualitiesAsync;
  final EngineSubscribeTaskEvents engineSubscribeTaskEvents;
  final EngineUnsubscribeTaskEvents engineUnsubscribeTaskEvents;
}

NativeBindings? _bindings;

NativeBindings openNativeLibrary() {
  if (_bindings != null) {
    return _bindings!;
  }

  _bindings = NativeBindings(_loadNativeLibrary());
  return _bindings!;
}

DynamicLibrary _loadNativeLibrary() {
  if (Platform.isMacOS || Platform.isIOS) {
    try {
      return DynamicLibrary.process();
    } on ArgumentError {
      return DynamicLibrary.executable();
    }
  }
  if (Platform.isAndroid || Platform.isLinux) {
    return DynamicLibrary.open('libvideo_sniffing_engine_ffi.so');
  }
  if (Platform.isWindows) {
    return DynamicLibrary.open('video_sniffing_engine_ffi.dll');
  }
  throw UnsupportedError(
    'Unsupported platform for video_sniffing_engine_ffi: ${Platform.operatingSystem}',
  );
}
