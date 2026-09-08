class EngineSettings {
  const EngineSettings({
    this.deviceId = '',
    required this.mediaDir,
    required this.maxConcurrency,
    this.defaultQualityLabel,
    this.userAgent,
    required this.deviceName,
    this.lanEnabled = false,
  });

  final String deviceId;
  final String mediaDir;
  final int maxConcurrency;
  final String? defaultQualityLabel;
  final String? userAgent;
  final String deviceName;
  final bool lanEnabled;

  static const EngineSettings defaults = EngineSettings(
    deviceId: '',
    mediaDir: 'media',
    maxConcurrency: 2,
    defaultQualityLabel: 'highest',
    userAgent: null,
    deviceName: 'VideoSniffing',
    lanEnabled: false,
  );

  factory EngineSettings.fromJson(Map<String, dynamic> json) {
    return EngineSettings(
      deviceId: json['device_id'] as String? ?? '',
      mediaDir: json['media_dir'] as String,
      maxConcurrency: json['max_concurrency'] as int,
      defaultQualityLabel: json['default_quality_label'] as String?,
      userAgent: json['user_agent'] as String?,
      deviceName: json['device_name'] as String,
      lanEnabled: json['lan_enabled'] as bool? ?? false,
    );
  }

  Map<String, dynamic> toJson() => {
        if (deviceId.isNotEmpty) 'device_id': deviceId,
        'media_dir': mediaDir,
        'max_concurrency': maxConcurrency,
        if (defaultQualityLabel != null)
          'default_quality_label': defaultQualityLabel,
        if (userAgent != null) 'user_agent': userAgent,
        'device_name': deviceName,
        'lan_enabled': lanEnabled,
      };

  EngineSettings copyWith({
    String? deviceId,
    String? mediaDir,
    int? maxConcurrency,
    String? defaultQualityLabel,
    String? userAgent,
    String? deviceName,
    bool? lanEnabled,
  }) {
    return EngineSettings(
      deviceId: deviceId ?? this.deviceId,
      mediaDir: mediaDir ?? this.mediaDir,
      maxConcurrency: maxConcurrency ?? this.maxConcurrency,
      defaultQualityLabel: defaultQualityLabel ?? this.defaultQualityLabel,
      userAgent: userAgent ?? this.userAgent,
      deviceName: deviceName ?? this.deviceName,
      lanEnabled: lanEnabled ?? this.lanEnabled,
    );
  }

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is EngineSettings &&
          deviceId == other.deviceId &&
          mediaDir == other.mediaDir &&
          maxConcurrency == other.maxConcurrency &&
          defaultQualityLabel == other.defaultQualityLabel &&
          userAgent == other.userAgent &&
          deviceName == other.deviceName &&
          lanEnabled == other.lanEnabled;

  @override
  int get hashCode => Object.hash(
        deviceId,
        mediaDir,
        maxConcurrency,
        defaultQualityLabel,
        userAgent,
        deviceName,
        lanEnabled,
      );
}
