import 'dart:async';

import 'package:media_kit/media_kit.dart';
import 'package:media_kit_video/media_kit_video.dart';

class CastStreamPlayerController {
  CastStreamPlayerController(String streamUrl) : player = Player() {
    controller = VideoController(player);
    unawaited(player.open(Media(streamUrl), play: true));
  }

  final Player player;
  late final VideoController controller;

  Future<void> seekTo(Duration position) => player.seek(position);

  Future<void> dispose() async {
    await player.dispose();
  }
}
