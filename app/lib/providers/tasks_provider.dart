import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:video_sniffing/engine/models/download_task.dart';
import 'package:video_sniffing/providers/engine_host_provider.dart';
import 'package:video_sniffing/providers/engine_repository.dart';

final tasksProvider = Provider<List<DownloadTask>>((ref) {
  ref.watch(engineHostProvider);
  final repo = ref.watch(engineRepositoryProvider);
  return repo.listTasks();
});
