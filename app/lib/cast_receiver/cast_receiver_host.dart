import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:video_sniffing/cast_receiver/cast_providers.dart';
import 'package:video_sniffing/engine/models/cast_types.dart';
import 'package:video_sniffing/providers/engine_host_provider.dart';
import 'package:video_sniffing/router.dart';

class CastReceiverHost extends ConsumerStatefulWidget {
  const CastReceiverHost({super.key, required this.child});

  final Widget child;

  @override
  ConsumerState<CastReceiverHost> createState() => _CastReceiverHostState();
}

class _CastReceiverHostState extends ConsumerState<CastReceiverHost> {
  StreamSubscription<CastEvent>? _subscription;

  @override
  void initState() {
    super.initState();
    final repo = ref.read(engineRepositoryProvider);
    _subscription = repo.castEvents.listen(_onCastEvent);
  }

  void _onCastEvent(CastEvent event) {
    final router = ref.read(appRouterProvider);
    switch (event) {
      case CastEventIncomingPlay(:final request):
        ref.read(activeCastRequestProvider.notifier).state = request;
        final sessionId = Uri.encodeComponent(request.sessionId);
        router.push('/play/cast?session_id=$sessionId');
      case CastEventSessionEnded(:final sessionId):
        final active = ref.read(activeCastRequestProvider);
        if (active?.sessionId == sessionId) {
          ref.read(activeCastRequestProvider.notifier).state = null;
          final path = router.state.uri.path;
          if (path == '/play/cast') {
            router.pop();
          }
        }
      case CastEventError():
        break;
    }
  }

  @override
  void dispose() {
    _subscription?.cancel();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => widget.child;
}
