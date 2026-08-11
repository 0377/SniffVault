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
  LocalPlayerController? _playerController;
  StreamSubscription<Duration>? _positionSubscription;
  StreamSubscription<Duration>? _durationSubscription;
  Timer? _positionDebounce;
  bool _hasSeekedToSavedPosition = false;

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addObserver(this);
    _init();
  }

  void _init() {
    final repo = ref.read(engineRepositoryProvider);
    final episode = _findEpisode(repo, widget.episodeId);
    if (episode == null) {
      return;
    }

    final controller = LocalPlayerController(episode.filePath);
    _playerController = controller;

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

  Future<void> _persistPosition() async {
    final controller = _playerController;
    if (controller == null) {
      return;
    }
    controller.updateCachedPosition();
    ref.read(engineRepositoryProvider).setEpisodePosition(
          widget.episodeId,
          controller.lastPositionMs,
        );
  }

  Future<void> _handleBack() async {
    await _persistPosition();
    if (mounted) {
      context.pop();
    }
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
      controller.updateCachedPosition();
      ref.read(engineRepositoryProvider).setEpisodePosition(
            widget.episodeId,
            controller.lastPositionMs,
          );
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
          leading: BackButton(onPressed: _handleBack),
        ),
        body: const Center(child: Text('找不到分集')),
      );
    }

    return PopScope(
      canPop: false,
      onPopInvokedWithResult: (didPop, result) {
        if (!didPop) {
          unawaited(_handleBack());
        }
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
