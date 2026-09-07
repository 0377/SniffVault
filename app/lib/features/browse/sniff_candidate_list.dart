import 'package:flutter/material.dart';
import 'package:video_sniffing/engine/models/resolve_types.dart';

class SniffCandidateList extends StatelessWidget {
  const SniffCandidateList({
    super.key,
    required this.candidates,
    required this.onSelect,
  });

  final List<ResourceCandidate> candidates;
  final ValueChanged<ResourceCandidate> onSelect;

  @override
  Widget build(BuildContext context) {
    if (candidates.isEmpty) {
      return const SizedBox.shrink();
    }
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Text('嗅探候选 ${candidates.length}'),
        for (final candidate in candidates)
          ListTile(
            title: Text(candidate.title ?? candidate.url),
            onTap: () => onSelect(candidate),
          ),
      ],
    );
  }
}
