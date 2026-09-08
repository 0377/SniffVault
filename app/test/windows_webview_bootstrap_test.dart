import 'package:flutter_test/flutter_test.dart';
import 'package:video_sniffing/bootstrap/windows_webview_bootstrap.dart';
import 'package:webview_sniff/webview_sniff.dart';

void main() {
  test('bootstrapWindowsWebViewFromProfile returns false for empty profile', () {
    expect(bootstrapWindowsWebViewFromProfile(null), completion(isFalse));
    expect(bootstrapWindowsWebViewFromProfile(''), completion(isFalse));
  });

  test('setUserDataFolder returns false for empty path', () async {
    expect(await WebViewSniff.setUserDataFolder(''), isFalse);
  });
}
