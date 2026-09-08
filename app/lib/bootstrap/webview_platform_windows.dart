import 'package:webview_flutter_platform_interface/webview_flutter_platform_interface.dart';

/// Windows [WebViewPlatform] registration for [browseWebViewAvailable].
///
/// WebView2 environment initialization uses
/// [WebviewController.initializeEnvironment] from `webview_flutter_windows`.
class WebViewPlatformWindows extends WebViewPlatform {}
