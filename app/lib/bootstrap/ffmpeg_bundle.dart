import 'dart:io';

import 'package:flutter/services.dart';

const _channel = MethodChannel('com.videosniffing.video_sniffing/device');

/// Android 启动时将 APK assets 中的 ffmpeg 解压到 `{dataDir}/bin/ffmpeg`。
Future<void> ensureAndroidFfmpegInstalled(String dataDir) async {
  if (!Platform.isAndroid) {
    return;
  }
  await _channel.invokeMethod<void>('ensureFfmpeg', {'dataDir': dataDir});
}
