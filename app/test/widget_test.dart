import 'package:flutter_test/flutter_test.dart';

import 'package:video_sniffing/main.dart';

void main() {
  testWidgets('shows engine smoke placeholder', (WidgetTester tester) async {
    await tester.pumpWidget(const VideoSniffingApp());

    expect(find.text('Video Sniffing engine smoke OK'), findsOneWidget);
  });
}
