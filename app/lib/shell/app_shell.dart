import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:video_sniffing/providers/device_profile.dart';

const kAppShellBreakpoint = 600.0;

/// 「添加」分支在 [StatefulShellRoute] 中的索引（仅文档/测试引用）。
const kAddShellBranchIndex = 3;

class AppShell extends ConsumerWidget {
  const AppShell({super.key, required this.navigationShell});

  final StatefulNavigationShell navigationShell;

  static const _browseBranchIndex = 1;

  static const _destinations = [
    NavigationDestination(icon: Icon(Icons.video_library), label: '片库'),
    NavigationDestination(icon: Icon(Icons.travel_explore), label: '浏览'),
    NavigationDestination(icon: Icon(Icons.download), label: '任务'),
    NavigationDestination(icon: Icon(Icons.add_link), label: '添加'),
    NavigationDestination(icon: Icon(Icons.settings), label: '设置'),
  ];

  List<NavigationDestination> _visibleDestinations(bool isTelevision) {
    if (!isTelevision) {
      return _destinations;
    }
    return [_destinations[0], ..._destinations.skip(_browseBranchIndex + 1)];
  }

  int _branchIndexFor(int destinationIndex, bool isTelevision) {
    if (!isTelevision || destinationIndex == 0) {
      return destinationIndex;
    }
    return destinationIndex + 1;
  }

  int _destinationIndexFor(int branchIndex, bool isTelevision) {
    if (!isTelevision) {
      return branchIndex;
    }
    if (branchIndex <= _browseBranchIndex) {
      return 0;
    }
    return branchIndex - 1;
  }

  void _onTap(int destinationIndex, bool isTelevision) {
    final branchIndex = _branchIndexFor(destinationIndex, isTelevision);
    navigationShell.goBranch(
      branchIndex,
      initialLocation: branchIndex == navigationShell.currentIndex,
    );
  }

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final isTelevision = ref.watch(isTelevisionProvider).value ?? false;
    final destinations = _visibleDestinations(isTelevision);
    final selectedIndex = _destinationIndexFor(
      navigationShell.currentIndex,
      isTelevision,
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
