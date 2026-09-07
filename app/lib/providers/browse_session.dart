import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:video_sniffing/engine/models/download_auth.dart';
import 'package:video_sniffing/engine/models/resolve_types.dart';
import 'package:video_sniffing/engine/models/sniff_types.dart';
import 'package:video_sniffing/features/browse/browse_url.dart';
import 'package:video_sniffing/features/browse/cookie_store.dart';
import 'package:video_sniffing/features/browse/sniff_accumulator.dart';
import 'package:video_sniffing/providers/engine_host_provider.dart';
import 'package:video_sniffing/providers/engine_repository.dart';

final cookieExporterProvider = Provider<CookieExporter>(
  (ref) => const PluginCookieExporter(),
);

final browseCookieStoreProvider = Provider<BrowseCookieStore>(
  (ref) => const PluginCookieExporter(),
);

final browseSessionProvider = ChangeNotifierProvider<BrowseSession>((ref) {
  return BrowseSession(
    repo: ref.watch(engineRepositoryProvider),
    cookies: ref.watch(cookieExporterProvider),
  );
});

class BrowseSession extends ChangeNotifier {
  BrowseSession({required this.repo, required this.cookies});

  static const sniffDebounce = Duration(milliseconds: 300);

  final EngineRepository repo;
  final CookieExporter cookies;
  final SniffAccumulator _accumulator = SniffAccumulator();
  Timer? _sniffDebounce;

  Uri? currentUrl;
  Uri? pendingLoadUrl;
  DownloadAuth? auth;
  ResolveOutcome? outcome;
  List<ResourceCandidate> candidates = const [];

  List<SniffEvent> get sniffEvents => _accumulator.events;

  void applyRouteUrl(String? raw) {
    final parsed = raw == null ? null : parseBrowseUrl(raw);
    if (parsed == null) {
      return;
    }
    pendingLoadUrl = parsed;
    currentUrl = parsed;
    notifyListeners();
  }

  void onTopLevelNavigation(Uri url) {
    _sniffDebounce?.cancel();
    _sniffDebounce = null;
    currentUrl = url;
    _accumulator.onTopLevelNavigation();
    candidates = const [];
    notifyListeners();
  }

  void onHookEvent(SniffEvent event) {
    _accumulator.add(event);
    _sniffDebounce?.cancel();
    _sniffDebounce = Timer(sniffDebounce, _refreshCandidates);
    notifyListeners();
  }

  Future<void> refreshAuth() async {
    final url = currentUrl;
    if (url == null) {
      return;
    }
    final cookieHeader = await cookies.cookieHeaderFor(url);
    auth = DownloadAuth(cookies: cookieHeader, referer: url.toString());
    notifyListeners();
  }

  Future<void> resolveThisPage() async {
    final url = currentUrl;
    if (url == null) {
      return;
    }
    await refreshAuth();
    outcome = await repo.resolveUrl(
      url.toString(),
      opts: ResolveOptions(
        cookies: auth?.cookies,
        referer: auth?.referer,
        pageUrl: auth?.referer,
      ),
    );
    notifyListeners();
  }

  void _refreshCandidates() {
    candidates = repo.sniffUrls(
      _accumulator.events,
      pageUrl: currentUrl?.toString(),
    );
    notifyListeners();
  }

  @override
  void dispose() {
    _sniffDebounce?.cancel();
    super.dispose();
  }
}
