import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:video_sniffing/features/tv/tv_library_grid.dart';
import 'package:video_sniffing/providers/device_profile.dart';
import 'package:video_sniffing/shell/shell_destinations.dart';

class TvShell extends ConsumerWidget {
  const TvShell({super.key, required this.navigationShell});

  final StatefulNavigationShell navigationShell;

  bool _isLibraryRoot(BuildContext context) {
    return GoRouterState.of(context).uri.path == '/library';
  }

  Widget _buildBody(BuildContext context) {
    if (navigationShell.currentIndex == 0 && _isLibraryRoot(context)) {
      return const TvLibraryGrid();
    }
    return navigationShell;
  }

  void _onTap(int destinationIndex) {
    final branchIndex = shellBranchIndexForDestination(
      destinationIndex,
      isTelevision: true,
    );
    navigationShell.goBranch(
      branchIndex,
      initialLocation: branchIndex == navigationShell.currentIndex,
    );
  }

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    ref.watch(isTelevisionProvider);
    final destinations = visibleShellDestinations(isTelevision: true);
    final selectedIndex = shellDestinationIndexForBranch(
      navigationShell.currentIndex,
      isTelevision: true,
    );

    return Shortcuts(
      shortcuts: const {
        SingleActivator(LogicalKeyboardKey.select): ActivateIntent(),
        SingleActivator(LogicalKeyboardKey.enter): ActivateIntent(),
      },
      child: Actions(
        actions: {
          ActivateIntent: CallbackAction<ActivateIntent>(
            onInvoke: (_) {
              final focused = FocusManager.instance.primaryFocus;
              if (focused != null && focused.context != null) {
                Actions.invoke(focused.context!, const ActivateIntent());
              }
              return null;
            },
          ),
        },
        child: Scaffold(
          body: Row(
            children: [
              NavigationRail(
                selectedIndex: selectedIndex,
                onDestinationSelected: _onTap,
                labelType: NavigationRailLabelType.all,
                destinations: destinations
                    .map(
                      (destination) => NavigationRailDestination(
                        icon: destination.icon,
                        label: Text(destination.label),
                      ),
                    )
                    .toList(),
              ),
              const VerticalDivider(width: 1),
              Expanded(child: _buildBody(context)),
            ],
          ),
        ),
      ),
    );
  }
}
