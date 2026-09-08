import 'dart:io' show Platform;

import 'package:video_sniffing/bootstrap/webview_platform_windows.dart';
import 'package:webview_flutter_platform_interface/webview_flutter_platform_interface.dart';
import 'package:webview_flutter_windows/webview_flutter_windows.dart';

Future<bool> bootstrapWindowsWebView(String userDataPath) async {
  if (!Platform.isWindows) {
    return false;
  }
  WebViewPlatform.instance = WebViewPlatformWindows();
  try {
    await WebviewController.initializeEnvironment(userDataPath: userDataPath);
    return true;
  } catch (_) {
    return false;
  }
}
