import 'dart:io' show Platform;

import 'package:flutter/foundation.dart' show kIsWeb;
import 'package:flutter/services.dart';

const kDeviceChannel = MethodChannel('webview_sniff/device');

Future<bool> detectIsTelevision() async {
  if (kIsWeb || !Platform.isAndroid) {
    return false;
  }
  try {
    final result = await kDeviceChannel.invokeMethod<bool>('isTelevision');
    return result ?? false;
  } on MissingPluginException {
    return false;
  } on PlatformException {
    return false;
  }
}
