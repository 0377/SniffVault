import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:go_router/go_router.dart';
import 'package:video_sniffing/features/browse/browse_screen.dart';
import 'package:video_sniffing/providers/device_profile.dart';
import 'package:video_sniffing/providers/engine_host_provider.dart';
import 'package:video_sniffing/providers/engine_repository.dart';
import 'package:video_sniffing/shell/shell_selector.dart';

import 'fakes/fake_engine_repository.dart';
import 'fakes/fake_ready_engine_host.dart';

final _rootNavigatorKey = GlobalKey<NavigatorState>();

GoRouter _testRouter({String initialLocation = '/library'}) {
  return GoRouter(
    navigatorKey: _rootNavigatorKey,
    initialLocation: initialLocation,
    routes: [
      StatefulShellRoute.indexedStack(
        builder: (context, state, navigationShell) =>
            ShellSelector(navigationShell: navigationShell),
        branches: [
          StatefulShellBranch(
            routes: [
              GoRoute(
                path: '/library',
                builder: (_, _) => const Text('library'),
              ),
            ],
          ),
          StatefulShellBranch(
            routes: [
              GoRoute(
                path: '/browse',
                builder: (_, _) => const BrowseScreen(),
                routes: [
                  GoRoute(
                    parentNavigatorKey: _rootNavigatorKey,
                    path: 'wizard',
                    builder: (_, _) => const Text('wizard'),
                  ),
                ],
              ),
            ],
          ),
          StatefulShellBranch(
            routes: [
              GoRoute(path: '/tasks', builder: (_, _) => const Text('tasks')),
            ],
          ),
          StatefulShellBranch(
            routes: [
              GoRoute(path: '/add', builder: (_, _) => const Text('add')),
            ],
          ),
          StatefulShellBranch(
            routes: [
              GoRoute(
                path: '/settings',
                builder: (_, _) => const Text('settings'),
              ),
            ],
          ),
        ],
      ),
    ],
  );
}

List<Override> _engineOverrides() => [
  engineHostProvider.overrideWith((ref) async {
    final host = FakeReadyEngineHost();
    ref.onDispose(host.dispose);
    return host;
  }),
  engineRepositoryProvider.overrideWithValue(FakeEngineRepository()),
];

Widget _shellApp({
  required Size size,
  List<Override> overrides = const [],
  String location = '/library',
}) {
  return ProviderScope(
    overrides: [..._engineOverrides(), ...overrides],
    child: MediaQuery(
      data: MediaQueryData(size: size),
      child: MaterialApp.router(
        routerConfig: _testRouter(initialLocation: location),
      ),
    ),
  );
}

void main() {
  testWidgets('W16 ShellSelector on TV hides browse tab', (tester) async {
    await tester.pumpWidget(
      _shellApp(
        size: const Size(800, 800),
        overrides: [isTelevisionProvider.overrideWith((ref) async => true)],
      ),
    );
    await tester.pumpAndSettle();
    expect(find.text('浏览'), findsNothing);
    expect(find.text('片库'), findsOneWidget);
    expect(find.text('任务'), findsOneWidget);
    expect(find.text('添加'), findsOneWidget);
    expect(find.text('设置'), findsOneWidget);
  });

  testWidgets('ShellSelector on TV destination taps skip browse branch', (
    tester,
  ) async {
    await tester.pumpWidget(
      _shellApp(
        size: const Size(800, 800),
        overrides: [isTelevisionProvider.overrideWith((ref) async => true)],
      ),
    );
    await tester.pumpAndSettle();
    await tester.tap(find.text('任务'));
    await tester.pumpAndSettle();
    expect(find.text('tasks'), findsOneWidget);
    await tester.tap(find.text('添加'));
    await tester.pumpAndSettle();
    expect(find.text('add'), findsOneWidget);
    await tester.tap(find.text('设置'));
    await tester.pumpAndSettle();
    expect(find.text('settings'), findsOneWidget);
  });

  testWidgets('ShellSelector on non-TV still shows browse tab', (tester) async {
    await tester.pumpWidget(_shellApp(size: const Size(400, 800)));
    await tester.pumpAndSettle();
    expect(find.text('浏览'), findsOneWidget);
  });
}
