import 'package:video_sniffing/engine/engine_host.dart';

/// 返回 `null` 表示无需向用户展示（如 downloads already running）。
String? presentEngineError(EngineException exception) {
  final error = exception.error;
  final lower = error.message.toLowerCase();
  if (lower.contains('downloads already running')) {
    return null;
  }
  switch (error.kind) {
    case 'http':
      return '网络请求失败，请检查连接后重试';
    case 'not_found':
      return '找不到对应内容';
    case 'invalid_arg':
      return error.message;
    case 'db':
    case 'io':
      return '本地存储异常：${error.message}';
    default:
      return error.message;
  }
}
