import 'dart:async';
import 'dart:convert';
import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';
import 'package:video_sniffing/engine/engine_host.dart';

import 'support/cast_flow.dart';
import 'support/playable_mp4.dart';

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();

  testWidgets('U11c engine poster register refresh delete', (tester) async {
    final dataDir = Directory.systemTemp.createTempSync('u11c');
    final host = await EngineHost.open(dataDir.path);
    final fixture = await _PosterFixture.start();
    try {
      // register + poster_url → listLibrary 含 poster_path
      final title = 'u11c-poster-${DateTime.now().millisecondsSinceEpoch}';
      final taskId = host.enqueueSingle(
        title: title,
        url: fixture.mp4Url,
        posterUrl: fixture.posterImageUrl,
      );
      expect(taskId, isNotEmpty);
      await waitForTaskCompleted(tester, host, taskId: taskId);

      final item = host.listLibrary().firstWhere((i) => i.title == title);
      expect(item.posterPath, isNotNull);
      expect(item.posterPath!, contains('.posters'));
      expect(File(item.posterPath!).existsSync(), isTrue);
      final posterBeforeRefresh = item.posterPath!;

      // refreshLibraryPoster 更新封面（经 og:image 页面）
      final refreshed = host.refreshLibraryPoster(
        item.id,
        pageUrl: fixture.pageUrl,
      );
      expect(refreshed.posterPath, isNotNull);
      expect(refreshed.posterPath!, contains('.posters'));
      expect(File(refreshed.posterPath!).existsSync(), isTrue);

      // delete 清 poster 文件
      host.removeLibraryItem(item.id, deleteFiles: true);
      expect(File(posterBeforeRefresh).existsSync(), isFalse);
      if (refreshed.posterPath != posterBeforeRefresh) {
        expect(File(refreshed.posterPath!).existsSync(), isFalse);
      }
      expect(host.listLibrary(), isEmpty);
    } finally {
      await fixture.stop();
      host.dispose();
    }
  }, timeout: const Timeout(Duration(minutes: 3)));
}

class _PosterFixture {
  _PosterFixture(this._server, this.mp4Url, this.posterImageUrl, this.pageUrl);

  final HttpServer _server;
  final String mp4Url;
  final String posterImageUrl;
  final String pageUrl;

  static Future<_PosterFixture> start() async {
    final mp4Bytes = playableMp4Bytes();
    final posterBytes = _samplePosterBytes();
    final server = await HttpServer.bind(InternetAddress.loopbackIPv4, 0);
    final base = 'http://127.0.0.1:${server.port}';
    final posterImageUrl = '$base/sample.jpg';
    final pageUrl = '$base/page.html';

    server.listen((request) async {
      switch (request.uri.path) {
        case '/clip.mp4':
          request.response.headers.contentType = ContentType('video', 'mp4');
          request.response.contentLength = mp4Bytes.length;
          request.response.statusCode = HttpStatus.ok;
          request.response.add(mp4Bytes);
          await request.response.close();
        case '/sample.jpg':
          request.response.headers.contentType = ContentType('image', 'jpeg');
          request.response.contentLength = posterBytes.length;
          request.response.statusCode = HttpStatus.ok;
          request.response.add(posterBytes);
          await request.response.close();
        case '/page.html':
          request.response.headers.contentType = ContentType('text', 'html');
          final html =
              '<!DOCTYPE html><html><head>'
              '<meta property="og:image" content="$posterImageUrl">'
              '</head><body></body></html>';
          final body = utf8.encode(html);
          request.response.contentLength = body.length;
          request.response.statusCode = HttpStatus.ok;
          request.response.add(body);
          await request.response.close();
        default:
          request.response.statusCode = HttpStatus.notFound;
          await request.response.close();
      }
    });

    return _PosterFixture(
      server,
      '$base/clip.mp4',
      posterImageUrl,
      pageUrl,
    );
  }

  Future<void> stop() async {
    await _server.close(force: true);
  }
}

const _sampleJpegBase64 =
    '/9j/4AAQSkZJRgABAQAAAQABAAD/2wBDAAgGBgcGBQgHBwcJCQgKDBQNDAsLDBkSEw8UHRofHh0aHBwgJC4nICIsIxwcKDcpLDAxNDQ0Hyc5PTgyPC4zNDL/wAALCAABAAEBAREA/8QAHwAAAQUBAQEBAQEAAAAAAAAAAAECAwQFBgcICQoL/8QAtRAAAgEDAwIEAwUFBAQAAAF9AQIDAAQRBRIhMUEGE1FhByJxFDKBkaEII0KxwRVS0fAkM2JyggkKFhcYGRolJicoKSo0NTY3ODk6Q0RFRkdISUpTVFVWV1hZWmNkZWZnaGlqc3R1dnd4eXqDhIWGh4iJipKTlJWWl5iZmqKjpKWmp6ipqrKztLW2t7i5usLDxMXGx8jJytLT1NXW19jZ2uHi4+Tl5ufo6erx8vP09fb3+Pn6/9oACAEBAAA/ADf/2Q==';

List<int> _samplePosterBytes() {
  final candidates = [
    '../engine/tests/fixtures/posters/sample.jpg',
    '../../engine/tests/fixtures/posters/sample.jpg',
    '../../../engine/tests/fixtures/posters/sample.jpg',
  ];
  for (final path in candidates) {
    final file = File(path);
    if (file.existsSync()) {
      return file.readAsBytesSync();
    }
  }
  return base64Decode(_sampleJpegBase64);
}
