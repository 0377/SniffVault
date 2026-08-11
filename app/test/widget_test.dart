import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'package:video_sniffing/app.dart';

void main() {
  testWidgets('shows bootstrap placeholder', (WidgetTester tester) async {
    await tester.pumpWidget(
      const ProviderScope(
        child: VideoSniffingApp(),
      ),
    );

    expect(find.text('bootstrap ok'), findsOneWidget);
  });
}
