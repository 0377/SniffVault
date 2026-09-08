import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:go_router/go_router.dart';
import 'package:video_sniffing/engine/models/resolve_types.dart';
import 'package:video_sniffing/features/add/add_screen.dart';
import 'package:video_sniffing/providers/device_profile.dart';
import 'package:video_sniffing/providers/engine_host_provider.dart';
import 'package:video_sniffing/providers/settings_provider.dart';

import 'fakes/fake_engine_repository.dart';

class _NeedsBrowserRepo extends FakeEngineRepository {
  @override
  Future<ResolveOutcome> resolveUrl(String url, {ResolveOptions? opts}) async {
    lastResolveOpts = opts;
    return const ResolveOutcomeNeedsBrowser(reason: 'auth_required');
  }
}

GoRouter _router() {
  return GoRouter(
    initialLocation: '/add',
    routes: [
      GoRoute(
        path: '/add',
        builder: (_, _) =>
            const AddScreen(initialUrl: 'https://example.com/watch'),
      ),
      GoRoute(
        path: '/browse',
        builder: (_, state) =>
            Text('browse url=${state.uri.queryParameters['url']}'),
      ),
      GoRoute(path: '/tasks', builder: (_, _) => const Text('tasks')),
    ],
  );
}

Widget _app({
  required FakeEngineRepository fake,
  List<Override> extraOverrides = const [],
}) {
  return ProviderScope(
    overrides: [
      engineRepositoryProvider.overrideWithValue(fake),
      settingsProvider.overrideWith((ref) => fake.settings()),
      ...extraOverrides,
    ],
    child: MaterialApp.router(routerConfig: _router()),
  );
}

Future<void> _resolve(WidgetTester tester) async {
  await tester.tap(find.byKey(const Key('add_resolve_button')));
  await tester.pumpAndSettle();
}

void main() {
  testWidgets('NeedsBrowser opens encoded browse url when not television', (
    tester,
  ) async {
    await tester.pumpWidget(_app(fake: _NeedsBrowserRepo()));
    await tester.pumpAndSettle();
    await _resolve(tester);

    expect(find.text('打开内置浏览'), findsOneWidget);
    await tester.tap(find.text('打开内置浏览'));
    await tester.pumpAndSettle();
    expect(find.text('browse url=https://example.com/watch'), findsOneWidget);
  });

  testWidgets('NeedsBrowser hides open browse on television', (tester) async {
    await tester.pumpWidget(
      _app(
        fake: _NeedsBrowserRepo(),
        extraOverrides: [
          isTelevisionProvider.overrideWith((ref) async => true),
        ],
      ),
    );
    await tester.pumpAndSettle();
    await _resolve(tester);

    expect(find.textContaining('需要在内置浏览中打开'), findsOneWidget);
    expect(find.text('打开内置浏览'), findsNothing);
    expect(find.text('返回'), findsOneWidget);
  });
}
