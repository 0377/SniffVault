import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:video_sniffing/engine/engine_host.dart';
import 'package:video_sniffing/engine/models/engine_settings.dart';
import 'package:video_sniffing/providers/browse_session.dart';
import 'package:video_sniffing/providers/device_profile.dart';
import 'package:video_sniffing/providers/engine_host_provider.dart';
import 'package:video_sniffing/providers/lan_settings_coordinator.dart';
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
  String? _pairingPin;

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
    _pairingPin = ref.read(engineRepositoryProvider).pairingPin();
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

  Future<void> _clearBrowseCookies() async {
    await ref.read(browseCookieStoreProvider).clearAll();
    if (!mounted) {
      return;
    }
    ScaffoldMessenger.of(
      context,
    ).showSnackBar(const SnackBar(content: Text('已清除浏览 Cookie')));
  }

  Future<void> _save() async {
    setState(() => _errorMessage = null);
    final repo = ref.read(engineRepositoryProvider);
    try {
      repo.saveSettings(_draft);
      ref.invalidate(settingsProvider);
      final isTv = await ref.read(isTelevisionProvider.future);
      await applyLanSettings(ref, isReceiver: isTv);
      if (isTv && _draft.lanEnabled) {
        setState(() {
          _pairingPin = repo.pairingPin();
        });
      }
      if (mounted) {
        ScaffoldMessenger.of(
          context,
        ).showSnackBar(const SnackBar(content: Text('设置已保存')));
      }
    } on EngineException catch (e) {
      final message = presentEngineError(e);
      if (message != null) {
        setState(() => _errorMessage = message);
      }
    }
  }

  void _refreshPairingPin() {
    final repo = ref.read(engineRepositoryProvider);
    final pin = repo.beginPairing();
    setState(() => _pairingPin = pin);
  }

  @override
  Widget build(BuildContext context) {
    final isTv = ref.watch(isTelevisionProvider).value == true;
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
          const SizedBox(height: 16),
          SwitchListTile(
            key: const Key('settings_lan_enabled'),
            title: const Text('局域网投送'),
            value: _draft.lanEnabled,
            onChanged: (value) {
              setState(() {
                _draft = _draft.copyWith(lanEnabled: value);
                if (!value) {
                  _pairingPin = null;
                } else if (isTv) {
                  _pairingPin = ref.read(engineRepositoryProvider).pairingPin();
                }
              });
            },
          ),
          if (isTv && _draft.lanEnabled) ...[
            const SizedBox(height: 8),
            Text(
              '配对码',
              style: Theme.of(context).textTheme.titleMedium,
            ),
            const SizedBox(height: 8),
            Text(
              _pairingPin ?? '------',
              key: const Key('settings_pairing_pin'),
              style: Theme.of(context).textTheme.displayMedium?.copyWith(
                letterSpacing: 8,
                fontWeight: FontWeight.bold,
              ),
              textAlign: TextAlign.center,
            ),
            const SizedBox(height: 8),
            const Text(
              '配对码 60 秒后过期，请在发送端输入',
              textAlign: TextAlign.center,
            ),
            const SizedBox(height: 8),
            OutlinedButton(
              key: const Key('settings_refresh_pin'),
              onPressed: _refreshPairingPin,
              child: const Text('刷新配对码'),
            ),
          ],
          ListTile(
            key: const Key('settings_trusted_devices'),
            title: const Text('已信任设备'),
            trailing: const Icon(Icons.chevron_right),
            onTap: () => context.push('/settings/trusted-devices'),
          ),
          const SizedBox(height: 24),
          if (_errorMessage != null)
            Text(
              _errorMessage!,
              key: const Key('settings_error'),
              style: TextStyle(color: Theme.of(context).colorScheme.error),
            ),
          const SizedBox(height: 16),
          OutlinedButton(
            key: const Key('settings_clear_browse_cookies'),
            onPressed: _clearBrowseCookies,
            child: const Text('清除浏览 Cookie'),
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
