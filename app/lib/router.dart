import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:video_sniffing/features/add/add_screen.dart';
import 'package:video_sniffing/features/browse/browse_screen.dart';
import 'package:video_sniffing/features/library/library_detail_screen.dart';
import 'package:video_sniffing/features/library/library_screen.dart';
import 'package:video_sniffing/features/player/player_screen.dart';
import 'package:video_sniffing/features/cast/trusted_devices_screen.dart';
import 'package:video_sniffing/features/settings/settings_screen.dart';
import 'package:video_sniffing/features/tasks/tasks_screen.dart';
import 'package:video_sniffing/shell/shell_selector.dart';

final _rootNavigatorKey = GlobalKey<NavigatorState>();

final appRouterProvider = Provider<GoRouter>((ref) {
  return GoRouter(
    navigatorKey: _rootNavigatorKey,
    initialLocation: '/library',
    routes: [
      StatefulShellRoute.indexedStack(
        builder: (context, state, navigationShell) =>
            ShellSelector(navigationShell: navigationShell),
        branches: [
          StatefulShellBranch(
            routes: [
              GoRoute(
                path: '/library',
                builder: (_, __) => const LibraryScreen(),
                routes: [
                  GoRoute(
                    path: ':itemId',
                    builder: (_, state) => LibraryDetailScreen(
                      itemId: state.pathParameters['itemId']!,
                    ),
                  ),
                ],
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
                    builder: (_, _) => const BrowseWizardPage(),
                  ),
                ],
              ),
            ],
          ),
          StatefulShellBranch(
            routes: [
              GoRoute(path: '/tasks', builder: (_, __) => const TasksScreen()),
            ],
          ),
          StatefulShellBranch(
            routes: [
              GoRoute(
                path: '/add',
                builder: (_, state) => AddScreen(
                  initialUrl: state.uri.queryParameters['url'],
                ),
              ),
            ],
          ),
          StatefulShellBranch(
            routes: [
              GoRoute(
                path: '/settings',
                builder: (_, __) => const SettingsScreen(),
                routes: [
                  GoRoute(
                    parentNavigatorKey: _rootNavigatorKey,
                    path: 'trusted-devices',
                    builder: (_, __) => const TrustedDevicesScreen(),
                  ),
                ],
              ),
            ],
          ),
        ],
      ),
      GoRoute(
        parentNavigatorKey: _rootNavigatorKey,
        path: '/play/:episodeId',
        builder: (_, state) => PlayerScreen(
          episodeId: state.pathParameters['episodeId']!,
        ),
      ),
    ],
  );
});
