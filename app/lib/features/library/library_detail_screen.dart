import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:video_sniffing/engine/models/library_episode.dart';
import 'package:video_sniffing/engine/models/library_item.dart';
import 'package:video_sniffing/engine/models/library_item_kind.dart';
import 'package:video_sniffing/features/library/widgets/episode_tile.dart';
import 'package:video_sniffing/providers/engine_host_provider.dart';
import 'package:video_sniffing/providers/library_provider.dart';

class LibraryDetailScreen extends ConsumerWidget {
  const LibraryDetailScreen({super.key, required this.itemId});

  final String itemId;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final items = ref.watch(libraryProvider);
    LibraryItem? item;
    for (final candidate in items) {
      if (candidate.id == itemId) {
        item = candidate;
        break;
      }
    }

    final repo = ref.read(engineRepositoryProvider);
    final episodes = List<LibraryEpisode>.from(repo.listEpisodes(itemId))
      ..sort((a, b) => a.index.compareTo(b.index));

    return Scaffold(
      appBar: AppBar(
        title: Text(item?.title ?? '片库详情'),
      ),
      body: _buildBody(context, item, episodes),
    );
  }

  Widget _buildBody(
    BuildContext context,
    LibraryItem? item,
    List<LibraryEpisode> episodes,
  ) {
    if (episodes.isEmpty) {
      return const Center(child: Text('暂无分集'));
    }

    final isSingle = item?.kind == LibraryItemKind.single || episodes.length == 1;
    if (isSingle && episodes.length == 1) {
      final episode = episodes.first;
      return Center(
        child: FilledButton.icon(
          onPressed: () => context.push('/play/${episode.id}'),
          icon: const Icon(Icons.play_arrow),
          label: const Text('播放'),
        ),
      );
    }

    return ListView.builder(
      itemCount: episodes.length,
      itemBuilder: (context, index) {
        final episode = episodes[index];
        return EpisodeTile(
          episode: episode,
          onTap: () => context.push('/play/${episode.id}'),
        );
      },
    );
  }
}
