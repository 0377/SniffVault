import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:video_sniffing/engine/models/engine_settings.dart';
import 'package:video_sniffing/providers/engine_host_provider.dart';
import 'package:video_sniffing/providers/engine_repository.dart';

final settingsProvider = Provider<EngineSettings>((ref) {
  ref.watch(engineHostProvider);
  return ref.watch(engineRepositoryProvider).settings();
});
