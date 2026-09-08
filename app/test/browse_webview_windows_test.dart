import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:video_sniffing/bootstrap/windows_webview_bootstrap.dart';
import 'package:video_sniffing/features/browse/browse_webview.dart';
import 'package:webview_flutter/webview_flutter.dart';
import 'package:webview_win_floating/webview_plugin.dart';

void main() {
  test(
    'W10 bootstrap registers platform and WebViewController constructs',
    () async {
      final tempDir = Directory.systemTemp.createTempSync('webview_w10_');
      try {
        final registered = await bootstrapWindowsWebView(tempDir.path);
        expect(registered, isTrue);
        expect(browseWebViewAvailable(), isTrue);

        expect(
          () => WebViewController.fromPlatformCreationParams(
            WindowsWebViewControllerCreationParams(
              userDataFolder: tempDir.path,
            ),
          ),
          returnsNormally,
        );
      } finally {
        tempDir.deleteSync(recursive: true);
      }
    },
    skip: Platform.isWindows ? false : 'Windows only',
  );
}
