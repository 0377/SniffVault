import 'package:flutter_test/flutter_test.dart';
import 'package:video_sniffing/deep_link/share_url_extractor.dart';

void main() {
  test('W12 extracts first http url from surrounding text', () {
    expect(
      extractHttpUrl('看这里 https://a.com/x 和 https://b.com'),
      'https://a.com/x',
    );
  });

  test('accepts whole string when already valid url', () {
    expect(
      extractHttpUrl('https://example.com/watch?v=1'),
      'https://example.com/watch?v=1',
    );
  });

  test('rejects javascript scheme', () {
    expect(extractHttpUrl('javascript:alert(1)'), isNull);
  });

  test('rejects empty and plain text without url', () {
    expect(extractHttpUrl(''), isNull);
    expect(extractHttpUrl('只是文字'), isNull);
  });

  test('isIngressPayloadTooLong at boundary', () {
    expect(isIngressPayloadTooLong('x' * 8192), isFalse);
    expect(isIngressPayloadTooLong('x' * 8193), isTrue);
  });
}
