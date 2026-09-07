import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:video_sniffing/engine/models/resolve_types.dart';
import 'package:video_sniffing/features/add/resolve_wizard.dart';

Widget _wrap(ResolveOutcome outcome) {
  return MaterialApp(
    home: Scaffold(
      body: ResolveWizard(
        outcome: outcome,
        onEnqueue: (_) async {},
      ),
    ),
  );
}

void main() {
  testWidgets('W1 Single shows download button', (tester) async {
    await tester.pumpWidget(
      _wrap(
        ResolveOutcomeSingle(
          ResourceCandidate(id: '1', url: 'https://x/y.mp4', kind: MediaKind.mp4),
        ),
      ),
    );
    expect(find.text('下载'), findsOneWidget);
  });

  testWidgets('W1 Candidates shows quality picker', (tester) async {
    await tester.pumpWidget(
      _wrap(
        ResolveOutcomeCandidates([
          ResourceCandidate(id: '1', url: 'https://x/a.m3u8', kind: MediaKind.hls),
        ]),
      ),
    );
    expect(find.textContaining('清晰度'), findsOneWidget);
  });

  testWidgets('W1 EpisodeList shows multi select', (tester) async {
    await tester.pumpWidget(
      _wrap(
        ResolveOutcomeEpisodeList(
          EpisodeList(
            title: 'Series',
            episodes: [
              Episode(index: 1, title: 'E1', url: 'https://x/1', qualityOptions: []),
            ],
          ),
        ),
      ),
    );
    expect(find.textContaining('选择分集'), findsOneWidget);
  });

  testWidgets('W1 NeedsBrowser shows browser hint', (tester) async {
    await tester.pumpWidget(
      _wrap(const ResolveOutcomeNeedsBrowser(reason: 'auth_required')),
    );
    expect(find.textContaining('需要在内置浏览中打开'), findsOneWidget);
    expect(find.text('打开内置浏览'), findsNothing);
    expect(find.text('返回'), findsOneWidget);
  });

  testWidgets('W5 open browse button when callback set', (tester) async {
    var opened = false;
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: ResolveWizard(
            outcome: const ResolveOutcomeNeedsBrowser(reason: 'auth_required'),
            onEnqueue: (_) async {},
            onOpenBrowser: () => opened = true,
          ),
        ),
      ),
    );
    await tester.tap(find.text('打开内置浏览'));
    expect(opened, isTrue);
  });
}
