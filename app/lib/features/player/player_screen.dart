import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:media_kit_video/media_kit_video.dart';
import 'package:video_sniffing/engine/models/library_episode.dart';
import 'package:video_sniffing/features/player/player_controller.dart';
import 'package:video_sniffing/providers/engine_host_provider.dart';
import 'package:video_sniffing/providers/engine_repository.dart';

class PlayerScreen extends ConsumerStatefulWidget {
  const PlayerScreen({super.key, required this.episodeId});

  final String episodeId;

  @override
  ConsumerState<PlayerScreen> createState() => _PlayerScreenState();
}

class _PlayerScreenState extends ConsumerState<PlayerScreen>
    with WidgetsBindingObserver {
  late final EngineRepository _repo;
  LocalPlayerController? _playerController;
  StreamSubscription<Duration>? _positionSubscription;
  StreamSubscription<Duration>? _durationSubscription;
  Timer? _positionDebounce;
  bool _hasSeekedToSavedPosition = false;
  DateTime? _playbackStartedAt;
  var _positionPersisted = false;

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addObserver(this);
    _repo = ref.read(engineRepositoryProvider);
    _init();
  }

  void _init() {
    final episode = _findEpisode(_repo, widget.episodeId);
    if (episode == null) {
      return;
    }

    final controller = LocalPlayerController(episode.filePath);
    _playerController = controller;
    _playbackStartedAt = DateTime.now();

    _durationSubscription = controller.player.stream.duration.listen((duration) {
      if (!_hasSeekedToSavedPosition && duration.inMilliseconds > 0) {
        _hasSeekedToSavedPosition = true;
        final positionMs = episode.positionMs;
        if (positionMs > 0) {
          controller.seekTo(Duration(milliseconds: positionMs));
        }
        _durationSubscription?.cancel();
        _durationSubscription = null;
      }
    });

    _positionSubscription = controller.player.stream.position.listen((position) {
      controller.lastPositionMs = position.inMilliseconds;
      _positionDebounce?.cancel();
      _positionDebounce = Timer(const Duration(seconds: 5), () {
        controller.lastPositionMs = position.inMilliseconds;
      });
    });
  }

  LibraryEpisode? _findEpisode(EngineRepository repo, String episodeId) {
    for (final item in repo.listLibrary()) {
      for (final episode in repo.listEpisodes(item.id)) {
        if (episode.id == episodeId) {
          return episode;
        }
      }
    }
    return null;
  }

  int _resolvedPositionMs(LocalPlayerController controller) {
    controller.updateCachedPosition();
    var positionMs = controller.lastPositionMs;
    if (positionMs == 0 && _playbackStartedAt != null) {
      final elapsed = DateTime.now().difference(_playbackStartedAt!).inMilliseconds;
      if (elapsed > 0) {
        final durationMs = controller.player.state.duration.inMilliseconds;
        positionMs = durationMs > 0 ? elapsed.clamp(0, durationMs) : elapsed;
      }
    }
    return positionMs;
  }

  void _persistPosition() {
    if (_positionPersisted) {
      return;
    }
    final controller = _playerController;
    if (controller == null) {
      return;
    }
    final positionMs = _resolvedPositionMs(controller);
    controller.lastPositionMs = positionMs;
    _repo.setEpisodePosition(
          widget.episodeId,
          positionMs,
        );
    _positionPersisted = true;
  }

  void _handleBack() {
    _persistPosition();
    if (mounted) {
      Navigator.of(context).pop();
    }
  }

  void _requestBack() {
    _handleBack();
  }

  @override
  void didChangeAppLifecycleState(AppLifecycleState state) {
    if (state == AppLifecycleState.paused) {
      _persistPosition();
    }
  }

  @override
  void dispose() {
    WidgetsBinding.instance.removeObserver(this);
    _positionDebounce?.cancel();
    _positionSubscription?.cancel();
    _durationSubscription?.cancel();

    final controller = _playerController;
    if (controller != null) {
      if (!_positionPersisted) {
        final positionMs = _resolvedPositionMs(controller);
        controller.lastPositionMs = positionMs;
        _repo.setEpisodePosition(
              widget.episodeId,
              positionMs,
            );
      }
      unawaited(controller.dispose());
    }

    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final controller = _playerController;
    if (controller == null) {
      return Scaffold(
        appBar: AppBar(
          leading: BackButton(onPressed: _requestBack),
        ),
        body: const Center(child: Text('找不到分集')),
      );
    }

    return     PopScope(
      canPop: false,
      onPopInvokedWithResult: (didPop, result) {
        if (didPop || _positionPersisted) {
          return;
        }
        _handleBack();
      },
      child: Scaffold(
        backgroundColor: Colors.black,
        body: Stack(
          fit: StackFit.expand,
          children: [
            Video(
              controller: controller.controller,
              controls: MaterialVideoControls,
              fill: Colors.black,
            ),
            SafeArea(
              child: Align(
                alignment: Alignment.topLeft,
                child: IconButton(
                  key: const Key('player_back'),
                  icon: const Icon(Icons.arrow_back, color: Colors.white),
                  onPressed: _handleBack,
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }
}
