import 'package:webview_sniff/webview_sniff.dart';

abstract class CookieExporter {
  Future<String?> cookieHeaderFor(Uri page);
}

abstract class BrowseCookieStore {
  Future<void> clearAll();
}

class PluginCookieExporter implements CookieExporter, BrowseCookieStore {
  const PluginCookieExporter();

  @override
  Future<String?> cookieHeaderFor(Uri page) =>
      WebViewSniff.cookieHeaderFor(page);

  @override
  Future<void> clearAll() => WebViewSniff.clearCookies();
}
