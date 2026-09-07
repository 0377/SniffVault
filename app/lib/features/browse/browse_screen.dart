import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:video_sniffing/engine/engine_host.dart';
import 'package:video_sniffing/engine/models/resolve_types.dart';
import 'package:video_sniffing/features/add/resolve_wizard.dart';
import 'package:video_sniffing/features/browse/browse_chrome.dart';
import 'package:video_sniffing/features/browse/browse_load_error.dart';
import 'package:video_sniffing/features/browse/browse_unavailable_screen.dart';
import 'package:video_sniffing/features/browse/browse_url.dart';
import 'package:video_sniffing/features/browse/hook_to_sniff.dart';
import 'package:video_sniffing/features/browse/sniff_candidate_list.dart';
import 'package:video_sniffing/features/browse/sniff_script.dart';
import 'package:video_sniffing/providers/browse_resolve_provider.dart';
import 'package:video_sniffing/providers/browse_session.dart';
import 'package:video_sniffing/providers/device_profile.dart';
import 'package:video_sniffing/providers/download_coordinator.dart';
import 'package:video_sniffing/providers/engine_host_provider.dart';
import 'package:video_sniffing/providers/settings_provider.dart';
import 'package:video_sniffing/ui/error_presenter.dart';
import 'package:video_sniffing/ui/loading_overlay.dart';
import 'package:webview_flutter/webview_flutter.dart';

class BrowseScreen extends ConsumerStatefulWidget {
  const BrowseScreen({super.key, this.session});

  final BrowseSession? session;

  @override
  ConsumerState<BrowseScreen> createState() => _BrowseScreenState();
}

