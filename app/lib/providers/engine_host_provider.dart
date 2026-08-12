import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:path_provider/path_provider.dart';
import 'package:video_sniffing/engine/engine_host.dart';
import 'package:video_sniffing/providers/engine_repository.dart';

final engineHostProvider = FutureProvider<EngineHost>((ref) async {
  final dir = await getApplicationSupportDirectory();
  final host = await EngineHost.open(dir.path);
  ref.onDispose(host.dispose);
  return host;
});

final engineRepositoryProvider = Provider<EngineRepository>((ref) {
  final host = ref.watch(engineHostProvider).requireValue;
  return EngineHostRepository(host);
});
