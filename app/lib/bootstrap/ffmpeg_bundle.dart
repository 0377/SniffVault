import 'dart:io';

import 'package:flutter/services.dart';

const _channel = MethodChannel('com.videosniffing.video_sniffing/device');

/// ffmpeg 解压/安装失败时抛出，用于启动阶段快速失败。
class FfmpegBundleException implements Exception {
  FfmpegBundleException(this.message);

  final String message;

  @override
  String toString() => message;
}

/// 供单元测试验证原生层返回值解析。
bool ffmpegInstallSucceeded(bool? nativeResult) => nativeResult == true;

/// Android 启动时将 APK assets 中的 ffmpeg 解压到 `{dataDir}/bin/ffmpeg`。
Future<void> ensureAndroidFfmpegInstalled(String dataDir) async {
  if (!Platform.isAndroid) {
    return;
  }
  final ok = await _channel.invokeMethod<bool>('ensureFfmpeg', {
    'dataDir': dataDir,
  });
  if (!ffmpegInstallSucceeded(ok)) {
    throw FfmpegBundleException(
      '无法安装 HLS 所需的 ffmpeg。请重新安装应用；'
      '开发构建请执行 engine/scripts/fetch_ffmpeg_android.sh 后重新编译。',
    );
  }
}