class _BrowseScreenState extends ConsumerState<BrowseScreen> {
  String? _appliedRawUrl;
  Uri? _pendingLoadUrl;
  WebViewController? _controller;
  String? _loadError;
  String? _lastMainFrameUrl;
  bool _canGoBack = false;
  bool _canGoForward = false;

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    final raw = GoRouterState.of(context).uri.queryParameters['url'];
    if (raw == _appliedRawUrl) {
      return;
    }
    _appliedRawUrl = raw;
    final parsed = raw == null ? null : parseBrowseUrl(raw);
    if (parsed != null) {
      _pendingLoadUrl = parsed;
    }
    final injected = widget.session;
    if (injected != null) {
      injected.applyRouteUrl(raw);
    } else {
      ref.read(browseSessionProvider).applyRouteUrl(raw);
    }
  }

  BrowseSession _resolvedSession({required bool watch}) {
    final injected = widget.session;
    if (injected != null) {
      return injected;
    }
    return watch
        ? ref.watch(browseSessionProvider)
        : ref.read(browseSessionProvider);
  }

  void _ensureController(BrowseSession session, String? userAgent) {
    if (_controller != null) {
      if (userAgent != null && userAgent.isNotEmpty) {
        _controller!.setUserAgent(userAgent);
      }
      return;
    }
    final controller = WebViewController()
      ..setJavaScriptMode(JavaScriptMode.unrestricted)
      ..addJavaScriptChannel(
        sniffChannelName,
        onMessageReceived: (message) => _onJsMessage(session, message.message),
      )
      ..setNavigationDelegate(
        NavigationDelegate(
          onNavigationRequest: (request) {
            if (request.isMainFrame) {
              _onMainFrameNavigation(session, request.url);
            }
            return NavigationDecision.navigate;
          },
          onPageStarted: (url) {
            _onMainFrameNavigation(session, url);
            if (mounted) {
              setState(() => _loadError = null);
            }
          },
          onPageFinished: (url) async {
            try {
              await _controller?.runJavaScript(sniffScript);
            } catch (_) {}
            await _refreshHistoryButtons();
          },
          onWebResourceError: (error) {
            if (error.isForMainFrame == false) {
              return;
            }
            if (mounted) {
              setState(() => _loadError = error.description);
            }
          },
        ),
      );
    if (userAgent != null && userAgent.isNotEmpty) {
      controller.setUserAgent(userAgent);
    }
    _controller = controller;
    final pending =
        _pendingLoadUrl ?? session.pendingLoadUrl ?? session.currentUrl;
    if (pending != null) {
      controller.loadRequest(pending);
    }
  }

  void _onJsMessage(BrowseSession session, String message) {
    final request = hookRequestFromJsMessage(
      message,
      pageUrl: session.currentUrl?.toString(),
    );
    if (request == null) {
      return;
    }
    session.onHookEvent(hookToSniffEvent(request));
  }

  void _onMainFrameNavigation(BrowseSession session, String url) {
    if (url.isEmpty || url.toLowerCase() == 'about:blank') {
      return;
    }
    if (url == _lastMainFrameUrl) {
      return;
    }
    _lastMainFrameUrl = url;
    final uri = Uri.tryParse(url);
    if (uri == null) {
      return;
    }
    session.onTopLevelNavigation(uri);
    session.onHookEvent(
      hookToSniffEvent(HookRequest(url: url, pageUrl: url, isMainFrame: true)),
    );
    if (mounted) {
      setState(() => _pendingLoadUrl = uri);
    }
  }

  Future<void> _loadUrl(Uri uri) async {
    setState(() {
      _pendingLoadUrl = uri;
      _loadError = null;
    });
    try {
      await _controller?.loadRequest(uri);
    } catch (error) {
      if (mounted) {
        setState(() => _loadError = error.toString());
      }
    }
  }

  Future<void> _retryLoad() async {
    setState(() => _loadError = null);
    final controller = _controller;
    if (controller == null) {
      return;
    }
    final uri = _pendingLoadUrl ?? _resolvedSession(watch: false).currentUrl;
    try {
      if (uri != null) {
        await controller.loadRequest(uri);
      } else {
        await controller.reload();
      }
    } catch (error) {
      if (mounted) {
        setState(() => _loadError = error.toString());
      }
    }
  }

  Future<void> _refreshHistoryButtons() async {
    final controller = _controller;
    if (controller == null) {
      return;
    }
    final back = await controller.canGoBack();
    final forward = await controller.canGoForward();
    if (!mounted) {
      return;
    }
    if (back != _canGoBack || forward != _canGoForward) {
      setState(() {
        _canGoBack = back;
        _canGoForward = forward;
      });
    }
  }

  Future<void> _onResolvePage() async {
    final session = _resolvedSession(watch: false);
    session.currentUrl ??= _pendingLoadUrl;
    if (session.currentUrl == null) {
      return;
    }
    try {
      await LoadingOverlay.run(context, session.resolveThisPage);
      if (!mounted) {
        return;
      }
      final outcome = session.outcome;
      if (outcome == null) {
        return;
      }
      ref.read(browseResolveProvider.notifier).state = BrowseResolveArgs(
        outcome: outcome,
        auth: session.auth,
      );
      context.push('/browse/wizard');
    } on EngineException catch (e) {
      final message = presentEngineError(e);
      if (message != null && mounted) {
        ScaffoldMessenger.of(
          context,
        ).showSnackBar(SnackBar(content: Text(message)));
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    final tvAsync = ref.watch(isTelevisionProvider);
    if (!tvAsync.hasValue) {
      return const Scaffold(body: SizedBox.shrink());
    }
    if (tvAsync.requireValue) {
      return const BrowseUnavailableScreen();
    }
    final session = _resolvedSession(watch: true);
    String? userAgent;
    if (widget.session == null) {
      final ua = ref.watch(settingsProvider).userAgent;
      if (ua != null && ua.isNotEmpty) {
        userAgent = ua;
      }
    }
    _ensureController(session, userAgent);
    final address =
        _pendingLoadUrl?.toString() ?? session.currentUrl?.toString();
    final controller = _controller;
    return Scaffold(
      body: Column(
        children: [
          BrowseChrome(
            url: address,
            canGoBack: _canGoBack,
            canGoForward: _canGoForward,
            onSubmit: (uri) {
              session.applyRouteUrl(uri.toString());
              _loadUrl(uri);
            },
            onBack: () => _controller?.goBack(),
            onForward: () => _controller?.goForward(),
            onReload: _retryLoad,
            onResolvePage: _onResolvePage,
          ),
          Expanded(
            child: _loadError != null
                ? BrowseLoadError(message: _loadError!, onRetry: _retryLoad)
                : controller == null
                ? const SizedBox.shrink()
                : WebViewWidget(controller: controller),
          ),
          SniffCandidateList(candidates: session.candidates, onSelect: (_) {}),
        ],
      ),
    );
  }
}

class BrowseWizardPage extends ConsumerWidget {
  const BrowseWizardPage({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final args = ref.watch(browseResolveProvider);
    if (args == null) {
      return Scaffold(appBar: AppBar(), body: const SizedBox.shrink());
    }
    final repo = ref.watch(engineRepositoryProvider);
    final settings = ref.watch(settingsProvider);
    final opts = ResolveOptions(
      cookies: args.auth?.cookies,
      referer: args.auth?.referer,
      pageUrl: args.auth?.referer,
    );
    return Scaffold(
      appBar: AppBar(title: const Text('确认下载')),
      body: ResolveWizard(
        outcome: args.outcome,
        auth: args.auth,
        defaultQualityLabel: settings.defaultQualityLabel,
        resolveQualities: (url) => repo.resolveQualities(url, opts: opts),
        enqueueSingle: repo.enqueueSingle,
        enqueueEpisodes: repo.enqueueEpisodes,
        onEnqueue: (_) async {
          ref.read(downloadCoordinatorProvider).ensureDownloads();
          if (!context.mounted) {
            return;
          }
          context.go('/tasks');
        },
      ),
    );
  }
}
