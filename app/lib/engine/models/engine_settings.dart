class EngineSettings {
  const EngineSettings({
    required this.mediaDir,
    required this.maxConcurrency,
    this.defaultQualityLabel,
    this.userAgent,
    required this.deviceName,
  });

  final String mediaDir;
  final int maxConcurrency;
  final String? defaultQualityLabel;
  final String? userAgent;
  final String deviceName;

  static const EngineSettings defaults = EngineSettings(
    mediaDir: 'media',
    maxConcurrency: 2,
    defaultQualityLabel: 'highest',
    userAgent: null,
    deviceName: 'VideoSniffing',
  );

  factory EngineSettings.fromJson(Map<String, dynamic> json) {
    return EngineSettings(
      mediaDir: json['media_dir'] as String,
      maxConcurrency: json['max_concurrency'] as int,
      defaultQualityLabel: json['default_quality_label'] as String?,
      userAgent: json['user_agent'] as String?,
      deviceName: json['device_name'] as String,
    );
  }

  Map<String, dynamic> toJson() => {
        'media_dir': mediaDir,
        'max_concurrency': maxConcurrency,
        if (defaultQualityLabel != null)
          'default_quality_label': defaultQualityLabel,
        if (userAgent != null) 'user_agent': userAgent,
        'device_name': deviceName,
      };

  EngineSettings copyWith({
    String? mediaDir,
    int? maxConcurrency,
    String? defaultQualityLabel,
    String? userAgent,
    String? deviceName,
  }) {
    return EngineSettings(
      mediaDir: mediaDir ?? this.mediaDir,
      maxConcurrency: maxConcurrency ?? this.maxConcurrency,
      defaultQualityLabel: defaultQualityLabel ?? this.defaultQualityLabel,
      userAgent: userAgent ?? this.userAgent,
      deviceName: deviceName ?? this.deviceName,
    );
  }

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is EngineSettings &&
          mediaDir == other.mediaDir &&
          maxConcurrency == other.maxConcurrency &&
          defaultQualityLabel == other.defaultQualityLabel &&
          userAgent == other.userAgent &&
          deviceName == other.deviceName;

  @override
  int get hashCode => Object.hash(
        mediaDir,
        maxConcurrency,
        defaultQualityLabel,
        userAgent,
        deviceName,
      );
}
