import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';

import 'support/cast_flow.dart';

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();

  testWidgets('U10 pair cast metadata has no source_url', (tester) async {
    await runPairCastMetadataFlow(tester);
  }, timeout: const Timeout(Duration(minutes: 3)));
}
