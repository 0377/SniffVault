import 'package:flutter/material.dart';

/// 「添加」分支在 [StatefulShellRoute] 中的索引（仅文档/测试引用）。
const kAddShellBranchIndex = 3;

/// 「浏览」分支在 [StatefulShellRoute] 中的索引。
const kBrowseShellBranchIndex = 1;

const kShellDestinations = <NavigationDestination>[
  NavigationDestination(icon: Icon(Icons.video_library), label: '片库'),
  NavigationDestination(icon: Icon(Icons.travel_explore), label: '浏览'),
  NavigationDestination(icon: Icon(Icons.download), label: '任务'),
  NavigationDestination(icon: Icon(Icons.add_link), label: '添加'),
  NavigationDestination(icon: Icon(Icons.settings), label: '设置'),
];

List<NavigationDestination> visibleShellDestinations({
  required bool isTelevision,
}) {
  if (!isTelevision) {
    return kShellDestinations;
  }
  return [
    kShellDestinations[0],
    ...kShellDestinations.skip(kBrowseShellBranchIndex + 1),
  ];
}

int shellBranchIndexForDestination(
  int destinationIndex, {
  required bool isTelevision,
}) {
  if (!isTelevision || destinationIndex == 0) {
    return destinationIndex;
  }
  return destinationIndex + 1;
}

int shellDestinationIndexForBranch(
  int branchIndex, {
  required bool isTelevision,
}) {
  if (!isTelevision) {
    return branchIndex;
  }
  if (branchIndex <= kBrowseShellBranchIndex) {
    return 0;
  }
  return branchIndex - 1;
}
