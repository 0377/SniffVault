import 'package:flutter_test/flutter_test.dart';
import 'package:video_sniffing/bootstrap/webview_platform_windows.dart';
import 'package:video_sniffing/features/browse/browse_webview.dart';
import 'package:webview_flutter_platform_interface/webview_flutter_platform_interface.dart';

void main() {
  test('W10 browseWebViewAvailable true with WebViewPlatformWindows', () {
    WebViewPlatform.instance = WebViewPlatformWindows();
    expect(browseWebViewAvailable(), isTrue);
  });
}
