import 'package:webview_flutter_platform_interface/webview_flutter_platform_interface.dart';

const _unset = Object();

bool browseWebViewAvailable({Object? platform = _unset}) {
  final WebViewPlatform? resolved = identical(platform, _unset)
      ? WebViewPlatform.instance
      : platform as WebViewPlatform?;
  return resolved != null;
}
