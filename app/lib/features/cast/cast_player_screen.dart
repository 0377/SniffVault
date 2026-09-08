import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:media_kit_video/media_kit_video.dart';
import 'package:video_sniffing/cast_receiver/cast_providers.dart';
import 'package:video_sniffing/features/cast/cast_player_controller.dart';
import 'package:video_sniffing/providers/engine_host_provider.dart';

class CastPlayerScreen extends ConsumerStatefulWidget {
  const CastPlayerScreen({super.key, required this.sessionId});

  final String sessionId;

  @override
  ConsumerState<CastPlayerScreen> createState() => _CastPlayerScreenState();
}

class _CastPlayerScreenState extends ConsumerState<CastPlayerScreen> {
  CastStreamPlayerController? _playerController;
  StreamSubscription<Duration>? _durationSubscription;
  var _hasSeekedToStartPosition = false;
  var _stopped = false;

  @override
  void initState() {
    super.initState();
    _init();
  }

  void _init() {
    final request = ref.read(activeCastRequestProvider);
    if (request == null || request.sessionId != widget.sessionId) {
      return;
    }

    final controller = CastStreamPlayerController(request.streamUrl);
    _playerController = controller;

    final startMs = request.metadata.positionMs;
    if (startMs > 0) {
      _durationSubscription = controller.player.stream.duration.listen((duration) {
        if (!_hasSeekedToStartPosition && duration.inMilliseconds > 0) {
          _hasSeekedToStartPosition = true;
          controller.seekTo(Duration(milliseconds: startMs));
          _durationSubscription?.cancel();
          _durationSubscription = null;
        }
      });
    }
  }

  void _stopCastAndPop() {
    if (_stopped) {
      return;
    }
    _stopped = true;
    ref.read(engineRepositoryProvider).stopCast();
    ref.read(activeCastRequestProvider.notifier).state = null;
    if (mounted) {
      context.pop();
    }
  }

  @override
  void dispose() {
    _durationSubscription?.cancel();
    final controller = _playerController;
    if (controller != null) {
      unawaited(controller.dispose());
    }
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final request = ref.watch(activeCastRequestProvider);
    final controller = _playerController;
    if (request == null ||
        request.sessionId != widget.sessionId ||
        controller == null) {
      return Scaffold(
        appBar: AppBar(
          leading: BackButton(onPressed: _stopCastAndPop),
        ),
        body: const Center(child: Text('投送会话不存在或已结束')),
      );
    }

    final metadata = request.metadata;

    return PopScope(
      canPop: false,
      onPopInvokedWithResult: (didPop, result) {
        if (didPop || _stopped) {
          return;
        }
        _stopCastAndPop();
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
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Row(
                    children: [
                      IconButton(
                        key: const Key('cast_player_back'),
                        icon: const Icon(Icons.arrow_back, color: Colors.white),
                        onPressed: _stopCastAndPop,
                      ),
                      Expanded(
                        child: Text(
                          metadata.episodeTitle,
                          style: const TextStyle(color: Colors.white),
                          maxLines: 1,
                          overflow: TextOverflow.ellipsis,
                        ),
                      ),
                    ],
                  ),
                  Padding(
                    padding: const EdgeInsets.symmetric(horizontal: 16),
                    child: Text(
                      metadata.title,
                      style: const TextStyle(color: Colors.white70),
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                    ),
                  ),
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }
}
