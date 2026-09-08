import 'package:flutter/foundation.dart';

class UserAgentPreset {
  const UserAgentPreset({
    required this.id,
    required this.label,
    required this.value,
  });

  final String id;
  final String label;
  final String value;

  Key get testKey => Key('settings_user_agent_preset_$id');
}

const userAgentPresetPc = UserAgentPreset(
  id: 'pc',
  label: 'PC',
  value:
      'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36',
);

const userAgentPresetAndroid = UserAgentPreset(
  id: 'android',
  label: '安卓',
  value:
      'Mozilla/5.0 (Linux; Android 10; K) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/152.0.0.0 Mobile Safari/537.36',
);

const userAgentPresetIos = UserAgentPreset(
  id: 'ios',
  label: '苹果',
  value:
      'Mozilla/5.0 (iPhone; CPU iPhone OS 18_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.0 Mobile/15E148 Safari/604.1',
);

const kUserAgentPresets = [
  userAgentPresetPc,
  userAgentPresetAndroid,
  userAgentPresetIos,
];
