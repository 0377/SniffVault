import 'package:flutter/material.dart';
import 'package:video_sniffing/engine/models/resolve_types.dart';

class QualityPicker extends StatelessWidget {
  const QualityPicker({
    super.key,
    required this.qualities,
    required this.selected,
    required this.onSelected,
  });

  final List<Quality> qualities;
  final Quality? selected;
  final ValueChanged<Quality> onSelected;

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        const Text('选择清晰度'),
        const SizedBox(height: 8),
        if (qualities.isEmpty)
          const Padding(
            padding: EdgeInsets.symmetric(vertical: 8),
            child: LinearProgressIndicator(),
          )
        else
          ...qualities.map(
            (quality) => RadioListTile<Quality>(
              title: Text(quality.label),
              value: quality,
              groupValue: selected,
              onChanged: (value) {
                if (value != null) {
                  onSelected(value);
                }
              },
            ),
          ),
      ],
    );
  }
}
