import 'package:flutter_test/flutter_test.dart';
import 'package:video_sniffing/engine/models/sniff_types.dart';
import 'package:video_sniffing/features/browse/hook_to_sniff.dart';
import 'package:video_sniffing/features/browse/sniff_accumulator.dart';

void main() {
  group('hookToSniffEvent', () {
    test('maps main frame to navigation', () {
      final event = hookToSniffEvent(
        const HookRequest(
          url: 'https://site/page',
          pageUrl: 'https://site/page',
          isMainFrame: true,
          mime: 'text/html',
        ),
      );

      expect(event.initiator, SniffInitiator.navigation);
      expect(event.url, 'https://site/page');
      expect(event.pageUrl, 'https://site/page');
    });

    test('maps video mime or media path to media', () {
      expect(
        hookToSniffEvent(
          const HookRequest(
            url: 'https://cdn/seg.ts',
            pageUrl: 'https://site/page',
            isMainFrame: false,
            mime: 'video/mp4',
          ),
        ).initiator,
        SniffInitiator.media,
      );
      expect(
        hookToSniffEvent(
          const HookRequest(
            url: 'https://cdn/a.m3u8?token=1',
            isMainFrame: false,
          ),
        ).initiator,
        SniffInitiator.media,
      );
      expect(
        hookToSniffEvent(
          const HookRequest(
            url: 'https://cdn/a.mp4?token=1',
            isMainFrame: false,
            mime: 'application/octet-stream',
          ),
        ).initiator,
        SniffInitiator.media,
      );
    });

    test('maps other non-main-frame to sub_resource', () {
      final event = hookToSniffEvent(
        const HookRequest(
          url: 'https://cdn/app.js',
          pageUrl: 'https://site/page',
          isMainFrame: false,
          mime: 'application/javascript',
        ),
      );

      expect(event.initiator, SniffInitiator.subResource);
      expect(event.url, 'https://cdn/app.js');
    });
  });

  group('SniffAccumulator', () {
    test('keeps 500 newest events when adding 501', () {
      final accumulator = SniffAccumulator();
      for (var i = 0; i < 501; i++) {
        accumulator.add(
          SniffEvent(
            url: 'https://x/$i',
            initiator: SniffInitiator.subResource,
          ),
        );
      }

      expect(accumulator.events, hasLength(500));
      expect(accumulator.events.first.url, 'https://x/1');
      expect(accumulator.events.last.url, 'https://x/500');
    });

    test('clears events on top-level navigation', () {
      final accumulator = SniffAccumulator();
      accumulator.add(
        const SniffEvent(
          url: 'https://x/a.m3u8',
          initiator: SniffInitiator.media,
        ),
      );

      accumulator.onTopLevelNavigation();

      expect(accumulator.events, isEmpty);
    });
  });
}
