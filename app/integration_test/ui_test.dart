import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';

import 'support/app_ui_flow.dart';

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();

  testWidgets('U1-U3 app smoke flow', (tester) async {
    await runAppUiSmokeFlow(tester);
  }, timeout: const Timeout(Duration(minutes: 5)));
}
