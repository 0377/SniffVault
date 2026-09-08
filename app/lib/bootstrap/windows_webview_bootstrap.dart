import 'windows_webview_bootstrap_stub.dart'
    if (dart.library.io) 'windows_webview_bootstrap_io.dart';

export 'webview_platform_windows.dart';

Future<bool> bootstrapWindowsWebViewFromProfile(String? userDataPath) async {
  if (userDataPath == null || userDataPath.isEmpty) {
    return false;
  }
  return bootstrapWindowsWebView(userDataPath);
}
