import 'package:webview_sniff/webview_sniff.dart';

abstract class CookieExporter {
  Future<String?> cookieHeaderFor(Uri page);
}

class PluginCookieExporter implements CookieExporter {
  const PluginCookieExporter();

  @override
  Future<String?> cookieHeaderFor(Uri page) => WebViewSniff.cookieHeaderFor(page);
}
