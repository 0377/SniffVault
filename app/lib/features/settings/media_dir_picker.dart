import 'package:path/path.dart' as p;

String? mediaDirNameFromPickerResult(String? pickedPath) {
  if (pickedPath == null) {
    return null;
  }
  final trimmed = pickedPath.replaceAll(RegExp(r'[/\\]+$'), '');
  if (trimmed.isEmpty) {
    return null;
  }
  final context = trimmed.contains(r'\')
      ? p.Context(style: p.Style.windows)
      : p.Context(style: p.Style.posix);
  final name = context.basename(trimmed);
  if (name.isEmpty || name == '.' || name == '..') {
    return null;
  }
  if (name.contains('/') || name.contains(r'\')) {
    return null;
  }
  return name;
}
