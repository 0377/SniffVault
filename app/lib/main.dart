import 'package:app_links/app_links.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:media_kit/media_kit.dart';

import 'app.dart';
import 'deep_link/deep_link_host.dart';
import 'deep_link/deep_link_providers.dart';
import 'engine/native_bindings.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  MediaKit.ensureInitialized();
  openNativeLibrary();
  setBootstrapIngressUri(await AppLinks().getInitialLink());
  runApp(
    const ProviderScope(
      child: IngressUriListener(
        child: VideoSniffingApp(),
      ),
    ),
  );
}
