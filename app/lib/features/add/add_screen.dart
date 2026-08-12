import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:video_sniffing/engine/engine_host.dart';
import 'package:video_sniffing/features/add/resolve_wizard.dart';
import 'package:video_sniffing/providers/download_coordinator.dart';
import 'package:video_sniffing/providers/engine_host_provider.dart';
import 'package:video_sniffing/providers/settings_provider.dart';
import 'package:video_sniffing/ui/error_presenter.dart';
import 'package:video_sniffing/ui/loading_overlay.dart';

class AddScreen extends ConsumerStatefulWidget {
  const AddScreen({super.key, this.initialUrl});

  final String? initialUrl;

  @override
  ConsumerState<AddScreen> createState() => _AddScreenState();
}

class _AddScreenState extends ConsumerState<AddScreen> {
  late final TextEditingController _urlController;
  String? _errorMessage;

  @override
  void initState() {
    super.initState();
    _urlController = TextEditingController(text: widget.initialUrl ?? '');
  }

  @override
  void dispose() {
    _urlController.dispose();
    super.dispose();
  }

  Future<void> _resolve() async {
    final url = _urlController.text.trim();
    if (url.isEmpty) return;

    setState(() => _errorMessage = null);
    final repo = ref.read(engineRepositoryProvider);

    try {
      final outcome = await LoadingOverlay.run(
        context,
        () => repo.resolveUrl(url),
      );
      if (!mounted) return;

      final settings = ref.read(settingsProvider);
      await Navigator.of(context).push<void>(
        MaterialPageRoute(
          builder: (context) => Scaffold(
            appBar: AppBar(title: const Text('确认下载')),
            body: ResolveWizard(
              outcome: outcome,
              defaultQualityLabel: settings.defaultQualityLabel,
              resolveQualities: repo.resolveQualities,
              enqueueSingle: repo.enqueueSingle,
              enqueueEpisodes: repo.enqueueEpisodes,
              onEnqueue: (_) async {
                ref.read(downloadCoordinatorProvider).ensureDownloads();
                if (!mounted) return;
                Navigator.of(context).pop();
                if (!mounted) return;
                ScaffoldMessenger.of(context).showSnackBar(
                  const SnackBar(content: Text('已加入下载队列')),
                );
                context.go('/tasks');
              },
            ),
          ),
        ),
      );
    } on EngineException catch (e) {
      final message = presentEngineError(e);
      if (message != null && mounted) {
        setState(() => _errorMessage = message);
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: const Text('添加')),
      body: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            TextField(
              key: const Key('add_url_field'),
              decoration: const InputDecoration(
                labelText: '视频 URL',
                border: OutlineInputBorder(),
              ),
              controller: _urlController,
              keyboardType: TextInputType.url,
              maxLines: 3,
            ),
            const SizedBox(height: 16),
            if (_errorMessage != null)
              Padding(
                padding: const EdgeInsets.only(bottom: 16),
                child: Text(
                  _errorMessage!,
                  style: TextStyle(color: Theme.of(context).colorScheme.error),
                ),
              ),
            FilledButton(
              key: const Key('add_resolve_button'),
              onPressed: _resolve,
              child: const Text('解析'),
            ),
          ],
        ),
      ),
    );
  }
}
