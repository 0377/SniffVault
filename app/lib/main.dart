import 'package:flutter/material.dart';
import 'package:path_provider/path_provider.dart';

import 'engine/engine_host.dart';
import 'engine/native_bindings.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();

  openNativeLibrary();
  final dataDir = await getApplicationSupportDirectory();
  final host = await EngineHost.open(dataDir.path);
  try {
    final settings = host.settings();
    final library = host.listLibrary();
    debugPrint(
      'Engine smoke: device=${settings.deviceName}, library=${library.length}',
    );
  } finally {
    host.dispose();
  }

  runApp(const VideoSniffingApp());
}

class VideoSniffingApp extends StatelessWidget {
  const VideoSniffingApp({super.key});

  @override
  Widget build(BuildContext context) {
    return const MaterialApp(
      home: Scaffold(
        body: Center(
          child: Text('Video Sniffing engine smoke OK'),
        ),
      ),
    );
  }
}
