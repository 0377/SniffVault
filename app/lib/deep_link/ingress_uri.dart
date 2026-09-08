import 'package:video_sniffing/deep_link/share_url_extractor.dart';

enum IngressFailure { missingPayload, payloadTooLong, noHttpUrl, invalidScheme }

sealed class IngressNavigateTarget {}

class IngressNavigateSuccess extends IngressNavigateTarget {
  IngressNavigateSuccess(this.url);
  final String url;
}

class IngressNavigateFailure extends IngressNavigateTarget {
  IngressNavigateFailure(this.reason);
  final IngressFailure reason;
}

final _schemePrefix = RegExp(r'[a-z][a-z0-9+.-]*://', caseSensitive: false);

bool _hasDisallowedScheme(String raw) {
  final trimmed = raw.trim();
  final direct = Uri.tryParse(trimmed);
  if (direct != null && direct.hasScheme) {
    final scheme = direct.scheme.toLowerCase();
    if (scheme != 'http' && scheme != 'https') {
      return true;
    }
  }
  for (final match in _schemePrefix.allMatches(trimmed)) {
    final prefix = match.group(0)!.toLowerCase();
    if (!prefix.startsWith('http://') && !prefix.startsWith('https://')) {
      return true;
    }
  }
  return false;
}

String snackBarMessageFor(IngressFailure failure) {
  return switch (failure) {
    IngressFailure.invalidScheme => '仅支持 http/https 链接',
    IngressFailure.payloadTooLong => '链接过长',
    IngressFailure.missingPayload ||
    IngressFailure.noHttpUrl =>
      '未能识别有效链接',
  };
}

IngressNavigateTarget? parseSniffVaultIngress(Uri uri) {
  if (uri.scheme != 'sniffvault' || uri.host != 'add') {
    return null;
  }
  final payload = uri.queryParameters['url'];
  if (payload == null || payload.trim().isEmpty) {
    return IngressNavigateFailure(IngressFailure.missingPayload);
  }
  if (isIngressPayloadTooLong(payload)) {
    return IngressNavigateFailure(IngressFailure.payloadTooLong);
  }
  if (_hasDisallowedScheme(payload)) {
    return IngressNavigateFailure(IngressFailure.invalidScheme);
  }
  final target = extractHttpUrl(payload);
  if (target == null) {
    return IngressNavigateFailure(IngressFailure.noHttpUrl);
  }
  return IngressNavigateSuccess(target);
}
