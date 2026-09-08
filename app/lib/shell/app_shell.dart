import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:video_sniffing/providers/device_profile.dart';
import 'package:video_sniffing/shell/shell_destinations.dart';

export 'shell_destinations.dart' show kAddShellBranchIndex;

const kAppShellBreakpoint = 600.0;

class AppShell extends ConsumerWidget {
  const AppShell({super.key, required this.navigationShell});

  final StatefulNavigationShell navigationShell;

  void _onTap(int destinationIndex, bool isTelevision) {
    final branchIndex = shellBranchIndexForDestination(
      destinationIndex,
      isTelevision: isTelevision,
    );
    navigationShell.goBranch(
      branchIndex,
      initialLocation: branchIndex == navigationShell.currentIndex,
    );
  }

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final isTelevision = ref.watch(isTelevisionProvider).value ?? false;
    final destinations = visibleShellDestinations(isTelevision: isTelevision);
    final selectedIndex = shellDestinationIndexForBranch(
      navigationShell.currentIndex,
      isTelevision: isTelevision,
    );
    final useRail = MediaQuery.sizeOf(context).width >= kAppShellBreakpoint;
    final body = navigationShell;
    if (useRail) {
      return Scaffold(
        body: Row(
          children: [
            NavigationRail(
              selectedIndex: selectedIndex,
              onDestinationSelected: (index) => _onTap(index, isTelevision),
              labelType: NavigationRailLabelType.all,
              destinations: destinations
                  .map(
                    (d) => NavigationRailDestination(
                      icon: d.icon,
                      label: Text(d.label),
                    ),
                  )
                  .toList(),
            ),
            const VerticalDivider(width: 1),
            Expanded(child: body),
          ],
        ),
      );
    }
    return Scaffold(
      body: body,
      bottomNavigationBar: NavigationBar(
        selectedIndex: selectedIndex,
        onDestinationSelected: (index) => _onTap(index, isTelevision),
        destinations: destinations,
      ),
    );
  }
}
