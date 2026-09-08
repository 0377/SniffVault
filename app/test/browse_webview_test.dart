import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:go_router/go_router.dart';
import 'package:video_sniffing/features/browse/browse_screen.dart';
import 'package:video_sniffing/features/browse/browse_webview.dart';
import 'package:video_sniffing/providers/device_profile.dart';
import 'package:webview_flutter_platform_interface/webview_flutter_platform_interface.dart';

import 'fakes/fake_webview_platform.dart';

void main() {
  test('browseWebViewAvailable is true when a platform is registered', () {
    expect(WebViewPlatform.instance, isA<FakeWebViewPlatform>());
    expect(browseWebViewAvailable(), isTrue);
  });

  test('browseWebViewAvailable is false when the platform is missing', () {
    expect(browseWebViewAvailable(platform: null), isFalse);
  });

  testWidgets('missing WebView platform shows unavailable copy without crash', (
    tester,
  ) async {
    await tester.pumpWidget(
      ProviderScope(
        overrides: [isTelevisionProvider.overrideWith((ref) async => false)],
        child: MaterialApp.router(
          routerConfig: GoRouter(
            initialLocation: '/browse',
            routes: [
              GoRoute(
                path: '/browse',
                builder: (_, _) => const BrowseScreen(webViewAvailable: false),
              ),
            ],
          ),
        ),
      ),
    );
    await tester.pump();

    expect(tester.takeException(), isNull);
    expect(find.text('此平台尚未提供内置浏览'), findsOneWidget);
    expect(find.text('请在手机或电脑使用内置浏览'), findsNothing);
  });
}
