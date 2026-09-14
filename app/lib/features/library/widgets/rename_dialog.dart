import 'package:flutter/material.dart';

class RenameDialog extends StatefulWidget {
  const RenameDialog({super.key, required this.initialTitle});

  final String initialTitle;

  @override
  State<RenameDialog> createState() => _RenameDialogState();
}

class _RenameDialogState extends State<RenameDialog> {
  late final TextEditingController _controller;

  @override
  void initState() {
    super.initState();
    _controller = TextEditingController(text: widget.initialTitle);
    _controller.addListener(() => setState(() {}));
  }

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final trimmed = _controller.text.trim();
    return AlertDialog(
      title: const Text('重命名'),
      content: TextField(
        key: const Key('rename_dialog_field'),
        controller: _controller,
        autofocus: true,
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.pop(context),
          child: const Text('取消'),
        ),
        FilledButton(
          onPressed: trimmed.isEmpty
              ? null
              : () => Navigator.pop(context, _controller.text.trim()),
          child: const Text('保存'),
        ),
      ],
    );
  }
}

Future<String?> showRenameDialogResult(
  BuildContext context, {
  required String initialTitle,
}) {
  return showDialog<String>(
    context: context,
    builder: (_) => RenameDialog(initialTitle: initialTitle),
  );
}
