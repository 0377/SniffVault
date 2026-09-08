class CookiePair {
  const CookiePair({required this.name, required this.value});

  final String name;
  final String value;
}

String? browseUrlError(String raw) {
  final trimmed = raw.trim();
  if (trimmed.isEmpty) {
    return '请输入地址';
  }
  final uri = Uri.tryParse(trimmed);
  if (uri == null) {
    return '非法地址';
  }
  final scheme = uri.scheme.toLowerCase();
  if (scheme != 'http' && scheme != 'https') {
    return '非法地址';
  }
  return null;
}

Uri? parseBrowseUrl(String raw) {
  if (browseUrlError(raw) != null) {
    return null;
  }
  return Uri.tryParse(raw.trim());
}

String cookieHeaderFrom(Iterable<CookiePair> cookies) {
  return cookies.map((cookie) => '${cookie.name}=${cookie.value}').join('; ');
}
