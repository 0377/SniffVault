import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:video_sniffing/features/library/widgets/library_card.dart';
import 'package:video_sniffing/providers/library_provider.dart';

class LibraryScreen extends ConsumerWidget {
  const LibraryScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final items = ref.watch(libraryProvider);

    if (items.isEmpty) {
      return const Scaffold(
        body: Center(child: Text('片库为空，去「添加」粘贴 URL')),
      );
    }

    return Scaffold(
      body: ListView.builder(
        key: const Key('library_list'),
        itemCount: items.length,
        itemBuilder: (context, index) {
          final item = items[index];
          return LibraryCard(
            item: item,
            onTap: () => context.push('/library/${item.id}'),
          );
        },
      ),
    );
  }
}
