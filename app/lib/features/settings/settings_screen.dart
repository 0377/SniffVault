import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:video_sniffing/engine/engine_host.dart';
import 'package:video_sniffing/engine/models/engine_settings.dart';
import 'package:video_sniffing/providers/engine_host_provider.dart';
import 'package:video_sniffing/providers/settings_provider.dart';
import 'package:video_sniffing/ui/error_presenter.dart';

class SettingsScreen extends ConsumerStatefulWidget {
  const SettingsScreen({super.key});

  @override
  ConsumerState<SettingsScreen> createState() => _SettingsScreenState();
}

class _SettingsScreenState extends ConsumerState<SettingsScreen> {
  late EngineSettings _draft;
  late final TextEditingController _mediaDirController;
  late final TextEditingController _maxConcurrencyController;
  late final TextEditingController _defaultQualityController;
  late final TextEditingController _userAgentController;
  late final TextEditingController _deviceNameController;
  String? _errorMessage;

  @override
  void initState() {
    super.initState();
    _draft = ref.read(settingsProvider);
    _mediaDirController = TextEditingController(text: _draft.mediaDir);
    _maxConcurrencyController = TextEditingController(
      text: _draft.maxConcurrency.toString(),
    );
    _defaultQualityController = TextEditingController(
      text: _draft.defaultQualityLabel ?? '',
    );
    _userAgentController = TextEditingController(text: _draft.userAgent ?? '');
    _deviceNameController = TextEditingController(text: _draft.deviceName);
  }

  @override
  void dispose() {
    _mediaDirController.dispose();
    _maxConcurrencyController.dispose();
    _defaultQualityController.dispose();
    _userAgentController.dispose();
    _deviceNameController.dispose();
    super.dispose();
  }

  void _save() {
    setState(() => _errorMessage = null);
    final repo = ref.read(engineRepositoryProvider);
    try {
      repo.saveSettings(_draft);
      ref.invalidate(settingsProvider);
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          const SnackBar(content: Text('设置已保存')),
        );
      }
    } on EngineException catch (e) {
      final message = presentEngineError(e);
      if (message != null) {
        setState(() => _errorMessage = message);
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: const Text('设置')),
      body: ListView(
        padding: const EdgeInsets.all(16),
        children: [
          TextField(
            key: const Key('settings_media_dir'),
            decoration: const InputDecoration(
              labelText: '媒体目录',
              border: OutlineInputBorder(),
            ),
            controller: _mediaDirController,
            onChanged: (value) => _draft = _draft.copyWith(mediaDir: value),
          ),
          const SizedBox(height: 16),
          TextField(
            key: const Key('settings_max_concurrency'),
            decoration: const InputDecoration(
              labelText: '最大并发数',
              border: OutlineInputBorder(),
            ),
            keyboardType: TextInputType.number,
            controller: _maxConcurrencyController,
            onChanged: (value) {
              final parsed = int.tryParse(value);
              if (parsed != null) {
                _draft = _draft.copyWith(maxConcurrency: parsed);
              }
            },
          ),
          const SizedBox(height: 16),
          TextField(
            key: const Key('settings_default_quality'),
            decoration: const InputDecoration(
              labelText: '默认清晰度',
              border: OutlineInputBorder(),
            ),
            controller: _defaultQualityController,
            onChanged: (value) => _draft = _draft.copyWith(
              defaultQualityLabel: value.isEmpty ? null : value,
            ),
          ),
          const SizedBox(height: 16),
          TextField(
            key: const Key('settings_user_agent'),
            decoration: const InputDecoration(
              labelText: 'User-Agent',
              border: OutlineInputBorder(),
            ),
            controller: _userAgentController,
            onChanged: (value) => _draft = _draft.copyWith(
              userAgent: value.isEmpty ? null : value,
            ),
          ),
          const SizedBox(height: 16),
          TextField(
            key: const Key('settings_device_name'),
            decoration: const InputDecoration(
              labelText: '设备名称',
              border: OutlineInputBorder(),
            ),
            controller: _deviceNameController,
            onChanged: (value) => _draft = _draft.copyWith(deviceName: value),
          ),
          const SizedBox(height: 24),
          if (_errorMessage != null)
            Text(
              _errorMessage!,
              key: const Key('settings_error'),
              style: TextStyle(color: Theme.of(context).colorScheme.error),
            ),
          const SizedBox(height: 16),
          FilledButton(
            key: const Key('settings_save'),
            onPressed: _save,
            child: const Text('保存'),
          ),
        ],
      ),
    );
  }
}
