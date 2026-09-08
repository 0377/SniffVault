import 'package:flutter/material.dart';
import 'package:video_sniffing/features/browse/browse_url.dart';

class BrowseChrome extends StatefulWidget {
  const BrowseChrome({
    super.key,
    this.canGoBack = false,
    this.canGoForward = false,
    this.resolveEnabled = true,
    this.url,
    this.onSubmit,
    this.onBack,
    this.onForward,
    this.onReload,
    this.onResolvePage,
  });

  final bool canGoBack;
  final bool canGoForward;
  final bool resolveEnabled;
  final String? url;
  final ValueChanged<Uri>? onSubmit;
  final VoidCallback? onBack;
  final VoidCallback? onForward;
  final VoidCallback? onReload;
  final VoidCallback? onResolvePage;

  @override
  State<BrowseChrome> createState() => _BrowseChromeState();
}

class _BrowseChromeState extends State<BrowseChrome> {
  late final TextEditingController _urlController;
  String? _errorMessage;

  @override
  void initState() {
    super.initState();
    _urlController = TextEditingController(text: widget.url ?? '');
  }

  @override
  void didUpdateWidget(BrowseChrome oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (widget.url != oldWidget.url &&
        widget.url != null &&
        widget.url != _urlController.text) {
      _urlController.text = widget.url!;
    }
  }

  @override
  void dispose() {
    _urlController.dispose();
    super.dispose();
  }

  bool get _canResolve {
    if (!widget.resolveEnabled) {
      return false;
    }
    final url = _urlController.text.trim();
    if (url.isEmpty || url.toLowerCase() == 'about:blank') {
      return false;
    }
    return true;
  }

  void _submit(String raw) {
    final error = browseUrlError(raw);
    setState(() => _errorMessage = error);
    if (error != null) {
      return;
    }
    final uri = parseBrowseUrl(raw);
    if (uri == null) {
      return;
    }
    widget.onSubmit?.call(uri);
  }

  @override
  Widget build(BuildContext context) {
    final errorColor = Theme.of(context).colorScheme.error;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Padding(
          padding: const EdgeInsets.fromLTRB(8, 8, 8, 0),
          child: Row(
            children: [
              IconButton(
                tooltip: '后退',
                onPressed: widget.canGoBack ? widget.onBack : null,
                icon: const Icon(Icons.arrow_back),
              ),
              IconButton(
                tooltip: '前进',
                onPressed: widget.canGoForward ? widget.onForward : null,
                icon: const Icon(Icons.arrow_forward),
              ),
              IconButton(
                tooltip: '刷新',
                onPressed: widget.onReload,
                icon: const Icon(Icons.refresh),
              ),
              Expanded(
                child: TextField(
                  key: const Key('browse_url_field'),
                  controller: _urlController,
                  keyboardType: TextInputType.url,
                  textInputAction: TextInputAction.go,
                  decoration: const InputDecoration(
                    labelText: '地址',
                    border: OutlineInputBorder(),
                    isDense: true,
                  ),
                  onChanged: (_) => setState(() {}),
                  onSubmitted: _submit,
                ),
              ),
              const SizedBox(width: 8),
              FilledButton(
                key: const Key('browse_resolve_page'),
                onPressed: _canResolve ? widget.onResolvePage : null,
                child: const Text('解析本页'),
              ),
            ],
          ),
        ),
        if (_errorMessage != null)
          Padding(
            padding: const EdgeInsets.fromLTRB(16, 8, 16, 0),
            child: Text(_errorMessage!, style: TextStyle(color: errorColor)),
          ),
      ],
    );
  }
}
