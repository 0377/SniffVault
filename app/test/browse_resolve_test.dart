import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:go_router/go_router.dart';
import 'package:video_sniffing/engine/models/engine_settings.dart';
import 'package:video_sniffing/engine/models/resolve_types.dart';
import 'package:video_sniffing/engine/models/sniff_types.dart';
import 'package:video_sniffing/features/add/resolve_wizard.dart';
import 'package:video_sniffing/features/browse/browse_screen.dart';
import 'package:video_sniffing/features/browse/cookie_store.dart';
import 'package:video_sniffing/providers/browse_resolve_provider.dart';
import 'package:video_sniffing/providers/browse_session.dart';
import 'package:video_sniffing/providers/engine_host_provider.dart';
import 'package:video_sniffing/providers/settings_provider.dart';
import 'package:webview_flutter/webview_flutter.dart';

import 'fakes/fake_engine_repository.dart';

class FakeCookieExporter implements CookieExporter {
  FakeCookieExporter(this.header);

  final String? header;

  @override
  Future<String?> cookieHeaderFor(Uri page) async => header;
}

class _SniffRecordingRepo extends FakeEngineRepository {
  String? lastSniffPageUrl;
  List<SniffEvent>? lastSniffEvents;
  List<ResourceCandidate> sniffResult = const [];

  @override
  List<ResourceCandidate> sniffUrls(
    List<SniffEvent> events, {
    String? pageUrl,
  }) {
    lastSniffEvents = List.of(events);
    lastSniffPageUrl = pageUrl;
    return sniffResult;
  }
}

