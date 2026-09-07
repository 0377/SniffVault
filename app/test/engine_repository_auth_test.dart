import 'package:flutter_test/flutter_test.dart';
import 'package:video_sniffing/engine/models/download_auth.dart';

import 'fakes/fake_engine_repository.dart';

void main() {
  test('Fake enqueueSingle stores DownloadAuth', () {
    final fake = FakeEngineRepository();
    fake.enqueueSingle(
      title: 't',
      url: 'https://x/v.mp4',
      auth: const DownloadAuth(cookies: 'sid=ok', referer: 'https://x/page'),
    );
    expect(fake.lastEnqueueAuth?.cookies, 'sid=ok');
  });
}
