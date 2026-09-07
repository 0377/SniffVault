import 'package:flutter_test/flutter_test.dart';
import 'package:video_sniffing/features/browse/sniff_script.dart';

void main() {
  test('sniff script hooks fetch and XMLHttpRequest via SniffChannel', () {
    expect(sniffScript, contains('fetch'));
    expect(sniffScript, contains('XMLHttpRequest'));
    expect(sniffScript, contains(sniffChannelName));
  });

  test('sniff channel name matches JavaScript SniffChannel', () {
    expect(sniffChannelName, 'SniffChannel');
  });
}
