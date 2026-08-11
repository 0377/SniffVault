enum MediaKind {
  mp4('mp4'),
  hls('hls'),
  other('other');

  const MediaKind(this.jsonValue);

  final String jsonValue;

  static MediaKind fromJson(String value) {
    return MediaKind.values.firstWhere(
      (kind) => kind.jsonValue == value,
      orElse: () => throw ArgumentError('unknown media kind: $value'),
    );
  }

  String toJson() => jsonValue;
}

class Quality {
  const Quality({
    required this.label,
    this.width,
    this.height,
    this.bandwidth,
  });

  final String label;
  final int? width;
  final int? height;
  final int? bandwidth;

  factory Quality.fromJson(Map<String, dynamic> json) {
    return Quality(
      label: json['label'] as String,
      width: json['width'] as int?,
      height: json['height'] as int?,
      bandwidth: json['bandwidth'] as int?,
    );
  }

  Map<String, dynamic> toJson() => {
        'label': label,
        if (width != null) 'width': width,
        if (height != null) 'height': height,
        if (bandwidth != null) 'bandwidth': bandwidth,
      };

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is Quality &&
          label == other.label &&
          width == other.width &&
          height == other.height &&
          bandwidth == other.bandwidth;

  @override
  int get hashCode => Object.hash(label, width, height, bandwidth);
}

class ResourceCandidate {
  const ResourceCandidate({
    required this.id,
    required this.url,
    this.title,
    required this.kind,
    this.quality,
    this.pageUrl,
  });

  final String id;
  final String url;
  final String? title;
  final MediaKind kind;
  final Quality? quality;
  final String? pageUrl;

  factory ResourceCandidate.fromJson(Map<String, dynamic> json) {
    return ResourceCandidate(
      id: json['id'] as String,
      url: json['url'] as String,
      title: json['title'] as String?,
      kind: MediaKind.fromJson(json['kind'] as String),
      quality: json['quality'] == null
          ? null
          : Quality.fromJson(json['quality'] as Map<String, dynamic>),
      pageUrl: json['page_url'] as String?,
    );
  }

  Map<String, dynamic> toJson() => {
        'id': id,
        'url': url,
        if (title != null) 'title': title,
        'kind': kind.toJson(),
        if (quality != null) 'quality': quality!.toJson(),
        if (pageUrl != null) 'page_url': pageUrl,
      };

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is ResourceCandidate &&
          id == other.id &&
          url == other.url &&
          title == other.title &&
          kind == other.kind &&
          quality == other.quality &&
          pageUrl == other.pageUrl;

  @override
  int get hashCode =>
      Object.hash(id, url, title, kind, quality, pageUrl);
}

class Episode {
  const Episode({
    required this.index,
    required this.title,
    required this.url,
    required this.qualityOptions,
  });

  final int index;
  final String title;
  final String url;
  final List<Quality> qualityOptions;

  factory Episode.fromJson(Map<String, dynamic> json) {
    return Episode(
      index: json['index'] as int,
      title: json['title'] as String,
      url: json['url'] as String,
      qualityOptions: (json['quality_options'] as List<dynamic>)
          .map((e) => Quality.fromJson(e as Map<String, dynamic>))
          .toList(),
    );
  }

  Map<String, dynamic> toJson() => {
        'index': index,
        'title': title,
        'url': url,
        'quality_options':
            qualityOptions.map((quality) => quality.toJson()).toList(),
      };

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is Episode &&
          index == other.index &&
          title == other.title &&
          url == other.url &&
          _listEquals(qualityOptions, other.qualityOptions);

  @override
  int get hashCode => Object.hash(index, title, url, Object.hashAll(qualityOptions));
}

class EpisodeList {
  const EpisodeList({
    required this.title,
    this.season,
    required this.episodes,
  });

  final String title;
  final int? season;
  final List<Episode> episodes;

  factory EpisodeList.fromJson(Map<String, dynamic> json) {
    return EpisodeList(
      title: json['title'] as String,
      season: json['season'] as int?,
      episodes: (json['episodes'] as List<dynamic>)
          .map((e) => Episode.fromJson(e as Map<String, dynamic>))
          .toList(),
    );
  }

