enum SniffInitiator {
  navigation('navigation'),
  subResource('sub_resource'),
  media('media'),
  other('other');

  const SniffInitiator(this.jsonValue);

  final String jsonValue;

  static SniffInitiator fromJson(String value) {
    return SniffInitiator.values.firstWhere(
      (initiator) => initiator.jsonValue == value,
      orElse: () => throw ArgumentError('unknown sniff initiator: $value'),
    );
  }

  String toJson() => jsonValue;
}

class SniffEvent {
  const SniffEvent({
    required this.url,
    this.pageUrl,
    required this.initiator,
  });

  final String url;
  final String? pageUrl;
  final SniffInitiator initiator;

  factory SniffEvent.fromJson(Map<String, dynamic> json) {
    return SniffEvent(
      url: json['url'] as String,
      pageUrl: json['page_url'] as String?,
      initiator: SniffInitiator.fromJson(json['initiator'] as String),
    );
  }

  Map<String, dynamic> toJson() => {
        'url': url,
        if (pageUrl != null) 'page_url': pageUrl,
        'initiator': initiator.toJson(),
      };

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is SniffEvent &&
          url == other.url &&
          pageUrl == other.pageUrl &&
          initiator == other.initiator;

  @override
  int get hashCode => Object.hash(url, pageUrl, initiator);
}
