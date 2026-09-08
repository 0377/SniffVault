import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:video_sniffing/engine/engine_host.dart';
import 'package:video_sniffing/engine/models/cast_types.dart';
import 'package:video_sniffing/providers/engine_host_provider.dart';
import 'package:video_sniffing/ui/error_presenter.dart';

final activeCastRequestProvider = StateProvider<CastPlayRequest?>((ref) => null);

const emptyPeersMessage = '未找到电视，请确认电视已开启局域网投送';

/// 已信任设备置顶，其余保持发现顺序。
List<LanPeer> sortPeersForDisplay(List<LanPeer> peers) {
  final sorted = List<LanPeer>.from(peers);
  sorted.sort((a, b) {
    if (a.isTrusted == b.isTrusted) {
      return 0;
    }
    return a.isTrusted ? -1 : 1;
  });
  return sorted;
}

final discoveredPeersProvider = FutureProvider.autoDispose<List<LanPeer>>((ref) async {
  final peers = ref.read(engineRepositoryProvider).discoverPeers();
  return sortPeersForDisplay(peers);
});

String? presentCastError(EngineException exception) {
  final error = exception.error;
  final lower = error.message.toLowerCase();

  if (lower.contains('pairing pin expired')) {
    return '配对码已过期，请在电视上刷新';
  }
  if (lower.contains('pairing pin incorrect')) {
    return '配对码错误';
  }
  if (lower.contains('peer is not trusted') ||
      (error.kind == 'http' && lower.contains('403'))) {
    return '电视未信任此设备';
  }
  if (lower.contains('invalid signature') ||
      (error.kind == 'http' && lower.contains('401'))) {
    return '投送验证失败';
  }

  return presentEngineError(exception);
}
