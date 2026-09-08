import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:video_sniffing/engine/models/library_item.dart';
import 'package:video_sniffing/engine/models/library_item_kind.dart';
import 'package:video_sniffing/providers/library_provider.dart';

const _itemKeyPrefix = 'tv_library_item_';

class TvLibraryGrid extends ConsumerWidget {
  const TvLibraryGrid({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final items = ref.watch(libraryProvider);

    if (items.isEmpty) {
      return const Scaffold(
        body: Center(child: Text('片库为空，去「添加」粘贴 URL')),
      );
    }

    final width = MediaQuery.sizeOf(context).width;
    final crossAxisCount = width >= 1200
        ? 4
        : width >= 800
        ? 3
        : 2;

    return Scaffold(
      body: GridView.builder(
        padding: const EdgeInsets.all(24),
        gridDelegate: SliverGridDelegateWithFixedCrossAxisCount(
          crossAxisCount: crossAxisCount,
          crossAxisSpacing: 16,
          mainAxisSpacing: 16,
          childAspectRatio: 0.72,
        ),
        itemCount: items.length,
        itemBuilder: (context, index) {
          final item = items[index];
          return TvLibraryGridTile(
            key: Key('$_itemKeyPrefix$index'),
            item: item,
            autofocus: index == 0,
            onSelect: () => context.push('/library/${item.id}'),
          );
        },
      ),
    );
  }
}

class TvLibraryGridTile extends StatefulWidget {
  const TvLibraryGridTile({
    super.key,
    required this.item,
    required this.onSelect,
    this.autofocus = false,
  });

  final LibraryItem item;
  final VoidCallback onSelect;
  final bool autofocus;

  @override
  State<TvLibraryGridTile> createState() => _TvLibraryGridTileState();
}

class _TvLibraryGridTileState extends State<TvLibraryGridTile> {
  bool _focused = false;

  KeyEventResult _handleKey(FocusNode node, KeyEvent event) {
    if (event is! KeyDownEvent) {
      return KeyEventResult.ignored;
    }
    if (event.logicalKey == LogicalKeyboardKey.select ||
        event.logicalKey == LogicalKeyboardKey.enter ||
        event.logicalKey == LogicalKeyboardKey.space) {
      widget.onSelect();
      return KeyEventResult.handled;
    }
    return KeyEventResult.ignored;
  }

  @override
  Widget build(BuildContext context) {
    final seasonLabel =
        widget.item.kind == LibraryItemKind.series && widget.item.season != null
        ? '第 ${widget.item.season} 季'
        : null;

    return Focus(
      autofocus: widget.autofocus,
      onFocusChange: (focused) => setState(() => _focused = focused),
      onKeyEvent: _handleKey,
      child: GestureDetector(
        onTap: widget.onSelect,
        child: AnimatedScale(
          scale: _focused ? 1.08 : 1.0,
          duration: const Duration(milliseconds: 150),
          child: Card(
            clipBehavior: Clip.antiAlias,
            child: ConstrainedBox(
              constraints: const BoxConstraints(minHeight: 48, minWidth: 48),
              child: Padding(
                padding: const EdgeInsets.all(12),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Expanded(
                      child: DecoratedBox(
                        decoration: BoxDecoration(
                          color: Theme.of(
                            context,
                          ).colorScheme.surfaceContainerHighest,
                          borderRadius: BorderRadius.circular(8),
                        ),
                        child: Center(
                          child: Icon(
                            Icons.movie_outlined,
                            size: 48,
                            color: Theme.of(context).colorScheme.primary,
                          ),
                        ),
                      ),
                    ),
                    const SizedBox(height: 8),
                    Text(
                      widget.item.title,
                      maxLines: 2,
                      overflow: TextOverflow.ellipsis,
                      style: Theme.of(context).textTheme.titleMedium,
                    ),
                    if (seasonLabel != null) ...[
                      const SizedBox(height: 4),
                      Text(
                        seasonLabel,
                        style: Theme.of(context).textTheme.bodySmall,
                      ),
                    ],
                  ],
                ),
              ),
            ),
          ),
        ),
      ),
    );
  }
}
