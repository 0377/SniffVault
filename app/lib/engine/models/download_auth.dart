class DownloadAuth {
  const DownloadAuth({
    this.cookies,
    this.referer,
  });

  final String? cookies;
  final String? referer;

  factory DownloadAuth.fromJson(Map<String, dynamic> json) {
    return DownloadAuth(
      cookies: json['cookies'] as String?,
      referer: json['referer'] as String?,
    );
  }

  Map<String, dynamic> toJson() => {
        if (cookies != null) 'cookies': cookies,
        if (referer != null) 'referer': referer,
      };

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is DownloadAuth &&
          cookies == other.cookies &&
          referer == other.referer;

  @override
  int get hashCode => Object.hash(cookies, referer);
}
