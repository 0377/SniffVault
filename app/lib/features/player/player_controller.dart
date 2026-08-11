import 'package:media_kit/media_kit.dart';
import 'package:media_kit_video/media_kit_video.dart';

class LocalPlayerController {
  LocalPlayerController(String filePath) : player = Player() {
    controller = VideoController(player);
    player.open(Media(filePath));
  }

  final Player player;
  late final VideoController controller;
  int lastPositionMs = 0;

  Future<void> seekTo(Duration position) => player.seek(position);

  void updateCachedPosition() {
    lastPositionMs = player.state.position.inMilliseconds;
  }

  Future<void> dispose() async {
    updateCachedPosition();
    await player.dispose();
  }
}
