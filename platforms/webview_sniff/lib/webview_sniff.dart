import 'package:flutter/services.dart';

/// Platform plugin for Android `isTelevision` and Cookie 仓读写.
///
/// Dart callers use `detectIsTelevision` in the app for TV detection.
/// Cookie methods talk to MethodChannel `webview_sniff/cookies`.
class WebViewSniff {
  static const MethodChannel _cookies = MethodChannel('webview_sniff/cookies');

  static Future<String?> cookieHeaderFor(Uri page) async {
    try {
      final header = await _cookies.invokeMethod<String>(
        'cookieHeaderFor',
        page.toString(),
      );
      if (header == null || header.isEmpty) {
        return null;
      }
      return header;
    } on MissingPluginException {
      return null;
    } on PlatformException {
      return null;
    }
  }

  static Future<void> clearCookies() async {
    try {
      await _cookies.invokeMethod<void>('clearCookies');
    } on MissingPluginException {
      return;
    } on PlatformException {
      return;
    }
  }

  static Future<void> setUserDataFolder(String path) async {
    try {
      await _cookies.invokeMethod<void>('setUserDataFolder', path);
    } on MissingPluginException {
      return;
    } on PlatformException {
      return;
    }
  }
}
