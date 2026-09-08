import 'dart:io';

import 'package:app_links/app_links.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:media_kit/media_kit.dart';
import 'app.dart';
import 'bootstrap/webview_profile.dart';
import 'bootstrap/windows_webview_bootstrap.dart';
import 'deep_link/deep_link_host.dart';
import 'deep_link/deep_link_providers.dart';
import 'engine/native_bindings.dart';
import 'providers/webview_bootstrap_provider.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  MediaKit.ensureInitialized();

  var webviewReady = true;
  if (Platform.isWindows) {
    final profile = await webviewUserDataPath();
    webviewReady = await bootstrapWindowsWebViewFromProfile(profile);
  }

  openNativeLibrary();
  setBootstrapIngressUri(await AppLinks().getInitialLink());
  runApp(
    ProviderScope(
      overrides: [
        webviewBootstrapReadyProvider.overrideWith((ref) => webviewReady),
      ],
      child: const IngressUriListener(
        child: VideoSniffingApp(),
      ),
    ),
  );
}
