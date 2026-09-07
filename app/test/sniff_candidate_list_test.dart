import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:video_sniffing/engine/models/resolve_types.dart';
import 'package:video_sniffing/features/browse/sniff_candidate_list.dart';

Widget _wrap(Widget child) {
  return MaterialApp(home: Scaffold(body: child));
}

void main() {
  testWidgets('empty list shrinks to nothing', (tester) async {
    await tester.pumpWidget(
      _wrap(SniffCandidateList(candidates: const [], onSelect: (_) {})),
    );

    expect(find.byType(ListTile), findsNothing);
    expect(tester.getSize(find.byType(SniffCandidateList)), Size.zero);
  });

  testWidgets('W7 tap candidate delivers that url to onSelect', (tester) async {
    const selectedUrl = 'https://cdn/a.m3u8';
    ResourceCandidate? selected;
    await tester.pumpWidget(
      _wrap(
        SniffCandidateList(
          candidates: const [
            ResourceCandidate(
              id: '1',
              url: selectedUrl,
              title: '1080p',
              kind: MediaKind.hls,
            ),
            ResourceCandidate(
              id: '2',
              url: 'https://cdn/b.mp4',
              kind: MediaKind.mp4,
            ),
          ],
          onSelect: (candidate) => selected = candidate,
        ),
      ),
    );

    expect(find.textContaining('2'), findsWidgets);
    expect(find.text('1080p'), findsOneWidget);
    expect(find.text('https://cdn/b.mp4'), findsOneWidget);

    await tester.tap(find.text('1080p'));
    expect(selected?.url, selectedUrl);
  });
}
