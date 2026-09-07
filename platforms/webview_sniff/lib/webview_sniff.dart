import 'package:flutter/services.dart';

/// Android host for MethodChannel `webview_sniff/device`.
///
/// Dart callers use `detectIsTelevision` in the app; this library exists so
/// Flutter can register the Android plugin.
///
/// Cookie export is a stub until the native plugin lands.
class WebViewSniff {
  static const MethodChannel _cookies = MethodChannel('webview_sniff/cookies');

  static Future<String?> cookieHeaderFor(Uri page) async {
    try {
      return await _cookies.invokeMethod<String>(
        'cookieHeaderFor',
        page.toString(),
      );
    } on MissingPluginException {
      return null;
    }
  }
}