void main() {
  test('W8 resolveThisPage passes cookies', () async {
    final fake = FakeEngineRepository();
    final session = BrowseSession(
      repo: fake,
      cookies: FakeCookieExporter('sid=ok'),
    );
    session.currentUrl = Uri.parse('http://x/page');
    await session.resolveThisPage();
    expect(fake.lastResolveOpts?.cookies, 'sid=ok');
  });

  test('resolveThisPage with null cookies still calls resolveUrl', () async {
    final fake = FakeEngineRepository();
    final session = BrowseSession(
      repo: fake,
      cookies: FakeCookieExporter(null),
    );
    session.currentUrl = Uri.parse('http://x/page');
    await session.resolveThisPage();
    expect(fake.lastResolveOpts?.cookies, isNull);
    expect(fake.lastResolveOpts?.referer, 'http://x/page');
  });

  test('W9 browse enqueueSingle receives session auth', () async {
    final fake = FakeEngineRepository();
    final session = BrowseSession(
      repo: fake,
      cookies: FakeCookieExporter('sid=ok'),
    );
    session.currentUrl = Uri.parse('http://x/page');
    await session.resolveThisPage();

    fake.enqueueSingle(title: 't', url: 'http://x/v.mp4', auth: session.auth);

    expect(fake.lastEnqueueAuth?.cookies, 'sid=ok');
    expect(fake.lastEnqueueAuth?.referer, 'http://x/page');
  });

  testWidgets('W9 add page enqueueSingle does not pass auth', (tester) async {
    final fake = FakeEngineRepository();
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: ResolveWizard(
            outcome: ResolveOutcomeSingle(
              ResourceCandidate(
                id: '1',
                url: 'https://x/y.mp4',
                kind: MediaKind.mp4,
              ),
            ),
            enqueueSingle: fake.enqueueSingle,
            onEnqueue: (_) async {},
          ),
        ),
      ),
    );

    await tester.tap(find.text('下载'));
    await tester.pump();

    expect(fake.lastEnqueueAuth, isNull);
  });

  testWidgets('W9 browse wizard enqueueSingle passes session auth', (
    tester,
  ) async {
    final fake = FakeEngineRepository();
    final session = BrowseSession(
      repo: fake,
      cookies: FakeCookieExporter('sid=ok'),
    );
    session.currentUrl = Uri.parse('http://x/page');
    await session.resolveThisPage();

    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: ResolveWizard(
            outcome: session.outcome!,
            auth: session.auth,
            enqueueSingle: fake.enqueueSingle,
            onEnqueue: (_) async {},
          ),
        ),
      ),
    );

    await tester.tap(find.text('下载'));
    await tester.pump();

    expect(fake.lastEnqueueAuth?.cookies, 'sid=ok');
    expect(fake.lastEnqueueAuth?.referer, 'http://x/page');
  });

  testWidgets('top-level URL change clears accumulator', (tester) async {
    final session = BrowseSession(
      repo: FakeEngineRepository(),
      cookies: FakeCookieExporter('sid=ok'),
    );
    session.currentUrl = Uri.parse('http://x/page');
    session.onHookEvent(
      const SniffEvent(url: 'http://x/a.m3u8', initiator: SniffInitiator.media),
    );

    session.onTopLevelNavigation(Uri.parse('http://x/other'));

    expect(session.currentUrl, Uri.parse('http://x/other'));
    expect(session.sniffEvents, isEmpty);
    expect(session.candidates, isEmpty);
  });

  testWidgets('hook events debounce 300ms then sniffUrls', (tester) async {
    final fake = _SniffRecordingRepo();
    fake.sniffResult = const [
      ResourceCandidate(id: '1', url: 'http://x/a.m3u8', kind: MediaKind.hls),
    ];
    final session = BrowseSession(
      repo: fake,
      cookies: FakeCookieExporter('sid=ok'),
    );
    session.currentUrl = Uri.parse('http://x/page');
    session.onHookEvent(
      const SniffEvent(url: 'http://x/a.m3u8', initiator: SniffInitiator.media),
    );

    await tester.pump(const Duration(milliseconds: 299));
    expect(fake.lastSniffPageUrl, isNull);
    expect(session.candidates, isEmpty);

    await tester.pump(const Duration(milliseconds: 1));
    expect(fake.lastSniffPageUrl, 'http://x/page');
    expect(fake.lastSniffEvents, isNotEmpty);
    expect(session.candidates, fake.sniffResult);
  });

  testWidgets('browse wizard NeedsBrowser does not pass onOpenBrowser', (
    tester,
  ) async {
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          engineRepositoryProvider.overrideWithValue(FakeEngineRepository()),
          settingsProvider.overrideWith((ref) => EngineSettings.defaults),
          browseResolveProvider.overrideWith(
            (ref) => const BrowseResolveArgs(
              outcome: ResolveOutcomeNeedsBrowser(reason: 'auth_required'),
            ),
          ),
        ],
        child: const MaterialApp(home: BrowseWizardPage()),
      ),
    );

    expect(find.textContaining('需要在内置浏览中打开'), findsOneWidget);
    expect(find.text('打开内置浏览'), findsNothing);
    expect(find.text('返回'), findsOneWidget);
  });

  testWidgets('/browse?url= parses for load without WebView', (tester) async {
    final session = BrowseSession(
      repo: FakeEngineRepository(),
      cookies: FakeCookieExporter('sid=ok'),
    );
    final encoded = Uri.encodeQueryComponent('https://example.com/watch');
    await tester.pumpWidget(
      ProviderScope(
        child: MaterialApp.router(
          routerConfig: GoRouter(
            initialLocation: '/browse?url=$encoded',
            routes: [
              GoRoute(
                path: '/browse',
                builder: (_, _) => BrowseScreen(session: session),
              ),
            ],
          ),
        ),
      ),
    );
    await tester.pump();

    expect(session.pendingLoadUrl, Uri.parse('https://example.com/watch'));
    expect(session.currentUrl, Uri.parse('https://example.com/watch'));
  });

  testWidgets('invalid /browse?url= is not loaded', (tester) async {
    final session = BrowseSession(
      repo: FakeEngineRepository(),
      cookies: FakeCookieExporter('sid=ok'),
    );
    await tester.pumpWidget(
      ProviderScope(
        child: MaterialApp.router(
          routerConfig: GoRouter(
            initialLocation:
                '/browse?url=${Uri.encodeQueryComponent('javascript:alert(1)')}',
            routes: [
              GoRoute(
                path: '/browse',
                builder: (_, _) => BrowseScreen(session: session),
              ),
            ],
          ),
        ),
      ),
    );
    await tester.pump();

    expect(session.pendingLoadUrl, isNull);
    expect(session.currentUrl, isNull);
  });

  testWidgets('BrowseScreen watches browseSessionProvider candidates', (
    tester,
  ) async {
    final fake = _SniffRecordingRepo();
    fake.sniffResult = const [
      ResourceCandidate(id: '1', url: 'http://x/a.m3u8', kind: MediaKind.hls),
    ];
    final session = BrowseSession(
      repo: fake,
      cookies: FakeCookieExporter('sid=ok'),
    );
    session.currentUrl = Uri.parse('http://x/page');

    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          browseSessionProvider.overrideWith((ref) => session),
          settingsProvider.overrideWith((ref) => EngineSettings.defaults),
        ],
        child: MaterialApp.router(
          routerConfig: GoRouter(
            initialLocation: '/browse',
            routes: [
              GoRoute(path: '/browse', builder: (_, _) => const BrowseScreen()),
            ],
          ),
        ),
      ),
    );
    await tester.pump();

    expect(find.byType(WebViewWidget), findsOneWidget);
    expect(find.text('嗅探候选 1'), findsNothing);

    session.onHookEvent(
      const SniffEvent(url: 'http://x/a.m3u8', initiator: SniffInitiator.media),
    );
    await tester.pump(const Duration(milliseconds: 300));

    expect(find.text('嗅探候选 1'), findsOneWidget);
    expect(find.text('http://x/a.m3u8'), findsOneWidget);
  });

  testWidgets('tap sniff candidate opens wizard and enqueue keeps cookies', (
    tester,
  ) async {
    final fake = FakeEngineRepository();
    fake.sniffResults = const [
      ResourceCandidate(id: '1', url: 'http://x/a.m3u8', kind: MediaKind.hls),
    ];
    final session = BrowseSession(
      repo: fake,
      cookies: FakeCookieExporter('sid=ok'),
    );
    session.currentUrl = Uri.parse('http://x/page');
    session.candidates = fake.sniffResults;

    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          engineRepositoryProvider.overrideWithValue(fake),
          settingsProvider.overrideWith((ref) => EngineSettings.defaults),
          browseSessionProvider.overrideWith((ref) => session),
        ],
        child: MaterialApp.router(
          routerConfig: GoRouter(
            initialLocation: '/browse',
            routes: [
              GoRoute(
                path: '/browse',
                builder: (_, _) => BrowseScreen(session: session),
                routes: [
                  GoRoute(
                    path: 'wizard',
                    builder: (_, _) => const BrowseWizardPage(),
                  ),
                ],
              ),
              GoRoute(path: '/tasks', builder: (_, _) => const Text('tasks')),
            ],
          ),
        ),
      ),
    );
    await tester.pump();

    await tester.tap(find.text('http://x/a.m3u8'));
    await tester.pumpAndSettle();

    expect(find.text('下载'), findsOneWidget);
    expect(find.text('确认下载'), findsOneWidget);
    expect(fake.lastResolveQualitiesOpts?.cookies, 'sid=ok');
    expect(fake.lastResolveQualitiesOpts?.referer, 'http://x/page');
    expect(fake.lastResolveQualitiesOpts?.pageUrl, 'http://x/page');

    await tester.tap(find.text('下载'));
    await tester.pumpAndSettle();

    expect(fake.lastEnqueueAuth?.cookies, 'sid=ok');
    expect(fake.lastEnqueueAuth?.referer, 'http://x/page');
  });
}
