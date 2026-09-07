import 'package:flutter_test/flutter_test.dart';
import 'package:video_sniffing/features/browse/hook_to_sniff.dart';

void main() {
  test('JS channel message fills page_url from Dart current main-frame URL', () {
    const payload =
        '{"url":"https://cdn/a.m3u8","mime":"application/vnd.apple.mpegurl","is_main_frame":false}';

    final request = hookRequestFromJsMessage(
      payload,
      pageUrl: 'https://site/watch',
    )!;

    expect(request.url, 'https://cdn/a.m3u8');
    expect(request.mime, 'application/vnd.apple.mpegurl');
    expect(request.isMainFrame, isFalse);
    expect(request.pageUrl, 'https://site/watch');
  });

  test('empty mime from JS is treated as null', () {
    const payload =
        '{"url":"https://cdn/app.js","mime":"","is_main_frame":false}';

    final request = hookRequestFromJsMessage(
      payload,
      pageUrl: 'https://site/watch',
    )!;

    expect(request.mime, isNull);
    expect(request.isMainFrame, isFalse);
  });

  test('malformed JS message is ignored', () {
    expect(
      hookRequestFromJsMessage('not-json', pageUrl: 'https://site/watch'),
      isNull,
    );
    expect(
      hookRequestFromJsMessage('{"mime":""}', pageUrl: 'https://x'),
      isNull,
    );
  });
}
