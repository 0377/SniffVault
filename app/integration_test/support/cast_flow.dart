import 'dart:async';
import 'dart:convert';
import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:path_provider/path_provider.dart';
import 'package:video_sniffing/engine/engine_host.dart';
import 'package:video_sniffing/engine/models/cast_types.dart';
import 'package:video_sniffing/engine/models/library_episode.dart';
import 'package:video_sniffing/engine/models/task_event.dart';
import 'package:video_sniffing/engine/models/task_status.dart';

import 'playable_mp4.dart';
import 'test_pump.dart';

HttpServer? _fixtureServer;
String? _fixtureMp4Url;

Future<void> _startFixtureServer() async {
  if (_fixtureServer != null) {
    return;
  }
  final bytes = playableMp4Bytes();
  _fixtureServer = await HttpServer.bind(InternetAddress.loopbackIPv4, 0);
  _fixtureMp4Url = 'http://127.0.0.1:${_fixtureServer!.port}/clip.mp4';

  _fixtureServer!.listen((request) async {
    if (request.uri.path != '/clip.mp4') {
      request.response.statusCode = HttpStatus.notFound;
      await request.response.close();
      return;
    }

    request.response.headers.contentType = ContentType('video', 'mp4');
    request.response.contentLength = bytes.length;
    request.response.statusCode = HttpStatus.ok;
    request.response.add(bytes);
    await request.response.close();
  });
}

Future<void> _stopFixtureServer() async {
  await _fixtureServer?.close(force: true);
  _fixtureServer = null;
  _fixtureMp4Url = null;
}

void enableLan(EngineHost host) {
  host.saveSettings(host.settings().copyWith(lanEnabled: true));
}

Future<LibraryEpisode> seedCachedEpisode(
  WidgetTester tester,
  EngineHost sender,
) async {
  await _startFixtureServer();
  final title = 'cast-seed-${DateTime.now().millisecondsSinceEpoch}';
  final taskId = sender.enqueueSingle(
    title: title,
    url: _fixtureMp4Url!,
  );
  expect(taskId, isNotEmpty);

  await waitForTaskCompleted(tester, sender, taskId: taskId);

  final items = sender.listLibrary().where((item) => item.title == title);
  expect(items, hasLength(1));
  final episodes = sender.listEpisodes(items.first.id);
  expect(episodes, hasLength(1));
  return episodes.first;
}

Future<void> waitForTaskCompleted(
  WidgetTester tester,
  EngineHost host, {
  required String taskId,
  Duration timeout = const Duration(seconds: 60),
}) async {
  final completer = Completer<void>();
  late final StreamSubscription<TaskEvent> subscription;
  subscription = host.taskEvents.listen((event) {
    if (event.task?.id != taskId) {
      return;
    }
    if (event.task!.status == TaskStatus.completed) {
      if (!completer.isCompleted) {
        completer.complete();
      }
      return;
    }
    if (event.task!.status == TaskStatus.failed) {
      if (!completer.isCompleted) {
        completer.completeError(
          'Download failed: ${event.task!.errorMessage ?? "unknown error"}',
        );
      }
    }
  });

  try {
    host.startDownloads();
  } on EngineException catch (e) {
    if (!e.error.message.toLowerCase().contains('downloads already running')) {
      rethrow;
    }
  }
  host.spawnDownloadWorker();

  final end = DateTime.now().add(timeout);
  while (!completer.isCompleted && DateTime.now().isBefore(end)) {
    final matches = host.listTasks().where((t) => t.id == taskId);
    if (matches.isNotEmpty) {
      final task = matches.first;
      if (task.status == TaskStatus.completed) {
        if (!completer.isCompleted) {
          completer.complete();
        }
        break;
      }
      if (task.status == TaskStatus.failed) {
        if (!completer.isCompleted) {
          completer.completeError(
            'Download failed: ${task.errorMessage ?? "unknown error"}',
          );
        }
        break;
      }
    }
    await pumpEngineEvents(tester);
  }

  await subscription.cancel();

  if (!completer.isCompleted) {
    throw TimeoutException('Timed out waiting for task $taskId', timeout);
  }
  await completer.future;
}

