import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:video_sniffing/engine/models/library_episode.dart';
import 'package:video_sniffing/engine/models/library_item.dart';
import 'package:video_sniffing/engine/models/library_item_kind.dart';
import 'package:video_sniffing/features/cast/cast_actions.dart';
import 'package:video_sniffing/features/library/widgets/confirm_delete_dialog.dart';
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

    return Shortcuts(
      shortcuts: const {
        SingleActivator(LogicalKeyboardKey.contextMenu): ActivateIntent(),
      },
      child: Actions(
        actions: {
          ActivateIntent: CallbackAction<ActivateIntent>(
            onInvoke: (_) {
              if (item != null) {
                _confirmDeleteItem(context, ref, item, episodes);
              }
              return null;
            },
          ),
        },
        child: Focus(
          autofocus: true,
          child: Scaffold(
            appBar: AppBar(
              title: Text(item?.title ?? '片库详情'),
              actions: [
                if (item != null)
                  PopupMenuButton<String>(
                    key: const Key('library_detail_menu'),
                    onSelected: (value) =>
                        _onMenuSelected(context, ref, value, item!, episodes),
                    itemBuilder: (_) => const [
                      PopupMenuItem(value: 'delete', child: Text('删除')),
                    ],
                  ),
              ],
            ),
            body: _buildBody(context, ref, item, episodes),
          ),
        ),
      ),
    );
  }

  Widget _buildBody(
    BuildContext context,
    WidgetRef ref,
    LibraryItem? item,
    List<LibraryEpisode> episodes,
  ) {
    if (episodes.isEmpty) {
      return const Center(child: Text('暂无分集'));
    }

    final isSingle = item?.kind == LibraryItemKind.single || episodes.length == 1;
    if (isSingle && episodes.length == 1) {
      final episode = episodes.first;
      final canCast = isEpisodeCastable(episode);
      return Center(
        child: Wrap(
          alignment: WrapAlignment.center,
          spacing: 12,
          runSpacing: 12,
          children: [
            FilledButton.icon(
              onPressed: () => context.push('/play/${episode.id}'),
              icon: const Icon(Icons.play_arrow),
              label: const Text('播放'),
            ),
            if (canCast)
              OutlinedButton.icon(
                key: const Key('library_detail_cast'),
                onPressed: () => requestCast(context, ref, episode.id),
                icon: const Icon(Icons.cast),
                label: const Text('投送'),
              ),
          ],
        ),
      );
    }

    final canDeleteEpisode =
        item?.kind == LibraryItemKind.series && episodes.length >= 2;

    return ListView.builder(
      itemCount: episodes.length,
      itemBuilder: (context, index) {
        final episode = episodes[index];
        final canCast = isEpisodeCastable(episode);
        return EpisodeTile(
          episode: episode,
          onTap: () => context.push('/play/${episode.id}'),
          onCast: canCast
              ? () => requestCast(context, ref, episode.id)
              : null,
          onDelete: canDeleteEpisode
              ? () => _confirmDeleteEpisode(context, ref, episode)
              : null,
        );
      },
    );
  }
}

Future<void> _onMenuSelected(
  BuildContext context,
  WidgetRef ref,
  String value,
  LibraryItem item,
  List<LibraryEpisode> episodes,
) async {
  if (value == 'delete') {
    await _confirmDeleteItem(context, ref, item, episodes);
  }
}

Future<void> _confirmDeleteItem(
  BuildContext context,
  WidgetRef ref,
  LibraryItem item,
  List<LibraryEpisode> episodes,
) async {
  final message = item.kind == LibraryItemKind.series
      ? '将删除 ${episodes.length} 个分集'
      : '将删除 1 个文件';
  final result = await showConfirmDeleteDialogResult(
    context,
    title: '删除「${item.title}」？',
    message: message,
  );
  if (result == null || !result.confirmed) {
    return;
  }

  final repo = ref.read(engineRepositoryProvider);
  repo.removeLibraryItem(item.id, deleteFiles: result.deleteFiles);
  ref.invalidate(libraryProvider);

  if (!context.mounted) {
    return;
  }
  ScaffoldMessenger.of(context).showSnackBar(
    const SnackBar(content: Text('已删除')),
  );
  context.pop();
}

Future<void> _confirmDeleteEpisode(
  BuildContext context,
  WidgetRef ref,
  LibraryEpisode episode,
) async {
  final result = await showConfirmDeleteDialogResult(
    context,
    title: '删除「${episode.title}」？',
    message: '将删除 1 个文件',
  );
  if (result == null || !result.confirmed) {
    return;
  }

  final repo = ref.read(engineRepositoryProvider);
  repo.removeEpisode(episode.id, deleteFiles: result.deleteFiles);
  ref.invalidate(libraryProvider);

  if (!context.mounted) {
    return;
  }
  ScaffoldMessenger.of(context).showSnackBar(
    const SnackBar(content: Text('已删除')),
  );
}
