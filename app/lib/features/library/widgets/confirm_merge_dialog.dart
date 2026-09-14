import 'package:flutter/material.dart';

class ConfirmMergeDialog extends StatefulWidget {
  const ConfirmMergeDialog({
    super.key,
    required this.targetTitle,
    required this.episodeCount,
    this.deleteOrphanFilesDefault = false,
  });

  final String targetTitle;
  final int episodeCount;
  final bool deleteOrphanFilesDefault;

  @override
  State<ConfirmMergeDialog> createState() => _ConfirmMergeDialogState();
}

class _ConfirmMergeDialogState extends State<ConfirmMergeDialog> {
  late bool _deleteOrphanFiles;

  @override
  void initState() {
    super.initState();
    _deleteOrphanFiles = widget.deleteOrphanFilesDefault;
  }

  @override
  Widget build(BuildContext context) {
    return AlertDialog(
      title: Text('合并到「${widget.targetTitle}」？'),
      content: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(
            '将把本条目下 ${widget.episodeCount} 个分集移至目标；本条目将被删除。'
            '若集号重复，保留目标分集及播放进度。',
          ),
          CheckboxListTile(
            value: _deleteOrphanFiles,
            onChanged: (v) => setState(() => _deleteOrphanFiles = v ?? false),
            title: const Text('删除被丢弃分集的本地缓存文件'),
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
            (confirmed: true, deleteOrphanFiles: _deleteOrphanFiles),
          ),
          child: const Text('合并'),
        ),
      ],
    );
  }
}

Future<({bool confirmed, bool deleteOrphanFiles})?> showConfirmMergeDialogResult(
  BuildContext context, {
  required String targetTitle,
  required int episodeCount,
  bool deleteOrphanFilesDefault = false,
}) {
  return showDialog<({bool confirmed, bool deleteOrphanFiles})>(
    context: context,
    builder: (_) => ConfirmMergeDialog(
      targetTitle: targetTitle,
      episodeCount: episodeCount,
      deleteOrphanFilesDefault: deleteOrphanFilesDefault,
    ),
  );
}
