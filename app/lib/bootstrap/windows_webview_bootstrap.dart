export 'windows_webview_bootstrap_stub.dart'
    if (dart.library.io) 'windows_webview_bootstrap_io.dart';

import 'package:webview_sniff/webview_sniff.dart';

import 'windows_webview_bootstrap_stub.dart'
    if (dart.library.io) 'windows_webview_bootstrap_io.dart';

Future<bool> bootstrapWindowsWebViewFromProfile(String? userDataPath) async {
  if (userDataPath == null || userDataPath.isEmpty) {
    return false;
  }
  if (!await WebViewSniff.setUserDataFolder(userDataPath)) {
    return false;
  }
  return bootstrapWindowsWebView(userDataPath);
}
