import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'cast_receiver/cast_receiver_host.dart';
import 'deep_link/deep_link_host.dart';
import 'providers/download_coordinator.dart';
import 'providers/engine_host_provider.dart';
import 'router.dart';
import 'ui/app_theme.dart';

class VideoSniffingApp extends ConsumerWidget {
  const VideoSniffingApp({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final hostAsync = ref.watch(engineHostProvider);

    return hostAsync.when(
      loading: () => MaterialApp(
        theme: AppTheme.light,
        darkTheme: AppTheme.dark,
        themeMode: ThemeMode.system,
        home: Scaffold(
          body: SafeArea(
            child: Center(
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
      ),
      error: (error, _) => MaterialApp(
        theme: AppTheme.light,
        darkTheme: AppTheme.dark,
        themeMode: ThemeMode.system,
        home: Scaffold(
          body: SafeArea(
            child: Center(
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
      ),
      data: (_) {
        // Eager：应用生命周期内订阅 taskEvents
        ref.watch(downloadCoordinatorProvider);
        final router = ref.watch(appRouterProvider);
        return MaterialApp.router(
          title: '嗅影库',
          theme: AppTheme.light,
          darkTheme: AppTheme.dark,
          themeMode: ThemeMode.system,
          routerConfig: router,
          builder: (context, child) => DeepLinkHost(
            child: CastReceiverHost(
              child: child ?? const SizedBox.shrink(),
            ),
          ),
        );
      },
    );
  }
}
