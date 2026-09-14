import 'package:flutter/material.dart';
import 'package:video_sniffing/engine/models/library_item.dart';

class MergeTargetPickerSheet extends StatelessWidget {
  const MergeTargetPickerSheet({super.key, required this.candidates});

  final List<LibraryItem> candidates;

  @override
  Widget build(BuildContext context) {
    return SafeArea(
      child: Padding(
        padding: const EdgeInsets.fromLTRB(16, 16, 16, 24),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Text(
              '选择合并目标',
              style: Theme.of(context).textTheme.titleLarge,
            ),
            const SizedBox(height: 16),
            Flexible(
              child: ListView.separated(
                shrinkWrap: true,
                itemCount: candidates.length,
                separatorBuilder: (_, _) => const Divider(height: 1),
                itemBuilder: (context, index) {
                  final candidate = candidates[index];
                  return ListTile(
                    key: Key('merge_target_${candidate.id}'),
                    title: Text(candidate.title),
                    subtitle: candidate.season != null
                        ? Text('第 ${candidate.season} 季')
                        : null,
                    onTap: () => Navigator.pop(context, candidate),
                  );
                },
              ),
            ),
          ],
        ),
      ),
    );
  }
}

Future<LibraryItem?> showMergeTargetPicker(
  BuildContext context, {
  required List<LibraryItem> candidates,
}) {
  return showModalBottomSheet<LibraryItem>(
    context: context,
    isScrollControlled: true,
    builder: (_) => MergeTargetPickerSheet(candidates: candidates),
  );
}
