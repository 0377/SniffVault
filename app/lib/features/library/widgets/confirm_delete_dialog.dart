import 'package:flutter/material.dart';

class ConfirmDeleteDialog extends StatefulWidget {
  const ConfirmDeleteDialog({
    super.key,
    required this.title,
    required this.message,
    this.deleteFilesDefault = true,
  });

  final String title;
  final String message;
  final bool deleteFilesDefault;

  @override
  State<ConfirmDeleteDialog> createState() => _ConfirmDeleteDialogState();
}

class _ConfirmDeleteDialogState extends State<ConfirmDeleteDialog> {
  late bool _deleteFiles;

  @override
  void initState() {
    super.initState();
    _deleteFiles = widget.deleteFilesDefault;
  }

  @override
  Widget build(BuildContext context) {
    return AlertDialog(
      title: Text(widget.title),
      content: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(widget.message),
          CheckboxListTile(
            value: _deleteFiles,
            onChanged: (v) => setState(() => _deleteFiles = v ?? true),
            title: const Text('同时删除本地缓存文件'),
            controlAffinity: ListTileControlAffinity.leading,
            contentPadding: EdgeInsets.zero,
          ),
        ],
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.pop(context),
          child: const Text('取消'),
        ),
        FilledButton(
          onPressed: () => Navigator.pop(
            context,
            (confirmed: true, deleteFiles: _deleteFiles),
          ),
          child: const Text('删除'),
        ),
      ],
    );
  }
}

Future<({bool confirmed, bool deleteFiles})?> showConfirmDeleteDialogResult(
  BuildContext context, {
  required String title,
  required String message,
  bool deleteFilesDefault = true,
}) {
  return showDialog<({bool confirmed, bool deleteFiles})>(
    context: context,
    builder: (_) => ConfirmDeleteDialog(
      title: title,
      message: message,
      deleteFilesDefault: deleteFilesDefault,
    ),
  );
}
