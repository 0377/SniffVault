enum CastEventKind {
  incomingPlay('incoming_play'),
  sessionEnded('session_ended'),
  error('error');

  const CastEventKind(this.jsonValue);

  final String jsonValue;

  static CastEventKind fromJson(String value) {
    return CastEventKind.values.firstWhere(
      (kind) => kind.jsonValue == value,
      orElse: () => throw ArgumentError('unknown cast event kind: $value'),
    );
  }

  String toJson() => jsonValue;
}

class CastMetadata {
  const CastMetadata({
    required this.title,
    this.season,
    required this.episodeIndex,
    required this.episodeTitle,
    this.durationMs,
    required this.positionMs,
    required this.mime,
  });

  final String title;
  final int? season;
  final int episodeIndex;
  final String episodeTitle;
  final int? durationMs;
  final int positionMs;
  final String mime;

  factory CastMetadata.fromJson(Map<String, dynamic> json) {
    return CastMetadata(
      title: json['title'] as String,
      season: json['season'] as int?,
      episodeIndex: json['episode_index'] as int,
      episodeTitle: json['episode_title'] as String,
      durationMs: json['duration_ms'] as int?,
      positionMs: json['position_ms'] as int,
      mime: json['mime'] as String,
    );
  }

  Map<String, dynamic> toJson() => {
        'title': title,
        if (season != null) 'season': season,
        'episode_index': episodeIndex,
        'episode_title': episodeTitle,
        if (durationMs != null) 'duration_ms': durationMs,
        'position_ms': positionMs,
        'mime': mime,
      };

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is CastMetadata &&
          title == other.title &&
          season == other.season &&
          episodeIndex == other.episodeIndex &&
          episodeTitle == other.episodeTitle &&
          durationMs == other.durationMs &&
          positionMs == other.positionMs &&
          mime == other.mime;

  @override
  int get hashCode => Object.hash(
        title,
        season,
        episodeIndex,
        episodeTitle,
        durationMs,
        positionMs,
        mime,
      );
}

class CastPlayRequest {
  const CastPlayRequest({
    required this.sessionId,
    required this.senderDeviceId,
    required this.senderName,
    required this.metadata,
    required this.streamUrl,
  });

  final String sessionId;
  final String senderDeviceId;
  final String senderName;
  final CastMetadata metadata;
  final String streamUrl;

  factory CastPlayRequest.fromJson(Map<String, dynamic> json) {
    return CastPlayRequest(
      sessionId: json['session_id'] as String,
      senderDeviceId: json['sender_device_id'] as String,
      senderName: json['sender_name'] as String,
      metadata: CastMetadata.fromJson(json['metadata'] as Map<String, dynamic>),
      streamUrl: json['stream_url'] as String,
    );
  }

  Map<String, dynamic> toJson() => {
        'session_id': sessionId,
        'sender_device_id': senderDeviceId,
        'sender_name': senderName,
        'metadata': metadata.toJson(),
        'stream_url': streamUrl,
      };

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is CastPlayRequest &&
          sessionId == other.sessionId &&
          senderDeviceId == other.senderDeviceId &&
          senderName == other.senderName &&
          metadata == other.metadata &&
          streamUrl == other.streamUrl;

  @override
  int get hashCode =>
      Object.hash(sessionId, senderDeviceId, senderName, metadata, streamUrl);
}

class LanPeer {
  const LanPeer({
    required this.deviceId,
    required this.deviceName,
    required this.host,
    required this.port,
    required this.isTrusted,
  });

  final String deviceId;
  final String deviceName;
  final String host;
  final int port;
  final bool isTrusted;

  factory LanPeer.fromJson(Map<String, dynamic> json) {
    return LanPeer(
      deviceId: json['device_id'] as String,
      deviceName: json['device_name'] as String,
      host: json['host'] as String,
      port: json['port'] as int,
      isTrusted: json['is_trusted'] as bool,
    );
  }

  Map<String, dynamic> toJson() => {
        'device_id': deviceId,
        'device_name': deviceName,
        'host': host,
        'port': port,
        'is_trusted': isTrusted,
      };

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is LanPeer &&
          deviceId == other.deviceId &&
          deviceName == other.deviceName &&
          host == other.host &&
          port == other.port &&
          isTrusted == other.isTrusted;

