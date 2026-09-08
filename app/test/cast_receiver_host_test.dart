import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:go_router/go_router.dart';
import 'package:video_sniffing/cast_receiver/cast_providers.dart';
import 'package:video_sniffing/cast_receiver/cast_receiver_host.dart';
import 'package:video_sniffing/engine/models/cast_types.dart';
import 'package:video_sniffing/providers/engine_host_provider.dart';

import 'fakes/fake_engine_repository.dart';

CastPlayRequest _request(String sessionId, {String episodeTitle = 'ep'}) =>
    CastPlayRequest(
      sessionId: sessionId,
      senderDeviceId: 'sender-1',
      senderName: 'Phone',
      streamUrl: 'http://127.0.0.1:9/stream',
      metadata: CastMetadata(
        title: 'series',
        episodeIndex: 1,
        episodeTitle: episodeTitle,
        positionMs: 0,
        mime: 'video/mp4',
      ),
    );

void main() {
  testWidgets('W12 second IncomingPlay replaces activeCastRequest', (tester) async {
    final container = ProviderContainer(
      overrides: [
        engineRepositoryProvider.overrideWithValue(FakeEngineRepository()),
      ],
    );
    final router = GoRouter(
      routes: [
        GoRoute(
          path: '/',
          builder: (_, _) => CastReceiverHost(
            child: const Scaffold(body: Text('home')),
          ),
        ),
        GoRoute(
          path: '/play/cast',
          builder: (_, state) => Text(
            'cast:${state.uri.queryParameters['session_id']}',
          ),
        ),
      ],
    );

    await tester.pumpWidget(
      UncontrolledProviderScope(
        container: container,
        child: MaterialApp.router(routerConfig: router),
      ),
    );

    container.read(activeCastRequestProvider.notifier).state =
        _request('session-a');

    applyIncomingCastPlay(
      container: container,
      router: router,
      request: _request('session-b'),
    );
    await tester.pumpAndSettle();

    expect(
      container.read(activeCastRequestProvider)?.sessionId,
      'session-b',
    );
    expect(find.text('cast:session-b'), findsOneWidget);
    expect(find.text('cast:session-a'), findsNothing);
  });
}
