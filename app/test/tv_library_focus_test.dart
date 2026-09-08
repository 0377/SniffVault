import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:video_sniffing/engine/models/library_item.dart';
import 'package:video_sniffing/engine/models/library_item_kind.dart';
import 'package:video_sniffing/features/tv/tv_library_grid.dart';
import 'package:video_sniffing/providers/engine_host_provider.dart';
import 'package:video_sniffing/providers/engine_repository.dart';
import 'package:video_sniffing/providers/library_provider.dart';

import 'fakes/fake_engine_repository.dart';
import 'fakes/fake_ready_engine_host.dart';

LibraryItem _sampleItem({required String id, required String title}) {
  return LibraryItem(
    id: id,
    kind: LibraryItemKind.single,
    title: title,
    createdAtMs: 1,
  );
}

Widget _gridApp({
  required List<LibraryItem> items,
  Size size = const Size(900, 600),
}) {
  final fake = FakeEngineRepository(libraryItems: items);
  return ProviderScope(
    overrides: [
      engineHostProvider.overrideWith((ref) async {
        final host = FakeReadyEngineHost();
        ref.onDispose(host.dispose);
        return host;
      }),
      engineRepositoryProvider.overrideWithValue(fake),
      libraryProvider.overrideWith((ref) => items),
    ],
    child: MediaQuery(
      data: MediaQueryData(size: size),
      child: const MaterialApp(home: TvLibraryGrid()),
    ),
  );
}

void main() {
  testWidgets('U11 TvLibraryGrid first item is focusable', (tester) async {
    await tester.pumpWidget(
      _gridApp(
        items: [
          _sampleItem(id: 'a', title: 'Alpha'),
          _sampleItem(id: 'b', title: 'Beta'),
        ],
      ),
    );
    await tester.pump();
    await tester.pump();

    final firstItem = find.byKey(const Key('tv_library_item_0'));
    expect(firstItem, findsOneWidget);

    final focusFinder = find.descendant(
      of: firstItem,
      matching: find.byType(Focus),
    );
    expect(focusFinder, findsOneWidget);

    final focusWidget = tester.widget<Focus>(focusFinder);
    expect(focusWidget.autofocus, isTrue);

    final primaryFocus = FocusManager.instance.primaryFocus;
    expect(primaryFocus, isNotNull);
    expect(primaryFocus!.hasFocus, isTrue);
    expect(
      primaryFocus.context!.findAncestorWidgetOfExactType<TvLibraryGridTile>(),
      isNotNull,
    );
  });
}
