import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:video_sniffing/features/tv/tv_shell.dart';
import 'package:video_sniffing/providers/device_profile.dart';
import 'package:video_sniffing/shell/app_shell.dart';

class ShellSelector extends ConsumerWidget {
  const ShellSelector({super.key, required this.navigationShell});

  final StatefulNavigationShell navigationShell;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final isTelevision = ref.watch(isTelevisionProvider).value ?? false;
    if (isTelevision) {
      return TvShell(navigationShell: navigationShell);
    }
    return AppShell(navigationShell: navigationShell);
  }
}