  Map<String, dynamic> toJson() => {
        'title': title,
        if (season != null) 'season': season,
        'episodes': episodes.map((episode) => episode.toJson()).toList(),
      };

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is EpisodeList &&
          title == other.title &&
          season == other.season &&
          _listEquals(episodes, other.episodes);

  @override
  int get hashCode => Object.hash(title, season, Object.hashAll(episodes));
}

class ResolveOptions {
  const ResolveOptions({
    this.cookies,
    this.referer,
    this.pageUrl,
  });

  final String? cookies;
  final String? referer;
  final String? pageUrl;

  factory ResolveOptions.fromJson(Map<String, dynamic> json) {
    return ResolveOptions(
      cookies: json['cookies'] as String?,
      referer: json['referer'] as String?,
      pageUrl: json['page_url'] as String?,
    );
  }

  Map<String, dynamic> toJson() => {
        if (cookies != null) 'cookies': cookies,
        if (referer != null) 'referer': referer,
        if (pageUrl != null) 'page_url': pageUrl,
      };

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is ResolveOptions &&
          cookies == other.cookies &&
          referer == other.referer &&
          pageUrl == other.pageUrl;

  @override
  int get hashCode => Object.hash(cookies, referer, pageUrl);
}

sealed class ResolveOutcome {
  const ResolveOutcome();

  factory ResolveOutcome.fromJson(Map<String, dynamic> json) {
    if (json.containsKey('single')) {
      return ResolveOutcomeSingle(
        ResourceCandidate.fromJson(json['single'] as Map<String, dynamic>),
      );
    }
    if (json.containsKey('candidates')) {
      return ResolveOutcomeCandidates(
        (json['candidates'] as List<dynamic>)
            .map((e) => ResourceCandidate.fromJson(e as Map<String, dynamic>))
            .toList(),
      );
    }
    if (json.containsKey('episode_list')) {
      return ResolveOutcomeEpisodeList(
        EpisodeList.fromJson(json['episode_list'] as Map<String, dynamic>),
      );
    }
    if (json.containsKey('needs_browser')) {
      final payload = json['needs_browser'] as Map<String, dynamic>;
      return ResolveOutcomeNeedsBrowser(reason: payload['reason'] as String);
    }
    throw ArgumentError('unknown resolve outcome: $json');
  }

  Map<String, dynamic> toJson();
}

class ResolveOutcomeSingle extends ResolveOutcome {
  const ResolveOutcomeSingle(this.candidate);

  final ResourceCandidate candidate;

  @override
  Map<String, dynamic> toJson() => {'single': candidate.toJson()};

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is ResolveOutcomeSingle && candidate == other.candidate;

  @override
  int get hashCode => candidate.hashCode;
}

class ResolveOutcomeCandidates extends ResolveOutcome {
  const ResolveOutcomeCandidates(this.candidates);

  final List<ResourceCandidate> candidates;

  @override
  Map<String, dynamic> toJson() => {
        'candidates': candidates.map((c) => c.toJson()).toList(),
      };

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is ResolveOutcomeCandidates &&
          _listEquals(candidates, other.candidates);

  @override
  int get hashCode => Object.hashAll(candidates);
}

class ResolveOutcomeEpisodeList extends ResolveOutcome {
  const ResolveOutcomeEpisodeList(this.episodeList);

  final EpisodeList episodeList;

  @override
  Map<String, dynamic> toJson() => {'episode_list': episodeList.toJson()};

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is ResolveOutcomeEpisodeList && episodeList == other.episodeList;

  @override
  int get hashCode => episodeList.hashCode;
}

class ResolveOutcomeNeedsBrowser extends ResolveOutcome {
  const ResolveOutcomeNeedsBrowser({required this.reason});

  final String reason;

  @override
  Map<String, dynamic> toJson() => {
        'needs_browser': {'reason': reason},
      };

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is ResolveOutcomeNeedsBrowser && reason == other.reason;

  @override
  int get hashCode => reason.hashCode;
}

bool _listEquals<T>(List<T> a, List<T> b) {
  if (a.length != b.length) return false;
  for (var i = 0; i < a.length; i++) {
    if (a[i] != b[i]) return false;
  }
  return true;
}
