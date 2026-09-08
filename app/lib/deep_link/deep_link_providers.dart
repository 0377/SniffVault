import 'package:app_links/app_links.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

final appLinksProvider = Provider<AppLinks>((ref) => AppLinks());

final ingressUriStreamProvider = Provider<Stream<Uri>>((ref) {
  return ref.watch(appLinksProvider).uriLinkStream;
});

Uri? _bootstrapIngressUri;

/// 在 runApp 之前调用，写入冷启动 initial link。
void setBootstrapIngressUri(Uri? uri) => _bootstrapIngressUri = uri;

final pendingIngressUriProvider = StateProvider<Uri?>(
  (ref) => _bootstrapIngressUri,
);
