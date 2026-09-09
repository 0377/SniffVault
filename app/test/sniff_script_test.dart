import 'package:flutter_test/flutter_test.dart';
import 'package:video_sniffing/features/browse/sniff_script.dart';

void main() {
  test('sniff script hooks fetch and XMLHttpRequest via SniffChannel', () {
    expect(sniffScript, contains('fetch'));
    expect(sniffScript, contains('XMLHttpRequest'));
    expect(sniffScript, contains(sniffChannelName));
  });

  test('sniff script hooks HTMLMediaElement src and Request.url', () {
    expect(sniffScript, contains('HTMLMediaElement'));
    expect(sniffScript, contains('.url'));
  });

  test('sniff channel name matches JavaScript SniffChannel', () {
    expect(sniffChannelName, 'SniffChannel');
  });

  test('W12 sniff script scans AJAX response bodies with pageOrigin', () {
    expect(sniffScript, contains('responseText'));
    expect(sniffScript, contains('clone'));
    expect(sniffScript, contains('pageOrigin'));
    expect(sniffScript, contains('scanMediaUrls'));
    expect(sniffScript, contains('524288'));
  });

  test('W12b scanMediaUrls resolves relative path in JSON fixture', () {
    const text = '{"url":"/index.m3u8"}';
    final urls = scanMediaUrls(text, 'https://example.com/player/page');
    expect(urls, ['https://example.com/index.m3u8']);
  });

  test('scanMediaUrls finds absolute URLs and deduplicates', () {
    const text =
        'a=https://cdn.example.com/v.m3u8 b=https://cdn.example.com/v.m3u8';
    final urls = scanMediaUrls(text, 'https://example.com/');
    expect(urls, ['https://cdn.example.com/v.m3u8']);
  });
}
