import 'dart:io' show Platform;

import 'package:webview_flutter_platform_interface/webview_flutter_platform_interface.dart';
import 'package:webview_sniff/webview_sniff.dart';
import 'package:webview_win_floating/webview_plugin.dart';

/// Windows WebView2 user data folder set by [bootstrapWindowsWebView].
///
/// Task 4 can read this when creating [WindowsWebViewControllerCreationParams].
String? windowsWebViewUserDataPath;

Future<bool> bootstrapWindowsWebView(String userDataPath) async {
  if (!Platform.isWindows) {
    return false;
  }
  try {
    WindowsWebViewPlatform.registerWith();
    windowsWebViewUserDataPath = userDataPath;
    if (WebViewPlatform.instance == null) {
      return false;
    }
    return await WebViewSniff.isWebView2Available();
  } catch (_) {
    return false;
  }
}
