import 'package:video_sniffing/features/browse/browse_url.dart';

const kMaxIngressPayloadLength = 8192;

final _httpUrlInText = RegExp(
  r'https?://[^\s]+',
  caseSensitive: false,
);

bool isIngressPayloadTooLong(String raw) =>
    raw.length > kMaxIngressPayloadLength;

String? extractHttpUrl(String raw) {
  final trimmed = raw.trim();
  if (browseUrlError(trimmed) == null) {
    return trimmed;
  }
  final match = _httpUrlInText.firstMatch(trimmed);
  if (match == null) {
    return null;
  }
  final candidate = match.group(0)!;
  if (browseUrlError(candidate) == null) {
    return candidate;
  }
  return null;
}