Future<CastEventIncomingPlay> waitForIncomingCast(
  WidgetTester tester,
  EngineHost receiver, {
  Duration timeout = const Duration(minutes: 2),
}) async {
  final completer = Completer<CastEventIncomingPlay>();
  late final StreamSubscription<CastEvent> subscription;
  subscription = receiver.castEvents.listen((event) {
    if (event is CastEventIncomingPlay && !completer.isCompleted) {
      completer.complete(event);
    }
  });

  final end = DateTime.now().add(timeout);
  while (!completer.isCompleted && DateTime.now().isBefore(end)) {
    await pumpEngineEvents(tester);
  }

  await subscription.cancel();

  if (!completer.isCompleted) {
    throw TimeoutException('Timed out waiting for cast event', timeout);
  }
  return completer.future;
}

/// U10：双 EngineHost loopback 配对投送，断言元数据不含 source_url。
Future<void> runPairCastMetadataFlow(WidgetTester tester) async {
  final base = await getTemporaryDirectory();
  final ts = DateTime.now().millisecondsSinceEpoch;
  final receiverDir = '${base.path}/cast_rx_$ts';
  final senderDir = '${base.path}/cast_tx_$ts';

  final receiver = await EngineHost.open(receiverDir);
  final sender = await EngineHost.open(senderDir);
  try {
    final episode = await seedCachedEpisode(tester, sender);

    enableLan(receiver);
    enableLan(sender);

    receiver.applyLanSettings(isReceiver: true);
    final pin = receiver.beginPairing();
    final receiverPort = receiver.lanHttpPort;
    expect(receiverPort, isNotNull);

    sender.applyLanSettings(isReceiver: false);
    sender.pairPeer(host: '127.0.0.1', port: receiverPort!, pin: pin);

    final receiverDeviceId = receiver.settings().deviceId;

    final castFuture = waitForIncomingCast(tester, receiver);
    sender.castEpisode(
      episodeId: episode.id,
      peerDeviceId: receiverDeviceId,
    );
    final event = await castFuture;

    final json = jsonEncode(event.request.metadata.toJson());
    expect(json.contains('source_url'), isFalse);
  } finally {
    sender.stopCast();
    sender.stopLan();
    receiver.stopLan();
    sender.dispose();
    receiver.dispose();
    await _stopFixtureServer();
  }
}

Future<void> runDoubleCastReplacementFlow(WidgetTester tester) async {
  final base = await getTemporaryDirectory();
  final ts = DateTime.now().millisecondsSinceEpoch;
  final receiverDir = '${base.path}/cast_rx2_$ts';
  final senderDir = '${base.path}/cast_tx2_$ts';

  final receiver = await EngineHost.open(receiverDir);
  final sender = await EngineHost.open(senderDir);
  try {
    final ep1 = await seedCachedEpisode(tester, sender);
    final ep2 = await seedCachedEpisode(tester, sender);

    enableLan(receiver);
    enableLan(sender);
    receiver.applyLanSettings(isReceiver: true);
    final pin = receiver.beginPairing();
    final receiverPort = receiver.lanHttpPort!;
    sender.applyLanSettings(isReceiver: false);
    sender.pairPeer(host: '127.0.0.1', port: receiverPort, pin: pin);

    final rxId = receiver.settings().deviceId;

    final firstFuture = waitForIncomingCast(tester, receiver);
    sender.castEpisode(episodeId: ep1.id, peerDeviceId: rxId);
    final first = await firstFuture;
    expect(first.request.sessionId, isNotEmpty);

    final secondFuture = waitForIncomingCast(tester, receiver);
    sender.castEpisode(episodeId: ep2.id, peerDeviceId: rxId);
    final second = await secondFuture;

    expect(second.request.sessionId, isNot(first.request.sessionId));
    expect(second.request.metadata.episodeTitle, ep2.title);
  } finally {
    sender.stopCast();
    sender.stopLan();
    receiver.stopLan();
    sender.dispose();
    receiver.dispose();
    await _stopFixtureServer();
  }
}
