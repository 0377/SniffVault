import 'dart:io';

import 'package:video_sniffing/bootstrap/webview_profile.dart';
import 'package:video_sniffing/bootstrap/windows_webview_bootstrap.dart';
import 'package:webview_sniff/webview_sniff.dart';

/// Mirrors Windows bootstrap from [main] before integration test `runApp`.
Future<bool> bootstrapWebViewForIntegrationTest() async {
  var webviewReady = true;
  if (Platform.isWindows) {
    final profile = await webviewUserDataPath();
    if (profile != null) {
      await WebViewSniff.setUserDataFolder(profile);
      webviewReady = await bootstrapWindowsWebViewFromProfile(profile);
    } else {
      webviewReady = false;
    }
  }
  return webviewReady;
}
