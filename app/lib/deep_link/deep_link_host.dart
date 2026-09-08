import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:video_sniffing/deep_link/deep_link_providers.dart';
import 'package:video_sniffing/deep_link/ingress_uri.dart';
import 'package:video_sniffing/router.dart';

class IngressUriListener extends ConsumerStatefulWidget {
  const IngressUriListener({super.key, required this.child});

  final Widget child;

  @override
  ConsumerState<IngressUriListener> createState() => _IngressUriListenerState();
}

class _IngressUriListenerState extends ConsumerState<IngressUriListener> {
  StreamSubscription<Uri>? _subscription;

  @override
  void initState() {
    super.initState();
    final stream = ref.read(ingressUriStreamProvider);
    _subscription = stream.listen((uri) {
      ref.read(pendingIngressUriProvider.notifier).state = uri;
    });
  }

  @override
  void dispose() {
    _subscription?.cancel();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => widget.child;
}

class DeepLinkHost extends ConsumerStatefulWidget {
  const DeepLinkHost({super.key, required this.child});

  final Widget child;

  @override
  ConsumerState<DeepLinkHost> createState() => _DeepLinkHostState();
}

class _DeepLinkHostState extends ConsumerState<DeepLinkHost> {
  String? _lastHandledUri;
  StreamSubscription<Uri>? _subscription;

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addPostFrameCallback((_) {
      _flushPending();
      _subscribeStream();
    });
  }

  @override
  void dispose() {
    _subscription?.cancel();
    super.dispose();
  }

  void _flushPending() {
    final pending = ref.read(pendingIngressUriProvider);
    if (pending != null) {
      ref.read(pendingIngressUriProvider.notifier).state = null;
      _handleUri(pending);
    }
  }

  void _subscribeStream() {
    final stream = ref.read(ingressUriStreamProvider);
    _subscription = stream.listen(_handleUri);
  }

  void _handleUri(Uri uri) {
    final key = uri.toString();
    if (_lastHandledUri == key) {
      return;
    }
    final parsed = parseSniffVaultIngress(uri);
    if (parsed == null) {
      return;
    }
    _lastHandledUri = key;
    final router = ref.read(appRouterProvider);
    final messenger = ScaffoldMessenger.maybeOf(context);

    if (parsed is IngressNavigateSuccess) {
      final encoded = Uri.encodeQueryComponent(parsed.url);
      router.go('/add?url=$encoded');
      return;
    }

    final failure = (parsed as IngressNavigateFailure).reason;
    router.go('/add');
    messenger?.showSnackBar(
      SnackBar(content: Text(snackBarMessageFor(failure))),
    );
  }

  @override
  Widget build(BuildContext context) => widget.child;
}
