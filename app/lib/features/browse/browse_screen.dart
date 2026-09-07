import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:video_sniffing/features/browse/browse_unavailable_screen.dart';
import 'package:video_sniffing/providers/device_profile.dart';

class BrowseScreen extends ConsumerWidget {
  const BrowseScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final isTelevision = ref.watch(isTelevisionProvider).value ?? false;
    if (isTelevision) {
      return const BrowseUnavailableScreen();
    }
    return const Scaffold(body: SizedBox.expand());
  }
}
