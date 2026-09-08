import 'package:flutter_test/flutter_test.dart';
import 'package:video_sniffing/features/browse/browse_url.dart';

void main() {
  test('W6 javascript rejected', () {
    expect(browseUrlError('javascript:alert(1)'), '非法地址');
    expect(parseBrowseUrl('javascript:alert(1)'), isNull);
  });
  test('http accepted', () {
    expect(browseUrlError('http://127.0.0.1:1/'), isNull);
    expect(parseBrowseUrl('https://example.com/a')?.host, 'example.com');
  });
  test('empty address asks for url', () {
    expect(browseUrlError(''), '请输入地址');
    expect(browseUrlError('   '), '请输入地址');
    expect(parseBrowseUrl(''), isNull);
  });
  test('cookieHeaderFrom joins name=value with semicolon space', () {
    expect(
      cookieHeaderFrom(const [
        CookiePair(name: 'sid', value: 'ok'),
        CookiePair(name: 'a', value: 'b'),
      ]),
      'sid=ok; a=b',
    );
  });
}
