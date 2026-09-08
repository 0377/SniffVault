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

  /// Configures the Windows WebView2 user data folder for cookie export.
  ///
  /// Returns `false` when the path is empty or the platform channel fails.
  static Future<bool> setUserDataFolder(String path) async {
    if (path.isEmpty) {
      return false;
    }
    try {
      await _cookies.invokeMethod<void>('setUserDataFolder', path);
      return true;
    } on MissingPluginException {
      return false;
    } on PlatformException {
      return false;
    }
  }

  /// Probes whether WebView2 Runtime can be created (Windows only).
  static Future<bool> isWebView2Available() async {
    try {
      final available = await _cookies.invokeMethod<bool>('isWebView2Available');
      return available ?? false;
    } on MissingPluginException {
      return false;
    } on PlatformException {
      return false;
    }
  }
}
