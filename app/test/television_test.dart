import 'package:flutter_test/flutter_test.dart';
import 'package:video_sniffing/platform/television.dart';

void main() {
  test('detectIsTelevision returns false off Android', () async {
    expect(await detectIsTelevision(), isFalse);
  });
}
