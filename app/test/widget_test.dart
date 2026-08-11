import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'package:video_sniffing/app.dart';
import 'package:video_sniffing/providers/engine_host_provider.dart';

import 'fakes/fake_engine_repository.dart';
import 'fakes/fake_ready_engine_host.dart';

void main() {
  testWidgets('shows library shell after engine is ready', (WidgetTester tester) async {
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          engineHostProvider.overrideWith((ref) async {
            final host = FakeReadyEngineHost();
            ref.onDispose(host.dispose);
            return host;
          }),
          engineRepositoryProvider.overrideWithValue(FakeEngineRepository()),
        ],
        child: const VideoSniffingApp(),
      ),
    );

    await tester.pump();
    await tester.pumpAndSettle();
    expect(find.text('片库'), findsWidgets);
  });
}
