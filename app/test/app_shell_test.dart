import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:go_router/go_router.dart';
import 'package:video_sniffing/features/browse/browse_screen.dart';
import 'package:video_sniffing/providers/device_profile.dart';
import 'package:video_sniffing/shell/app_shell.dart';

final _rootNavigatorKey = GlobalKey<NavigatorState>();

GoRouter _testRouter({String initialLocation = '/library'}) {
  return GoRouter(
    navigatorKey: _rootNavigatorKey,
    initialLocation: initialLocation,
    routes: [
      StatefulShellRoute.indexedStack(
        builder: (context, state, navigationShell) =>
            AppShell(navigationShell: navigationShell),
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

Widget _shellApp({
  required Size size,
  List<Override> overrides = const [],
  String location = '/library',
}) {
  return ProviderScope(
    overrides: overrides,
    child: MediaQuery(
      data: MediaQueryData(size: size),
      child: MaterialApp.router(
        routerConfig: _testRouter(initialLocation: location),
      ),
    ),
  );
}

void main() {
  testWidgets('W3 uses NavigationBar when width < 600', (tester) async {
    await tester.pumpWidget(_shellApp(size: const Size(400, 800)));
    await tester.pumpAndSettle();
    expect(find.byType(NavigationBar), findsOneWidget);
    expect(find.byType(NavigationRail), findsNothing);
  });

  testWidgets('W3 uses NavigationRail when width >= 600', (tester) async {
    await tester.pumpWidget(_shellApp(size: const Size(800, 800)));
    await tester.pumpAndSettle();
    expect(find.byType(NavigationRail), findsOneWidget);
    expect(find.byType(NavigationBar), findsNothing);
  });

  testWidgets('W3 shows 浏览 destination when not television', (tester) async {
    await tester.pumpWidget(_shellApp(size: const Size(400, 800)));
    await tester.pumpAndSettle();
    expect(find.text('浏览'), findsOneWidget);
    expect(find.text('片库'), findsOneWidget);
    expect(find.text('任务'), findsOneWidget);
    expect(find.text('添加'), findsOneWidget);
    expect(find.text('设置'), findsOneWidget);
  });

  testWidgets('W3 hides 浏览 destination on television', (tester) async {
    await tester.pumpWidget(
      _shellApp(
        size: const Size(400, 800),
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

  testWidgets('TV destination taps skip the hidden browse branch', (
    tester,
  ) async {
    await tester.pumpWidget(
      _shellApp(
        size: const Size(400, 800),
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

  testWidgets('TV /browse shows unavailable screen without WebView', (
    tester,
  ) async {
    await tester.pumpWidget(
      _shellApp(
        size: const Size(400, 800),
        location: '/browse',
        overrides: [isTelevisionProvider.overrideWith((ref) async => true)],
      ),
    );
    await tester.pumpAndSettle();
    expect(find.text('请在手机或电脑使用内置浏览'), findsOneWidget);
    expect(find.text('浏览'), findsNothing);
  });

  testWidgets('/browse/wizard uses the root navigator', (tester) async {
    await tester.pumpWidget(
      _shellApp(size: const Size(400, 800), location: '/browse/wizard'),
    );
    await tester.pumpAndSettle();
    expect(find.text('wizard'), findsOneWidget);
    expect(find.byType(NavigationBar), findsNothing);
  });
}
