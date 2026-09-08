import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:path_provider_platform_interface/path_provider_platform_interface.dart';
import 'package:plugin_platform_interface/plugin_platform_interface.dart';
import 'package:video_sniffing/bootstrap/webview_profile.dart';

class _FakePathProvider extends Fake
    with MockPlatformInterfaceMixin
    implements PathProviderPlatform {
  @override
  Future<String?> getApplicationSupportPath() async => '/fake/support';
}

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  test('webviewUserDataPath returns null on non-Windows', () async {
    if (Platform.isWindows) {
      return;
    }
    expect(await webviewUserDataPath(), isNull);
  });

  test('webviewUserDataPath returns webview_profile subdir on Windows', () async {
    if (!Platform.isWindows) {
      return;
    }
    PathProviderPlatform.instance = _FakePathProvider();
    final path = await webviewUserDataPath();
    expect(path, '/fake/support/webview_profile');
  });
}
