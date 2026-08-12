import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'providers/download_coordinator.dart';
import 'providers/engine_host_provider.dart';
import 'router.dart';

class VideoSniffingApp extends ConsumerWidget {
  const VideoSniffingApp({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final hostAsync = ref.watch(engineHostProvider);

    return hostAsync.when(
      loading: () => const MaterialApp(
        home: Scaffold(
          body: Center(
            child: Column(
              mainAxisSize: MainAxisSize.min,
              children: [
                CircularProgressIndicator(),
                SizedBox(height: 16),
                Text('正在初始化引擎…'),
              ],
            ),
          ),
        ),
      ),
      error: (error, _) => MaterialApp(
        home: Scaffold(
          body: Center(
            child: Column(
              mainAxisSize: MainAxisSize.min,
              children: [
                Text('引擎初始化失败：$error'),
                const SizedBox(height: 16),
                FilledButton(
                  onPressed: () => ref.invalidate(engineHostProvider),
                  child: const Text('重试'),
                ),
              ],
            ),
          ),
        ),
      ),
      data: (_) {
        // Eager：应用生命周期内订阅 taskEvents
        ref.watch(downloadCoordinatorProvider);
        final router = ref.watch(appRouterProvider);
        return MaterialApp.router(
          title: 'Video Sniffing',
          theme: ThemeData(
            colorScheme: ColorScheme.fromSeed(seedColor: Colors.deepPurple),
            useMaterial3: true,
          ),
          routerConfig: router,
        );
      },
    );
  }
}
