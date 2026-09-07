import 'package:flutter/material.dart';
import 'package:video_sniffing/engine/models/resolve_types.dart';

class EpisodeMultiSelect extends StatefulWidget {
  const EpisodeMultiSelect({
    super.key,
    required this.episodeList,
    required this.onSelectionChanged,
  });

  final EpisodeList episodeList;
  final ValueChanged<List<Episode>> onSelectionChanged;

  @override
  State<EpisodeMultiSelect> createState() => _EpisodeMultiSelectState();
}

class _EpisodeMultiSelectState extends State<EpisodeMultiSelect> {
  late Set<int> _selectedIndices;

  @override
  void initState() {
    super.initState();
    _selectedIndices = widget.episodeList.episodes.map((e) => e.index).toSet();
    WidgetsBinding.instance.addPostFrameCallback((_) => _notifySelection());
  }

  void _notifySelection() {
    final selected = widget.episodeList.episodes
        .where((episode) => _selectedIndices.contains(episode.index))
        .toList();
    widget.onSelectionChanged(selected);
  }

  void _toggle(int index, bool? checked) {
    setState(() {
      if (checked == true) {
        _selectedIndices.add(index);
      } else {
        _selectedIndices.remove(index);
      }
    });
    _notifySelection();
  }

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        const Text('选择分集'),
        const SizedBox(height: 8),
        ...widget.episodeList.episodes.map(
          (episode) => CheckboxListTile(
            title: Text(episode.title),
            subtitle: Text('第 ${episode.index} 集'),
            value: _selectedIndices.contains(episode.index),
            onChanged: (checked) => _toggle(episode.index, checked),
          ),
        ),
      ],
    );
  }
}
