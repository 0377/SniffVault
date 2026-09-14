import 'package:flutter/material.dart';

Future<String?> showRetryUrlDialog(
  BuildContext context, {
  required String initialUrl,
}) {
  return showDialog<String>(
    context: context,
    builder: (dialogContext) => _RetryUrlDialog(initialUrl: initialUrl),
  );
}

class _RetryUrlDialog extends StatefulWidget {
  const _RetryUrlDialog({required this.initialUrl});
  final String initialUrl;

  @override
  State<_RetryUrlDialog> createState() => _RetryUrlDialogState();
}

class _RetryUrlDialogState extends State<_RetryUrlDialog> {
  late final TextEditingController _controller;
  String? _error;

  @override
  void initState() {
    super.initState();
    _controller = TextEditingController(text: widget.initialUrl);
  }

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  void _submit() {
    final trimmed = _controller.text.trim();
    if (trimmed.isEmpty) {
      setState(() => _error = 'URL 不能为空');
      return;
    }
    Navigator.of(context).pop(trimmed);
  }

  @override
  Widget build(BuildContext context) {
    return AlertDialog(
      title: const Text('修改 URL 后重试'),
      content: TextField(
        key: const Key('retry_url_field'),
        controller: _controller,
        maxLines: 3,
        decoration: InputDecoration(
          labelText: '资源 URL',
          errorText: _error,
          border: const OutlineInputBorder(),
        ),
      ),
      actions: [
        TextButton(
          key: const Key('retry_url_cancel'),
          onPressed: () => Navigator.of(context).pop(),
          child: const Text('取消'),
        ),
        FilledButton(
          key: const Key('retry_url_confirm'),
          onPressed: _submit,
          child: const Text('重试'),
        ),
      ],
    );
  }
}
