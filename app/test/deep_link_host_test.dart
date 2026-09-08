import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:go_router/go_router.dart';
import 'package:video_sniffing/app.dart';
import 'package:video_sniffing/deep_link/deep_link_providers.dart';
import 'package:video_sniffing/features/add/add_screen.dart';
import 'package:video_sniffing/providers/engine_host_provider.dart';
import 'package:video_sniffing/router.dart';
import 'package:video_sniffing/shell/app_shell.dart';

import 'fakes/fake_engine_repository.dart';
import 'fakes/fake_ready_engine_host.dart';

void main() {
  tearDown(() {
    setBootstrapIngressUri(null);
  });

  testWidgets('W10 navigates to add with prefilled url from bootstrap ingress', (
    tester,
  ) async {
    setBootstrapIngressUri(
      Uri.parse('sniffvault://add?url=https%3A%2F%2Fexample.com'),
    );

    late GoRouter router;
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          engineHostProvider.overrideWith((ref) async {
            final host = FakeReadyEngineHost();
            ref.onDispose(host.dispose);
            return host;
          }),
          engineRepositoryProvider.overrideWithValue(FakeEngineRepository()),
          ingressUriStreamProvider.overrideWithValue(const Stream.empty()),
        ],
        child: Consumer(
          builder: (context, ref, _) {
            router = ref.watch(appRouterProvider);
            return const VideoSniffingApp();
          },
        ),
      ),
    );

    await tester.pump();
    await tester.pumpAndSettle();

    expect(router.state.uri.path, '/add');
    expect(kAddShellBranchIndex, 3);
    expect(find.byType(AddScreen), findsOneWidget);
    expect(
      find.byKey(const Key('add_url_field')),
      findsOneWidget,
    );
    final field = tester.widget<TextField>(find.byKey(const Key('add_url_field')));
    expect(field.controller?.text, 'https://example.com');
  });
}
