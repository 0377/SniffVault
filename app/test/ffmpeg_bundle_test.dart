import 'package:flutter/services.dart';
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

  test('ffmpegInstallExceptionFromPlatform prefers native message', () {
    final error = ffmpegInstallExceptionFromPlatform(
      PlatformException(code: 'invalid_arg', message: 'dataDir is required'),
    );
    expect(error.message, 'dataDir is required');
  });

  test('ffmpegInstallExceptionFromPlatform falls back to default message', () {
    final error = ffmpegInstallExceptionFromPlatform(
      PlatformException(code: 'invalid_arg'),
    );
    expect(
      error.message,
      contains('engine/scripts/fetch_ffmpeg_android.sh'),
    );
  });
}
