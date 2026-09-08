import 'dart:io';

import 'package:path_provider/path_provider.dart';

const _profileDirName = 'webview_profile';

/// Windows 浏览与 [WebViewSniff] 共用的 WebView2 User Data Folder。
/// 非 Windows 返回 null（使用平台默认 profile）。
Future<String?> webviewUserDataPath() async {
  if (!Platform.isWindows) {
    return null;
  }
  final support = await getApplicationSupportDirectory();
  final dir = Directory('${support.path}/$_profileDirName');
  if (!await dir.exists()) {
    await dir.create(recursive: true);
  }
  return dir.path;
}
