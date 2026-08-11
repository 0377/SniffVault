import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:video_sniffing/engine/models/library_item.dart';
import 'package:video_sniffing/providers/engine_host_provider.dart';
import 'package:video_sniffing/providers/engine_repository.dart';

final libraryProvider = Provider<List<LibraryItem>>((ref) {
  ref.watch(engineHostProvider);
  final repo = ref.watch(engineRepositoryProvider);
  final items = [...repo.listLibrary()]
    ..sort((a, b) => b.createdAtMs.compareTo(a.createdAtMs));
  return items;
});
