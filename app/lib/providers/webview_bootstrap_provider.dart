import 'package:flutter_riverpod/flutter_riverpod.dart';

/// Windows WebView2 环境是否初始化成功；非 Windows 恒 true。
final webviewBootstrapReadyProvider = Provider<bool>((ref) => true);
