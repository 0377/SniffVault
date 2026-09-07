import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:video_sniffing/engine/engine_host.dart';
import 'package:video_sniffing/engine/models/resolve_types.dart';
import 'package:video_sniffing/features/add/resolve_wizard.dart';
import 'package:video_sniffing/features/browse/browse_chrome.dart';
import 'package:video_sniffing/features/browse/browse_unavailable_screen.dart';
import 'package:video_sniffing/features/browse/browse_url.dart';
import 'package:video_sniffing/features/browse/sniff_candidate_list.dart';
import 'package:video_sniffing/providers/browse_resolve_provider.dart';
import 'package:video_sniffing/providers/browse_session.dart';
import 'package:video_sniffing/providers/device_profile.dart';
import 'package:video_sniffing/providers/download_coordinator.dart';
import 'package:video_sniffing/providers/engine_host_provider.dart';
import 'package:video_sniffing/providers/settings_provider.dart';
import 'package:video_sniffing/ui/error_presenter.dart';
import 'package:video_sniffing/ui/loading_overlay.dart';

class BrowseScreen extends ConsumerStatefulWidget {
  const BrowseScreen({super.key, this.session});

  final BrowseSession? session;

  @override
  ConsumerState<BrowseScreen> createState() => _BrowseScreenState();
}

class _BrowseScreenState extends ConsumerState<BrowseScreen> {
  String? _appliedRawUrl;
  Uri? _pendingLoadUrl;

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
    widget.session?.applyRouteUrl(raw);
  }

  Future<void> _onResolvePage() async {
    final BrowseSession session =
        widget.session ?? ref.read(browseSessionProvider);
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
        ScaffoldMessenger.of(context).showSnackBar(SnackBar(content: Text(message)));
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    final isTelevision = ref.watch(isTelevisionProvider).value ?? false;
    if (isTelevision) {
      return const BrowseUnavailableScreen();
    }
    final session = widget.session;
    final address = _pendingLoadUrl?.toString() ?? session?.currentUrl?.toString();
    return Scaffold(
      body: Column(
        children: [
          BrowseChrome(
            url: address,
            onSubmit: (uri) {
              setState(() => _pendingLoadUrl = uri);
              session?.applyRouteUrl(uri.toString());
            },
            onResolvePage: _onResolvePage,
          ),
          Expanded(
            child: SniffCandidateList(
              candidates: session?.candidates ?? const [],
              onSelect: (_) {},
            ),
          ),
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
      return Scaffold(
        appBar: AppBar(),
        body: const SizedBox.shrink(),
      );
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
