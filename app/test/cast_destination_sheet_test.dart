import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:video_sniffing/cast_receiver/cast_providers.dart';
import 'package:video_sniffing/engine/engine_host.dart';
import 'package:video_sniffing/engine/models/cast_types.dart';
import 'package:video_sniffing/engine/models/ffi_response.dart';
import 'package:video_sniffing/features/cast/cast_destination_sheet.dart';
import 'package:video_sniffing/providers/engine_host_provider.dart';

import 'fakes/fake_engine_repository.dart';

void main() {
  test('presentCastError maps untrusted peer to user message', () {
    expect(
      presentCastError(
        EngineException(
          const FfiError(kind: 'invalid_arg', message: 'peer is not trusted'),
        ),
      ),
      '电视未信任此设备',
    );
  });

  test('presentCastError maps http 403 to untrusted message', () {
    expect(
      presentCastError(
        EngineException(
          const FfiError(
            kind: 'http',
            message: 'HTTP status client error (403 Forbidden)',
          ),
        ),
      ),
      '电视未信任此设备',
    );
  });

  test('presentCastError maps invalid signature to verification failure', () {
    expect(
      presentCastError(
        EngineException(
          const FfiError(kind: 'invalid_arg', message: 'invalid signature'),
        ),
      ),
      '投送验证失败',
    );
  });

  test('presentCastError maps http 401 to verification failure', () {
    expect(
      presentCastError(
        EngineException(
          const FfiError(
            kind: 'http',
            message: 'HTTP status client error (401 Unauthorized)',
          ),
        ),
      ),
      '投送验证失败',
    );
  });

  testWidgets('W14 CastDestinationSheet empty list message', (tester) async {
    final fake = FakeEngineRepository();
    fake.discoverPeerResults = const [];

    await tester.pumpWidget(_sheetScope(fake));
    await tester.pumpAndSettle();

    expect(
      find.text('未找到电视，请确认电视已开启局域网投送'),
      findsOneWidget,
    );
  });

  testWidgets('W15 CastDestinationSheet trusted items on top', (tester) async {
    final fake = FakeEngineRepository();
    fake.discoverPeerResults = const [
      LanPeer(
        deviceId: 'tv-new',
        deviceName: '新电视',
        host: '192.168.1.20',
        port: 8080,
        isTrusted: false,
      ),
      LanPeer(
        deviceId: 'tv-trusted',
        deviceName: '客厅电视',
        host: '192.168.1.10',
        port: 8080,
        isTrusted: true,
      ),
    ];

    await tester.pumpWidget(_sheetScope(fake));
    await tester.pumpAndSettle();

    expect(find.text('已配对'), findsOneWidget);
    final trustedTile = find.byKey(const Key('cast_peer_tv-trusted'));
    final newTile = find.byKey(const Key('cast_peer_tv-new'));
    expect(tester.getTopLeft(trustedTile).dy, lessThan(tester.getTopLeft(newTile).dy));
  });
}

Widget _sheetScope(FakeEngineRepository fake) {
  return ProviderScope(
    overrides: [
      engineRepositoryProvider.overrideWithValue(fake),
    ],
    child: const MaterialApp(
      home: Scaffold(
        body: CastDestinationSheet(episodeId: 'ep-1'),
      ),
    ),
  );
}
