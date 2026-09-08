import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:video_sniffing/engine/models/download_task.dart';
import 'package:video_sniffing/engine/models/task_status.dart';
import 'package:video_sniffing/features/browse/cookie_store.dart';
import 'package:video_sniffing/features/browse/hook_to_sniff.dart';
import 'package:video_sniffing/features/settings/settings_screen.dart';
import 'package:video_sniffing/providers/browse_session.dart';
import 'package:video_sniffing/providers/engine_host_provider.dart';
import 'package:video_sniffing/providers/settings_provider.dart';

import 'fakes/fake_engine_repository.dart';

class FakeBrowseCookieStore implements BrowseCookieStore {
  var clearAllCalls = 0;

  @override
  Future<void> clearAll() async {
    clearAllCalls += 1;
  }
}

DownloadTask _task() {
  return const DownloadTask(
    id: 't1',
    title: 'clip',
    sourceUrl: 'https://x/v.mp4',
    status: TaskStatus.queued,
    progressBytes: 0,
    createdAtMs: 1,
    updatedAtMs: 1,
  );
}

void main() {
  testWidgets('settings tap clears browse cookies and shows snackbar', (
    tester,
  ) async {
    final fake = FakeEngineRepository(tasks: [_task()]);
    final cookies = FakeBrowseCookieStore();
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          engineRepositoryProvider.overrideWithValue(fake),
          settingsProvider.overrideWith((ref) => fake.settings()),
          browseCookieStoreProvider.overrideWithValue(cookies),
        ],
        child: const MaterialApp(home: SettingsScreen()),
      ),
    );
    await tester.pumpAndSettle();

    expect(find.widgetWithText(TextField, 'Cookie'), findsNothing);
    expect(find.text('Cookie'), findsNothing);

    await tester.tap(find.byKey(const Key('settings_clear_browse_cookies')));
    await tester.pump();

    expect(cookies.clearAllCalls, 1);
    expect(find.text('已清除浏览 Cookie'), findsOneWidget);
    expect(fake.tasks, hasLength(1));
    expect(fake.tasks.single.id, 't1');
  });

  test('hook payload does not include cookies', () {
    const request = HookRequest(
      url: 'https://site/page',
      pageUrl: 'https://site/page',
      isMainFrame: true,
    );
    final json = hookToSniffEvent(request).toJson();
    expect(json.containsKey('cookies'), isFalse);
    expect(json.containsKey('cookie'), isFalse);
  });

  test('plugin cookieHeaderFor returns null without native impl', () async {
    TestWidgetsFlutterBinding.ensureInitialized();
    const exporter = PluginCookieExporter();
    expect(
      await exporter.cookieHeaderFor(Uri.parse('https://example.com/')),
      isNull,
    );
    await exporter.clearAll();
  });
}
