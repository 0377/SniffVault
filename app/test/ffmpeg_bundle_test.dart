import 'package:flutter_test/flutter_test.dart';
import 'package:video_sniffing/bootstrap/ffmpeg_bundle.dart';

void main() {
  test('ensureAndroidFfmpegInstalled is a no-op off Android', () async {
    await ensureAndroidFfmpegInstalled('/tmp/data');
  });
}
