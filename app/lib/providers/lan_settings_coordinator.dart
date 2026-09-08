import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:video_sniffing/providers/engine_host_provider.dart';
import 'package:video_sniffing/providers/settings_provider.dart';

Future<void> applyLanSettings(WidgetRef ref, {required bool isReceiver}) async {
  final repo = ref.read(engineRepositoryProvider);
  final enabled = ref.read(settingsProvider).lanEnabled;
  if (!enabled) {
    repo.stopLan();
    return;
  }
  repo.applyLanSettings(isReceiver: isReceiver);
  if (isReceiver) {
    repo.beginPairing();
  }
}
