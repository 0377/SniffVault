import 'dart:io';

import 'package:flutter/material.dart';

class PosterThumbnail extends StatelessWidget {
  const PosterThumbnail({
    super.key,
    this.posterPath,
    required this.title,
    required this.width,
    required this.height,
  });

  final String? posterPath;
  final String title;
  final double width;
  final double height;

  @override
  Widget build(BuildContext context) {
    final placeholder = _letterAvatar(context, title);
    if (posterPath == null || posterPath!.isEmpty) {
      return placeholder;
    }
    return ClipRRect(
      borderRadius: BorderRadius.circular(8),
      child: Image.file(
        File(posterPath!),
        width: width,
        height: height,
        fit: BoxFit.cover,
        errorBuilder: (_, _, _) => placeholder,
      ),
    );
  }

  Widget _letterAvatar(BuildContext context, String title) {
    final trimmed = title.trim();
    final letter = trimmed.isEmpty ? '?' : trimmed.substring(0, 1);
    return Container(
      width: width,
      height: height,
      alignment: Alignment.center,
      decoration: BoxDecoration(
        color: Theme.of(context).colorScheme.primaryContainer,
        borderRadius: BorderRadius.circular(8),
      ),
      child: Text(
        letter,
        style: Theme.of(context).textTheme.headlineSmall?.copyWith(
          color: Theme.of(context).colorScheme.onPrimaryContainer,
        ),
      ),
    );
  }
}