  @override
  int get hashCode =>
      Object.hash(deviceId, deviceName, host, port, isTrusted);
}

class TrustedPeer {
  const TrustedPeer({
    required this.peerDeviceId,
    required this.peerName,
    required this.peerHost,
    required this.peerPort,
    required this.pairedAtMs,
    this.sessionSecret,
  });

  final String peerDeviceId;
  final String peerName;
  final String peerHost;
  final int peerPort;
  final int pairedAtMs;
  final List<int>? sessionSecret;

  factory TrustedPeer.fromJson(Map<String, dynamic> json) {
    return TrustedPeer(
      peerDeviceId: json['peer_device_id'] as String,
      peerName: json['peer_name'] as String,
      peerHost: json['peer_host'] as String,
      peerPort: json['peer_port'] as int,
      pairedAtMs: json['paired_at_ms'] as int,
      sessionSecret: json['session_secret'] == null
          ? null
          : (json['session_secret'] as List<dynamic>).cast<int>(),
    );
  }

  Map<String, dynamic> toJson() => {
        'peer_device_id': peerDeviceId,
        'peer_name': peerName,
        'peer_host': peerHost,
        'peer_port': peerPort,
        'paired_at_ms': pairedAtMs,
        if (sessionSecret != null) 'session_secret': sessionSecret,
      };

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is TrustedPeer &&
          peerDeviceId == other.peerDeviceId &&
          peerName == other.peerName &&
          peerHost == other.peerHost &&
          peerPort == other.peerPort &&
          pairedAtMs == other.pairedAtMs &&
          _listEquals(sessionSecret, other.sessionSecret);

  @override
  int get hashCode => Object.hash(
        peerDeviceId,
        peerName,
        peerHost,
        peerPort,
        pairedAtMs,
        sessionSecret == null ? null : Object.hashAll(sessionSecret!),
      );
}

sealed class CastEvent {
  const CastEvent();

  CastMetadata? get metadata => switch (this) {
        CastEventIncomingPlay(:final request) => request.metadata,
        _ => null,
      };

  factory CastEvent.fromJson(Map<String, dynamic> json) {
    switch (CastEventKind.fromJson(json['kind'] as String)) {
      case CastEventKind.incomingPlay:
        return CastEventIncomingPlay(
          CastPlayRequest.fromJson(json['request'] as Map<String, dynamic>),
        );
      case CastEventKind.sessionEnded:
        return CastEventSessionEnded(
          sessionId: json['session_id'] as String,
        );
      case CastEventKind.error:
        return CastEventError(message: json['message'] as String);
    }
  }

  Map<String, dynamic> toJson();
}

class CastEventIncomingPlay extends CastEvent {
  const CastEventIncomingPlay(this.request);

  final CastPlayRequest request;

  @override
  Map<String, dynamic> toJson() => {
        'kind': CastEventKind.incomingPlay.toJson(),
        'request': request.toJson(),
      };

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is CastEventIncomingPlay && request == other.request;

  @override
  int get hashCode => request.hashCode;
}

class CastEventSessionEnded extends CastEvent {
  const CastEventSessionEnded({required this.sessionId});

  final String sessionId;

  @override
  Map<String, dynamic> toJson() => {
        'kind': CastEventKind.sessionEnded.toJson(),
        'session_id': sessionId,
      };

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is CastEventSessionEnded && sessionId == other.sessionId;

  @override
  int get hashCode => sessionId.hashCode;
}

class CastEventError extends CastEvent {
  const CastEventError({required this.message});

  final String message;

  @override
  Map<String, dynamic> toJson() => {
        'kind': CastEventKind.error.toJson(),
        'message': message,
      };

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is CastEventError && message == other.message;

  @override
  int get hashCode => message.hashCode;
}

bool _listEquals<T>(List<T>? a, List<T>? b) {
  if (identical(a, b)) return true;
  if (a == null || b == null) return false;
  if (a.length != b.length) return false;
  for (var i = 0; i < a.length; i++) {
    if (a[i] != b[i]) return false;
  }
  return true;
}
