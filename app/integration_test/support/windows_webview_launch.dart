import 'dart:io';

import 'package:video_sniffing/bootstrap/webview_profile.dart';
import 'package:video_sniffing/bootstrap/windows_webview_bootstrap.dart';

/// Mirrors Windows bootstrap from [main] before integration test `runApp`.
Future<bool> bootstrapWebViewForIntegrationTest() async {
  if (!Platform.isWindows) {
    return true;
  }
  final profile = await webviewUserDataPath();
  return bootstrapWindowsWebViewFromProfile(profile);
}
