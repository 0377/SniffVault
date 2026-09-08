import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';

import 'support/cast_flow.dart';

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();

  testWidgets('U10 pair cast metadata has no source_url', (tester) async {
    await runPairCastMetadataFlow(tester);
  }, timeout: const Timeout(Duration(minutes: 3)));

  testWidgets('U10b second cast replaces first session on receiver', (
    tester,
  ) async {
    await runDoubleCastReplacementFlow(tester);
  }, timeout: const Timeout(Duration(minutes: 4)));
}
