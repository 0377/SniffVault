import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:video_sniffing/engine/models/library_episode.dart';
import 'package:video_sniffing/features/cast/cast_destination_sheet.dart';
import 'package:video_sniffing/providers/settings_provider.dart';

const lanDisabledMessage = '请先在设置中开启局域网投送';

bool isEpisodeCastable(LibraryEpisode episode) {
  if (episode.filePath.isEmpty) {
    return false;
  }
  return File(episode.filePath).existsSync();
}

void requestCast(BuildContext context, WidgetRef ref, String episodeId) {
  final lanEnabled = ref.read(settingsProvider).lanEnabled;
  if (!lanEnabled) {
    ScaffoldMessenger.of(context).showSnackBar(
      const SnackBar(content: Text(lanDisabledMessage)),
    );
    return;
  }
  showCastDestinationSheet(context, episodeId: episodeId);
}
