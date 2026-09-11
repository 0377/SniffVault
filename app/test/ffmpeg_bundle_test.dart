import 'package:flutter_test/flutter_test.dart';
import 'package:video_sniffing/bootstrap/ffmpeg_bundle.dart';

void main() {
  test('ensureAndroidFfmpegInstalled is a no-op off Android', () async {
    await ensureAndroidFfmpegInstalled('/tmp/data');
  });

  test('ffmpegInstallSucceeded treats only true as success', () {
    expect(ffmpegInstallSucceeded(true), isTrue);
    expect(ffmpegInstallSucceeded(false), isFalse);
    expect(ffmpegInstallSucceeded(null), isFalse);
  });

  test('FfmpegBundleException exposes message', () {
    final error = FfmpegBundleException('ffmpeg missing');
    expect(error.toString(), 'ffmpeg missing');
  });
}
