import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';

import 'support/deep_link_flow.dart';

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();

  testWidgets('U8 deep link prefills add url field', (tester) async {
    await runDeepLinkPrefillFlow(tester);
  }, timeout: const Timeout(Duration(minutes: 3)));
}
